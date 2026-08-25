import { useCallback, useEffect, useRef, useState } from "react";

import {
  getShareState,
  onCoreEvent,
  regenerateShareSession,
  startSharing,
  stopSharing,
  toDropLANError,
  type DropLANError,
} from "@/lib/tauri";
import { EVENTS, type Notice, type ShareState } from "@/types";

export interface SharingController {
  state: ShareState | null;
  /** True until the first snapshot has arrived. */
  loading: boolean;
  /** Set while a start/stop/regenerate call is in flight. */
  busy: boolean;
  error: DropLANError | null;
  notice: Notice | null;
  start: () => Promise<void>;
  stop: () => Promise<void>;
  regenerate: () => Promise<void>;
  refresh: () => Promise<void>;
  dismissError: () => void;
  dismissNotice: () => void;
}

/**
 * Owns the single source of truth for what the app is doing.
 *
 * The core pushes a fresh `ShareState` on every event that changes it, so this
 * never polls; the only reads are the initial snapshot and explicit refreshes.
 */
export function useSharing(): SharingController {
  const [state, setState] = useState<ShareState | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<DropLANError | null>(null);
  const [notice, setNotice] = useState<Notice | null>(null);

  // Guards against a late response overwriting newer pushed state.
  const mounted = useRef(true);

  // Written with `.then` rather than `async`/`await` so the state updates
  // happen in a callback, which is what makes it safe to call from an effect.
  const refresh = useCallback(
    () =>
      getShareState()
        .then((next) => {
          if (mounted.current) setState(next);
        })
        .catch((cause: unknown) => {
          if (mounted.current) setError(toDropLANError(cause));
        })
        .finally(() => {
          if (mounted.current) setLoading(false);
        }),
    [],
  );

  useEffect(() => {
    mounted.current = true;
    void refresh();

    const unlisteners: Promise<() => void>[] = [
      // Rust sends the whole ShareState with these, so there is nothing to
      // merge and no chance of the UI drifting out of sync with the core.
      onCoreEvent<ShareState>(EVENTS.sharingStarted, (payload) => setState(payload)),
      onCoreEvent<ShareState>(EVENTS.networkChanged, (payload) => setState(payload)),
      onCoreEvent<null>(EVENTS.sharingStopped, () => void refresh()),
      onCoreEvent<unknown>(EVENTS.sharedFilesChanged, () => void refresh()),
      onCoreEvent<Notice>(EVENTS.notice, (payload) => setNotice(payload)),
    ];

    return () => {
      mounted.current = false;
      for (const pending of unlisteners) {
        void pending.then((unlisten) => unlisten());
      }
    };
  }, [refresh]);

  const run = useCallback(async (action: () => Promise<ShareState>) => {
    setBusy(true);
    setError(null);
    try {
      const next = await action();
      if (mounted.current) setState(next);
    } catch (cause) {
      if (mounted.current) setError(toDropLANError(cause));
    } finally {
      if (mounted.current) setBusy(false);
    }
  }, []);

  return {
    state,
    loading,
    busy,
    error,
    notice,
    start: useCallback(() => run(startSharing), [run]),
    stop: useCallback(() => run(stopSharing), [run]),
    regenerate: useCallback(() => run(regenerateShareSession), [run]),
    refresh,
    dismissError: useCallback(() => setError(null), []),
    dismissNotice: useCallback(() => setNotice(null), []),
  };
}
