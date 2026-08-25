import { useCallback, useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";

import {
  addSharedFiles,
  clearSharedFiles,
  onFileDrop,
  removeSharedFile,
  toDropLANError,
} from "@/lib/tauri";
import type { AddOutcome } from "@/types";

export interface FilesController {
  /** True while a native drag is hovering the window. */
  dragActive: boolean;
  busy: boolean;
  /** Short feedback about the last add ("2 already shared"), or an error. */
  message: string | null;
  addPaths: (paths: string[]) => Promise<void>;
  chooseFiles: () => Promise<void>;
  chooseFolder: () => Promise<void>;
  remove: (id: string) => Promise<void>;
  clear: () => Promise<void>;
  dismissMessage: () => void;
}

/** Human summary of a partially successful add. */
function describe(outcome: AddOutcome): string | null {
  const parts: string[] = [];
  if (outcome.skippedDuplicates > 0) {
    parts.push(
      `${outcome.skippedDuplicates} ${outcome.skippedDuplicates === 1 ? "file was" : "files were"} already shared`,
    );
  }
  if (outcome.skippedUnreadable > 0) {
    parts.push(`${outcome.skippedUnreadable} could not be read`);
  }
  if (outcome.truncated) {
    parts.push("the folder was too large to add completely");
  }
  return parts.length > 0 ? `${parts.join(", ")}.` : null;
}

/**
 * File actions plus native drag and drop.
 *
 * Paths never round-trip through the browser `File` API, so adding a 50 GB
 * video costs a registry entry rather than a copy.
 */
export function useFiles(onChanged: () => void): FilesController {
  const [dragActive, setDragActive] = useState(false);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  const addPaths = useCallback(
    async (paths: string[]) => {
      if (paths.length === 0) return;
      setBusy(true);
      setMessage(null);
      try {
        setMessage(describe(await addSharedFiles(paths)));
      } catch (cause) {
        setMessage(toDropLANError(cause).message);
      } finally {
        setBusy(false);
        onChanged();
      }
    },
    [onChanged],
  );

  useEffect(() => {
    const pending = onFileDrop({
      onEnter: () => setDragActive(true),
      onLeave: () => setDragActive(false),
      onDrop: (paths) => {
        setDragActive(false);
        void addPaths(paths);
      },
    });
    return () => {
      void pending.then((unlisten) => unlisten());
    };
  }, [addPaths]);

  const chooseFiles = useCallback(async () => {
    const picked = await open({ multiple: true, directory: false, title: "Share files" });
    if (!picked) return;
    await addPaths(Array.isArray(picked) ? picked : [picked]);
  }, [addPaths]);

  const chooseFolder = useCallback(async () => {
    const picked = await open({ multiple: false, directory: true, title: "Share a folder" });
    if (!picked || Array.isArray(picked)) return;
    await addPaths([picked]);
  }, [addPaths]);

  const remove = useCallback(
    async (id: string) => {
      try {
        await removeSharedFile(id);
      } catch (cause) {
        setMessage(toDropLANError(cause).message);
      } finally {
        onChanged();
      }
    },
    [onChanged],
  );

  const clear = useCallback(async () => {
    try {
      await clearSharedFiles();
    } catch (cause) {
      setMessage(toDropLANError(cause).message);
    } finally {
      onChanged();
    }
  }, [onChanged]);

  return {
    dragActive,
    busy,
    message,
    addPaths,
    chooseFiles,
    chooseFolder,
    remove,
    clear,
    dismissMessage: useCallback(() => setMessage(null), []),
  };
}
