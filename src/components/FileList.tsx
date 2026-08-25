import { useState } from "react";

import { FileItem } from "@/components/FileItem";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { formatSize } from "@/lib/format";
import type { RegistryTotals, ShareItem } from "@/types";

interface FileListProps {
  files: ShareItem[];
  totals: RegistryTotals;
  onRemove: (id: string) => void;
  onClear: () => void;
}

export function FileList({ files, totals, onRemove, onClear }: FileListProps) {
  const [confirming, setConfirming] = useState(false);

  if (files.length === 0) return null;

  return (
    <section className="flex min-h-0 flex-1 flex-col gap-2">
      <div className="flex items-center justify-between px-1">
        <h2 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
          Shared files
        </h2>
        <Button
          variant="ghost"
          size="sm"
          className="h-7 px-2 text-xs text-muted-foreground hover:text-foreground"
          onClick={() => setConfirming(true)}
        >
          Clear all
        </Button>
      </div>

      <ul className="-mx-1 min-h-0 flex-1 overflow-y-auto">
        {files.map((item) => (
          <FileItem key={item.id} item={item} onRemove={onRemove} />
        ))}
      </ul>

      <p className="px-1 text-xs text-muted-foreground">
        {totals.fileCount} {totals.fileCount === 1 ? "file" : "files"} ·{" "}
        {formatSize(totals.totalBytes)} shared
        {totals.unavailableCount > 0 && ` · ${totals.unavailableCount} unavailable`}
      </p>

      <Dialog open={confirming} onOpenChange={setConfirming}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Stop sharing all files?</DialogTitle>
            <DialogDescription>
              This empties the list and makes every download link stop working. Your files are
              not deleted — they stay exactly where they are on disk.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="ghost" onClick={() => setConfirming(false)}>
              Cancel
            </Button>
            <Button
              variant="destructive"
              onClick={() => {
                onClear();
                setConfirming(false);
              }}
            >
              Clear all
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </section>
  );
}
