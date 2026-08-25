import { Loader2 } from "lucide-react";

import { StatusBadge } from "@/components/StatusBadge";
import { Switch } from "@/components/ui/switch";

interface SharingToggleProps {
  sharing: boolean;
  busy: boolean;
  disabled?: boolean;
  onChange: (next: boolean) => void;
}

export function SharingToggle({ sharing, busy, disabled, onChange }: SharingToggleProps) {
  return (
    <div className="flex items-center gap-3">
      <StatusBadge sharing={sharing} />
      {busy && <Loader2 className="size-3.5 animate-spin text-muted-foreground" aria-hidden />}
      <Switch
        checked={sharing}
        disabled={busy || disabled}
        onCheckedChange={onChange}
        aria-label={sharing ? "Stop sharing" : "Start sharing"}
      />
    </div>
  );
}
