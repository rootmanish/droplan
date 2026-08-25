import { useCallback, useState } from "react";

import { refreshNetwork, setPreferredInterface, toDropLANError } from "@/lib/tauri";
import type { NetworkInterface, NetworkSnapshot } from "@/types";

export interface NetworkController {
  interfaces: NetworkInterface[];
  usable: NetworkInterface[];
  selected: NetworkInterface | null;
  /** True while an interface is pinned rather than chosen automatically. */
  pinned: boolean;
  refreshing: boolean;
  refresh: () => Promise<void>;
  select: (name: string | null) => Promise<void>;
  error: string | null;
}

/**
 * Interface listing and selection.
 *
 * The snapshot itself arrives with the share state; this hook only owns the
 * actions and the transient "refreshing" flag.
 */
export function useNetwork(
  snapshot: NetworkSnapshot | undefined,
  preferredInterface: string | null | undefined,
  onChanged: () => void,
): NetworkController {
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const interfaces = snapshot?.interfaces ?? [];

  const refresh = useCallback(async () => {
    setRefreshing(true);
    setError(null);
    try {
      await refreshNetwork();
      onChanged();
    } catch (cause) {
      setError(toDropLANError(cause).message);
    } finally {
      setRefreshing(false);
    }
  }, [onChanged]);

  const select = useCallback(
    async (name: string | null) => {
      setError(null);
      try {
        await setPreferredInterface(name);
        onChanged();
      } catch (cause) {
        setError(toDropLANError(cause).message);
      }
    },
    [onChanged],
  );

  return {
    interfaces,
    usable: interfaces.filter((entry) => entry.usable),
    selected: snapshot?.selected ?? null,
    pinned: Boolean(preferredInterface),
    refreshing,
    refresh,
    select,
    error,
  };
}
