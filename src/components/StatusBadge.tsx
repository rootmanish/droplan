import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";

/**
 * The one thing a glance at the window must answer: is this machine serving
 * files right now?
 */
export function StatusBadge({ sharing }: { sharing: boolean }) {
  return (
    <Badge variant={sharing ? "live" : "muted"} className="px-2.5 py-1">
      <span
        className={cn(
          "size-1.5 rounded-full",
          sharing ? "bg-live-foreground animate-live-pulse" : "bg-muted-foreground",
        )}
        aria-hidden
      />
      {sharing ? "Sharing on" : "Sharing off"}
    </Badge>
  );
}
