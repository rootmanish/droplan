import { openUrl } from "@tauri-apps/plugin-opener";
import { Info, X } from "lucide-react";

import { Button } from "@/components/ui/button";
import type { PlatformNotice as PlatformNoticeData } from "@/types";

interface PlatformNoticeProps {
  notice: PlatformNoticeData;
  onDismiss: () => void;
}

/**
 * Firewall and local-network permission guidance.
 *
 * DropLAN never edits firewall rules itself, so when the OS gets in the way
 * the only honest thing to do is explain why and point at the right settings.
 */
export function PlatformNotice({ notice, onDismiss }: PlatformNoticeProps) {
  return (
    <div className="flex gap-3 rounded-lg border border-border bg-muted/40 p-3">
      <Info className="mt-0.5 size-4 shrink-0 text-muted-foreground" aria-hidden />
      <div className="flex min-w-0 flex-1 flex-col gap-2">
        <div>
          <p className="text-[13px] font-medium">{notice.title}</p>
          <p className="mt-0.5 text-xs leading-relaxed text-muted-foreground">{notice.body}</p>
        </div>
        {notice.actionUrl && notice.actionLabel && (
          <div>
            <Button
              variant="outline"
              size="sm"
              onClick={() => {
                void openUrl(notice.actionUrl ?? "").catch(() => undefined);
              }}
            >
              {notice.actionLabel}
            </Button>
          </div>
        )}
      </div>
      <Button variant="ghost" size="iconSm" onClick={onDismiss} className="-mr-1 -mt-1">
        <X className="size-3.5" aria-hidden />
        <span className="sr-only">Dismiss</span>
      </Button>
    </div>
  );
}
