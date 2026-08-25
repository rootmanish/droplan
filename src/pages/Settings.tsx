import { useState } from "react";
import { Loader2 } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import { Switch } from "@/components/ui/switch";
import { updateSettings } from "@/lib/tauri";
import { toDropLANError } from "@/lib/tauri";
import type { AppSettings } from "@/types";

const MIN_PORT = 1024;
const MAX_PORT = 65535;

interface SettingRowProps {
  id: string;
  title: string;
  description: string;
  children: React.ReactNode;
}

function SettingRow({ id, title, description, children }: SettingRowProps) {
  return (
    <div className="flex items-start justify-between gap-4 py-3">
      <div className="min-w-0">
        <Label htmlFor={id} className="text-[13px]">
          {title}
        </Label>
        <p className="mt-0.5 text-xs leading-relaxed text-muted-foreground">{description}</p>
      </div>
      <div className="shrink-0 pt-0.5">{children}</div>
    </div>
  );
}

interface SettingsFormProps {
  onOpenChange: (open: boolean) => void;
  settings: AppSettings;
  onSaved: () => void;
}

/**
 * The form body.
 *
 * Split out from the dialog shell so it unmounts when the dialog closes: the
 * draft is then seeded from `useState` on every open, with no effect needed
 * to keep it from going stale.
 */
function SettingsForm({ onOpenChange, settings, onSaved }: SettingsFormProps) {
  const [draft, setDraft] = useState<AppSettings>(settings);
  const [portText, setPortText] = useState(String(settings.preferredPort));
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const portValue = Number.parseInt(portText, 10);
  const portValid = Number.isInteger(portValue) && portValue >= MIN_PORT && portValue <= MAX_PORT;

  const save = async () => {
    if (!portValid) return;
    setSaving(true);
    setError(null);
    try {
      await updateSettings({ ...draft, preferredPort: portValue });
      onSaved();
      onOpenChange(false);
    } catch (cause) {
      setError(toDropLANError(cause).message);
    } finally {
      setSaving(false);
    }
  };

  return (
    <>
      <DialogHeader>
        <DialogTitle>Settings</DialogTitle>
        <DialogDescription>
          Preferences are saved on this computer. Shared files are never remembered between
          launches.
        </DialogDescription>
      </DialogHeader>

      <div className="overflow-y-auto px-4">
        <SettingRow
          id="preferred-port"
          title="Preferred port"
          description="DropLAN moves to the next free port if this one is taken."
        >
          <Input
            id="preferred-port"
            value={portText}
            inputMode="numeric"
            onChange={(event) => setPortText(event.target.value.replace(/[^0-9]/g, ""))}
            className="h-8 w-24 text-center font-mono"
            aria-invalid={!portValid}
          />
        </SettingRow>
        {!portValid && (
          <p className="pb-2 text-xs text-destructive">
            Choose a port between {MIN_PORT} and {MAX_PORT}.
          </p>
        )}

        <Separator />

        <SettingRow
          id="start-on-launch"
          title="Start sharing when DropLAN opens"
          description="A new, unguessable link is created every launch either way."
        >
          <Switch
            id="start-on-launch"
            checked={draft.startSharingOnLaunch}
            onCheckedChange={(checked) =>
              setDraft((current) => ({ ...current, startSharingOnLaunch: checked }))
            }
          />
        </SettingRow>

        <Separator />

        <SettingRow
          id="enable-mdns"
          title="Publish a .local name"
          description="Lets other devices reach this computer by name as well as by IP address."
        >
          <Switch
            id="enable-mdns"
            checked={draft.enableMdns}
            onCheckedChange={(checked) =>
              setDraft((current) => ({ ...current, enableMdns: checked }))
            }
          />
        </SettingRow>

        <Separator />

        <SettingRow
          id="require-pin"
          title="Require a PIN"
          description="Adds a 6-digit code in front of the share page. Changing this starts a new session."
        >
          <Switch
            id="require-pin"
            checked={draft.requirePin}
            onCheckedChange={(checked) =>
              setDraft((current) => ({ ...current, requirePin: checked }))
            }
          />
        </SettingRow>

        <Separator />

        <SettingRow
          id="close-to-tray"
          title="Keep sharing when the window is closed"
          description="Off by default. When on, the tray icon stays visible so a running server is never hidden."
        >
          <Switch
            id="close-to-tray"
            checked={draft.closeToTray}
            onCheckedChange={(checked) =>
              setDraft((current) => ({ ...current, closeToTray: checked }))
            }
          />
        </SettingRow>
      </div>

      {error && <p className="px-4 pb-2 text-xs text-destructive">{error}</p>}

      <DialogFooter>
        <Button variant="ghost" onClick={() => onOpenChange(false)} disabled={saving}>
          Cancel
        </Button>
        <Button onClick={() => void save()} disabled={saving || !portValid}>
          {saving && <Loader2 className="size-4 animate-spin" aria-hidden />}
          Save
        </Button>
      </DialogFooter>
    </>
  );
}

interface SettingsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  settings: AppSettings;
  onSaved: () => void;
}

export function SettingsDialog({ open, onOpenChange, settings, onSaved }: SettingsDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <SettingsForm onOpenChange={onOpenChange} settings={settings} onSaved={onSaved} />
      </DialogContent>
    </Dialog>
  );
}
