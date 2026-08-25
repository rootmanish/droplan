import { AlertTriangle, X } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Tooltip } from "@/components/ui/tooltip";
import { fileKind, formatSize } from "@/lib/format";
import { cn } from "@/lib/utils";
import type { ShareItem } from "@/types";

interface FileItemProps {
  item: ShareItem;
  onRemove: (id: string) => void;
}

export function FileItem({ item, onRemove }: FileItemProps) {
  return (
    <li
      className={cn(
        "group flex items-center gap-3 rounded-lg px-3 py-2 transition-colors hover:bg-accent/50",
        !item.available && "opacity-60",
      )}
    >
      <div className="min-w-0 flex-1">
        <p className="truncate text-[13px] font-medium" title={item.displayName}>
          {item.displayName}
        </p>
        <div className="mt-0.5 flex items-center gap-2 text-xs text-muted-foreground">
          <span>{formatSize(item.size)}</span>
          <Badge variant="outline" className="px-1.5 py-0 text-[10px] uppercase">
            {fileKind(item.mimeType, item.displayName)}
          </Badge>
          {!item.available && (
            <span className="flex items-center gap-1 text-destructive">
              <AlertTriangle className="size-3" aria-hidden />
              File unavailable
            </span>
          )}
        </div>
      </div>

      <Tooltip label="Stop sharing this file">
        <Button
          variant="ghost"
          size="iconSm"
          className="opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100"
          onClick={() => onRemove(item.id)}
        >
          <X className="size-3.5" aria-hidden />
          <span className="sr-only">Remove {item.displayName}</span>
        </Button>
      </Tooltip>
    </li>
  );
}
