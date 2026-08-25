import { useCallback, useEffect, useState } from "react";
import { AlertCircle, Loader2, Settings as SettingsIcon, X } from "lucide-react";

import { BrandMark } from "@/components/BrandMark";
import { DeviceList } from "@/components/DeviceList";
import { DropZone } from "@/components/DropZone";
import { FileList } from "@/components/FileList";
import { NetworkSelector } from "@/components/NetworkSelector";
import { PlatformNotice } from "@/components/PlatformNotice";
import { ShareAddress } from "@/components/ShareAddress";
import { SharingToggle } from "@/components/SharingToggle";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { TransferList } from "@/components/TransferList";
import { useFiles } from "@/hooks/useFiles";
import { useNetwork } from "@/hooks/useNetwork";
import { useSharing } from "@/hooks/useSharing";
import { useTransfers } from "@/hooks/useTransfers";
import { formatSize } from "@/lib/format";
import { SettingsDialog } from "@/pages/Settings";

type ActivityTab = "transfers" | "devices";

/** Errors worth explaining with the platform notice rather than a bare toast. */
const PERMISSION_RELATED = new Set(["server_start", "no_private_network"]);

export function Home() {
  const sharing = useSharing();
  const { refresh } = sharing;

  // The hooks below only need "something changed, re-read the state", so the
  // promise is intentionally discarded here rather than at every call site.
  const reload = useCallback(() => {
    void refresh();
  }, [refresh]);

  const files = useFiles(reload);
  const transfers = useTransfers(sharing.state?.sharing ?? false);
  const network = useNetwork(
    sharing.state?.network,
    sharing.state?.settings.preferredInterface,
    reload,
  );

  const [settingsOpen, setSettingsOpen] = useState(false);
  const [noticeDismissed, setNoticeDismissed] = useState(false);
  const [tab, setTab] = useState<ActivityTab>("transfers");

  // A file can be moved or deleted while the window is in the background.
  useEffect(() => {
    window.addEventListener("focus", reload);
    return () => window.removeEventListener("focus", reload);
  }, [reload]);

  const toggleSharing = useCallback(
    (next: boolean) => void (next ? sharing.start() : sharing.stop()),
    [sharing],
  );

  if (sharing.loading || !sharing.state) {
    return (
      <div className="flex h-full items-center justify-center">
        <Loader2 className="size-5 animate-spin text-muted-foreground" aria-hidden />
      </div>
    );
  }

  const state = sharing.state;
  const hasFiles = state.files.length > 0;
  // Firewall guidance is only useful while nothing has reached us yet. Once a
  // device has connected, the permission clearly went through, so the card
  // stops taking up space instead of nagging on every launch.
  const nothingHasConnected = transfers.clients.length === 0;
  const showPlatformNotice =
    !noticeDismissed &&
    ((state.sharing && nothingHasConnected) || PERMISSION_RELATED.has(sharing.error?.code ?? ""));

  return (
    <div className="flex h-full flex-col overflow-hidden">
      <header
        className="flex shrink-0 items-center justify-between gap-3 border-b border-border px-4 py-3"
        data-tauri-drag-region
      >
        <div className="flex min-w-0 items-center gap-2">
          <BrandMark className="size-[18px]" />
          <h1 className="text-[15px] font-semibold tracking-tight">
            Drop<span className="text-[#2F7FD4] dark:text-[#5CC8F5]">LAN</span>
          </h1>
          <span className="truncate text-xs text-muted-foreground">{state.deviceName}</span>
        </div>
        <div className="flex items-center gap-2">
          <SharingToggle
            sharing={state.sharing}
            busy={sharing.busy}
            disabled={network.usable.length === 0 && !state.sharing}
            onChange={toggleSharing}
          />
          <Button
            variant="ghost"
            size="icon"
            onClick={() => setSettingsOpen(true)}
            aria-label="Settings"
          >
            <SettingsIcon className="size-4" aria-hidden />
          </Button>
        </div>
      </header>

      <main className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto p-4">
        {sharing.error && (
          <div className="flex items-start gap-2.5 rounded-lg border border-destructive/40 bg-destructive/10 p-3">
            <AlertCircle className="mt-0.5 size-4 shrink-0 text-destructive" aria-hidden />
            <p className="min-w-0 flex-1 text-[13px] leading-relaxed">{sharing.error.message}</p>
            <Button variant="ghost" size="iconSm" onClick={sharing.dismissError}>
              <X className="size-3.5" aria-hidden />
              <span className="sr-only">Dismiss</span>
            </Button>
          </div>
        )}

        {sharing.notice && (
          <div className="flex items-start gap-2.5 rounded-lg border border-border bg-muted/50 p-3">
            <p className="min-w-0 flex-1 text-[13px] leading-relaxed text-muted-foreground">
              {sharing.notice.message}
            </p>
            <Button variant="ghost" size="iconSm" onClick={sharing.dismissNotice}>
              <X className="size-3.5" aria-hidden />
              <span className="sr-only">Dismiss</span>
            </Button>
          </div>
        )}

        <Card>
          <CardContent className="flex flex-col gap-3 p-4">
            <NetworkSelector
              interfaces={network.interfaces}
              selected={network.selected}
              pinned={network.pinned}
              refreshing={network.refreshing}
              onSelect={(name) => void network.select(name)}
              onRefresh={() => void network.refresh()}
            />
            {network.error && <p className="text-xs text-destructive">{network.error}</p>}

            <ShareAddress
              shareUrl={state.shareUrl}
              friendlyUrl={state.friendlyUrl}
              pin={state.session?.pin ?? null}
              busy={sharing.busy}
              onRegenerate={() => void sharing.regenerate()}
            />
          </CardContent>
        </Card>

        {showPlatformNotice && (
          <PlatformNotice
            notice={state.platformNotice}
            onDismiss={() => setNoticeDismissed(true)}
          />
        )}

        <DropZone
          dragActive={files.dragActive}
          busy={files.busy}
          compact={hasFiles}
          onChooseFiles={() => void files.chooseFiles()}
          onChooseFolder={() => void files.chooseFolder()}
        />

        {files.message && (
          <p className="px-1 text-xs text-muted-foreground">
            {files.message}{" "}
            <button
              type="button"
              className="underline underline-offset-2"
              onClick={files.dismissMessage}
            >
              Dismiss
            </button>
          </p>
        )}

        <FileList
          files={state.files}
          totals={state.totals}
          onRemove={(id) => void files.remove(id)}
          onClear={() => void files.clear()}
        />

        {state.sharing && (
          <Card className="shrink-0">
            <div className="flex items-center gap-1 border-b border-border px-2 py-1.5">
              {(["transfers", "devices"] as const).map((name) => (
                <Button
                  key={name}
                  variant={tab === name ? "secondary" : "ghost"}
                  size="sm"
                  className="h-7 px-2.5 text-xs capitalize"
                  onClick={() => setTab(name)}
                >
                  {name}
                  {name === "transfers" && transfers.active.length > 0 && (
                    <span className="ml-1 rounded-full bg-live px-1.5 text-[10px] text-live-foreground">
                      {transfers.active.length}
                    </span>
                  )}
                  {name === "devices" && transfers.clients.length > 0 && (
                    <span className="ml-1 text-muted-foreground">{transfers.clients.length}</span>
                  )}
                </Button>
              ))}
              {transfers.totalBytesServed > 0 && (
                <span className="ml-auto pr-2 text-[11px] text-muted-foreground">
                  {formatSize(transfers.totalBytesServed)} served
                </span>
              )}
            </div>
            <div className="py-1">
              {tab === "transfers" ? (
                <TransferList active={transfers.active} recent={transfers.recent} />
              ) : (
                <DeviceList clients={transfers.clients} />
              )}
            </div>
          </Card>
        )}
      </main>

      <footer className="shrink-0 border-t border-border px-4 py-2">
        <p className="text-[11px] text-muted-foreground">
          {state.sharing
            ? "Files are served directly over your local network. Nothing is uploaded anywhere."
            : "Sharing is off. No device on your network can reach this computer."}
        </p>
      </footer>

      <SettingsDialog
        open={settingsOpen}
        onOpenChange={setSettingsOpen}
        settings={state.settings}
        onSaved={reload}
      />
    </div>
  );
}
