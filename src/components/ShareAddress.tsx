import { useState } from "react";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { Check, Copy, KeyRound, Link2, QrCode as QrCodeIcon, RotateCw } from "lucide-react";

import { QrCode } from "@/components/QrCode";
import { Button } from "@/components/ui/button";
import { Tooltip } from "@/components/ui/tooltip";
import { splitShareUrl } from "@/lib/format";
import { cn } from "@/lib/utils";

interface ShareAddressProps {
  shareUrl: string | null;
  friendlyUrl: string | null;
  pin: string | null;
  busy: boolean;
  onRegenerate: () => void;
}

export function ShareAddress({
  shareUrl,
  friendlyUrl,
  pin,
  busy,
  onRegenerate,
}: ShareAddressProps) {
  const [copied, setCopied] = useState(false);
  const [showQr, setShowQr] = useState(false);

  if (!shareUrl) {
    return (
      <div className="rounded-lg border border-dashed border-border px-3 py-4 text-center text-[13px] text-muted-foreground">
        Turn sharing on to get an address for this device.
      </div>
    );
  }

  const { origin, path } = splitShareUrl(shareUrl);

  const copy = async () => {
    try {
      await writeText(shareUrl);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1600);
    } catch {
      setCopied(false);
    }
  };

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center gap-2 rounded-lg border border-border bg-background px-3 py-2">
        <Link2 className="size-4 shrink-0 text-muted-foreground" aria-hidden />
        <p className="selectable min-w-0 flex-1 truncate font-mono text-[13px] leading-5">
          <span>{origin}</span>
          <span className="text-muted-foreground">{path}</span>
        </p>

        <Tooltip label={copied ? "Copied" : "Copy URL"}>
          <Button variant="ghost" size="iconSm" onClick={() => void copy()}>
            {copied ? (
              <Check className="size-3.5 text-online" aria-hidden />
            ) : (
              <Copy className="size-3.5" aria-hidden />
            )}
            <span className="sr-only">Copy URL</span>
          </Button>
        </Tooltip>

        <Tooltip label={showQr ? "Hide QR code" : "Show QR code"}>
          <Button
            variant="ghost"
            size="iconSm"
            onClick={() => setShowQr((current) => !current)}
            className={cn(showQr && "bg-accent text-accent-foreground")}
          >
            <QrCodeIcon className="size-3.5" aria-hidden />
            <span className="sr-only">Show QR code</span>
          </Button>
        </Tooltip>

        <Tooltip label="New link (invalidates the current one)">
          <Button variant="ghost" size="iconSm" onClick={onRegenerate} disabled={busy}>
            <RotateCw className={cn("size-3.5", busy && "animate-spin")} aria-hidden />
            <span className="sr-only">Generate a new link</span>
          </Button>
        </Tooltip>
      </div>

      {pin && (
        <div className="flex items-center gap-2 rounded-lg border border-border bg-muted/40 px-3 py-2">
          <KeyRound className="size-4 shrink-0 text-muted-foreground" aria-hidden />
          <span className="text-[13px] text-muted-foreground">PIN</span>
          <span className="selectable font-mono text-base font-semibold tracking-[0.3em]">
            {pin}
          </span>
        </div>
      )}

      {friendlyUrl && (
        <p className="px-1 text-xs text-muted-foreground">
          Also reachable at <span className="selectable font-mono">{friendlyUrl}</span>
        </p>
      )}

      {showQr && (
        <div className="flex justify-center py-2">
          <QrCode url={shareUrl} />
        </div>
      )}
    </div>
  );
}
