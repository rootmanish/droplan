import { useCallback, useEffect, useState } from "react";

import { getTransferActivity, onCoreEvent } from "@/lib/tauri";
import {
  EVENTS,
  type ActivitySnapshot,
  type ClientSnapshot,
  type TransferSnapshot,
} from "@/types";

const EMPTY: ActivitySnapshot = {
  active: [],
  recent: [],
  clients: [],
  totalBytesServed: 0,
};

/** Newest first, capped to match the core's own retention. */
const MAX_RECENT = 25;

/**
 * Live download activity.
 *
 * Driven entirely by events: `transfer-progress` is already throttled in Rust,
 * so applying each one directly is cheap and there is nothing to poll.
 */
export function useTransfers(sharing: boolean): ActivitySnapshot & { refresh: () => void } {
  const [activity, setActivity] = useState<ActivitySnapshot>(EMPTY);

  const refresh = useCallback(() => {
    void getTransferActivity()
      .then(setActivity)
      .catch(() => setActivity(EMPTY));
  }, []);

  useEffect(() => {
    // Nothing to track while stopped. The hook returns EMPTY in that case
    // rather than clearing state here, which would be a second render pass.
    if (!sharing) return;

    refresh();

    const upsertActive = (transfer: TransferSnapshot) =>
      setActivity((current) => ({
        ...current,
        active: [
          transfer,
          ...current.active.filter((existing) => existing.id !== transfer.id),
        ],
      }));

    const finish = (transfer: TransferSnapshot) =>
      setActivity((current) => ({
        ...current,
        active: current.active.filter((existing) => existing.id !== transfer.id),
        recent: [transfer, ...current.recent].slice(0, MAX_RECENT),
        totalBytesServed: current.totalBytesServed + transfer.transferredBytes,
      }));

    const unlisteners: Promise<() => void>[] = [
      onCoreEvent<TransferSnapshot>(EVENTS.transferStarted, upsertActive),
      onCoreEvent<TransferSnapshot>(EVENTS.transferProgress, upsertActive),
      onCoreEvent<TransferSnapshot>(EVENTS.transferCompleted, finish),
      onCoreEvent<TransferSnapshot>(EVENTS.transferFailed, finish),
      onCoreEvent<ClientSnapshot[]>(EVENTS.clientsChanged, (clients) =>
        setActivity((current) => ({ ...current, clients })),
      ),
    ];

    return () => {
      for (const pending of unlisteners) {
        void pending.then((unlisten) => unlisten());
      }
    };
  }, [sharing, refresh]);

  return { ...(sharing ? activity : EMPTY), refresh };
}
