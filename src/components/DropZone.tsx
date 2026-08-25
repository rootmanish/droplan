import { FolderOpen, Loader2, Upload } from "lucide-react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

interface DropZoneProps {
  dragActive: boolean;
  busy: boolean;
  compact: boolean;
  onChooseFiles: () => void;
  onChooseFolder: () => void;
}

/**
 * The drop target.
 *
 * Native drag and drop is handled by the webview (see `useFiles`), so this is
 * presentation plus the picker buttons. It shrinks once files are shared so
 * the list gets the space.
 */
export function DropZone({
  dragActive,
  busy,
  compact,
  onChooseFiles,
  onChooseFolder,
}: DropZoneProps) {
  return (
    <div
      // The whole area is a target, not just the buttons. The nested buttons
      // stop propagation so "Add folder" does not also open the file picker.
      role="button"
      tabIndex={0}
      aria-label="Add files to share"
      onClick={onChooseFiles}
      onKeyDown={(event) => {
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          onChooseFiles();
        }
      }}
      className={cn(
        "flex cursor-default flex-col items-center justify-center gap-3 rounded-xl border-2 border-dashed px-4 text-center transition-colors",
        compact ? "py-5" : "py-10",
        dragActive
          ? "border-live bg-live/10"
          : "border-border bg-card/40 hover:border-muted-foreground/40",
      )}
    >
      {busy ? (
        <Loader2 className="size-5 animate-spin text-muted-foreground" aria-hidden />
      ) : (
        <Upload
          className={cn(
            "size-5 transition-colors",
            dragActive ? "text-live" : "text-muted-foreground",
          )}
          aria-hidden
        />
      )}

      <div className="flex flex-col gap-1">
        <p className="text-sm font-medium">{dragActive ? "Release to share" : "Drop files here"}</p>
        {!compact && (
          <p className="text-xs text-muted-foreground">
            or click to choose — documents, images, videos, archives, folders
          </p>
        )}
      </div>

      <div className="flex gap-2">
        <Button
          variant="secondary"
          size="sm"
          disabled={busy}
          onClick={(event) => {
            event.stopPropagation();
            onChooseFiles();
          }}
        >
          Add files
        </Button>
        <Button
          variant="ghost"
          size="sm"
          disabled={busy}
          onClick={(event) => {
            event.stopPropagation();
            onChooseFolder();
          }}
        >
          <FolderOpen className="size-4" aria-hidden />
          Add folder
        </Button>
      </div>
    </div>
  );
}
