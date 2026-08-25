import { Cable, HelpCircle, Network, RefreshCw, Router, Shield, Wifi } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Tooltip } from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import type { InterfaceKind, NetworkInterface } from "@/types";

const AUTOMATIC = "__automatic__";

function KindIcon({ kind, className }: { kind: InterfaceKind; className?: string }) {
  const props = { className: cn("size-4 shrink-0", className), "aria-hidden": true };
  switch (kind) {
    case "wifi":
      return <Wifi {...props} />;
    case "ethernet":
      return <Cable {...props} />;
    case "vpn":
      return <Shield {...props} />;
    case "bridge":
    case "virtual":
      return <Router {...props} />;
    case "loopback":
      return <HelpCircle {...props} />;
    default:
      return <Network {...props} />;
  }
}

interface NetworkSelectorProps {
  interfaces: NetworkInterface[];
  selected: NetworkInterface | null;
  pinned: boolean;
  refreshing: boolean;
  onSelect: (name: string | null) => void;
  onRefresh: () => void;
}

/**
 * Interface picker.
 *
 * Every usable address is listed rather than silently choosing one, because on
 * a machine with Docker, a VPN and two NICs the "obvious" answer often is not.
 */
export function NetworkSelector({
  interfaces,
  selected,
  pinned,
  refreshing,
  onSelect,
  onRefresh,
}: NetworkSelectorProps) {
  const usable = interfaces.filter((entry) => entry.usable);
  const value = pinned && selected ? selected.name : AUTOMATIC;

  return (
    <div className="flex items-center gap-2">
      <Select
        value={value}
        onValueChange={(next) => onSelect(next === AUTOMATIC ? null : next)}
        disabled={usable.length === 0}
      >
        <SelectTrigger className="h-9 flex-1" aria-label="Network interface">
          <SelectValue placeholder="No network">
            <span className="flex min-w-0 items-center gap-2">
              <KindIcon kind={selected?.kind ?? "unknown"} className="text-muted-foreground" />
              <span className="truncate font-medium">{selected?.label ?? "No network"}</span>
              {selected && (
                <span className="truncate font-mono text-xs text-muted-foreground">
                  {selected.address}
                </span>
              )}
            </span>
          </SelectValue>
        </SelectTrigger>
        <SelectContent>
          <SelectItem value={AUTOMATIC}>
            <span className="flex items-center gap-2">
              <Network className="size-4 text-muted-foreground" aria-hidden />
              Choose automatically
            </span>
          </SelectItem>
          {usable.map((entry) => (
            <SelectItem key={`${entry.name}-${entry.address}`} value={entry.name}>
              <span className="flex items-center gap-2">
                <KindIcon kind={entry.kind} className="text-muted-foreground" />
                <span className="font-medium">{entry.label}</span>
                <span className="font-mono text-xs text-muted-foreground">{entry.address}</span>
                {entry.isDefaultRoute && (
                  <span className="text-[10px] uppercase tracking-wide text-muted-foreground">
                    default
                  </span>
                )}
              </span>
            </SelectItem>
          ))}
        </SelectContent>
      </Select>

      <Tooltip label="Re-detect networks">
        <Button variant="outline" size="icon" onClick={onRefresh} disabled={refreshing}>
          <RefreshCw className={cn("size-4", refreshing && "animate-spin")} aria-hidden />
          <span className="sr-only">Re-detect networks</span>
        </Button>
      </Tooltip>
    </div>
  );
}
