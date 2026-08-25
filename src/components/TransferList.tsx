import { ArrowDownToLine, CheckCircle2, XCircle } from "lucide-react";

import { Progress } from "@/components/ui/progress";
import { formatPercent, formatRelativeTime, formatSize } from "@/lib/format";
import type { TransferSnapshot } from "@/types";

function ActiveTransfer({ transfer }: { transfer: TransferSnapshot }) {
  const percent = formatPercent(transfer.transferredBytes, transfer.totalBytes);
  return (
    <li className="flex flex-col gap-1.5 rounded-lg px-3 py-2">
      <div className="flex items-baseline justify-between gap-3">
        <span className="truncate text-[13px] font-medium" title={transfer.fileName}>
          {transfer.fileName}
        </span>
        <span className="shrink-0 font-mono text-xs text-muted-foreground">{percent}%</span>
      </div>
      <Progress value={percent} />
      <div className="flex items-center justify-between gap-2 text-xs text-muted-foreground">
        <span>
          {formatSize(transfer.transferredBytes)} / {formatSize(transfer.totalBytes)}
          {transfer.isRangeRequest && " (resumed)"}
        </span>
        <span className="font-mono">{transfer.clientIp}</span>
      </div>
    </li>
  );
}

function FinishedTransfer({ transfer }: { transfer: TransferSnapshot }) {
  const failed = transfer.status === "failed";
  return (
    <li className="flex items-center gap-2.5 px-3 py-1.5 text-xs">
      {failed ? (
        <XCircle className="size-3.5 shrink-0 text-destructive" aria-hidden />
      ) : (
        <CheckCircle2 className="size-3.5 shrink-0 text-online" aria-hidden />
      )}
      <span className="min-w-0 flex-1 truncate" title={transfer.fileName}>
        {transfer.fileName}
      </span>
      <span className="shrink-0 text-muted-foreground">
        {formatSize(transfer.transferredBytes)}
      </span>
      <span className="shrink-0 font-mono text-muted-foreground">{transfer.clientIp}</span>
      {transfer.finishedAt !== null && (
        <span className="shrink-0 text-muted-foreground">
          {formatRelativeTime(transfer.finishedAt)}
        </span>
      )}
    </li>
  );
}

interface TransferListProps {
  active: TransferSnapshot[];
  recent: TransferSnapshot[];
}

/**
 * Download activity.
 *
 * Fed by throttled events from the core, so a gigabit transfer updates a few
 * times a second rather than on every chunk.
 */
export function TransferList({ active, recent }: TransferListProps) {
  if (active.length === 0 && recent.length === 0) {
    return (
      <p className="px-3 py-4 text-center text-xs text-muted-foreground">
        No downloads yet. Activity from other devices shows up here.
      </p>
    );
  }

  return (
    <div className="flex flex-col gap-2">
      {active.length > 0 && (
        <div>
          <p className="flex items-center gap-1.5 px-3 pb-1 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">
            <ArrowDownToLine className="size-3" aria-hidden />
            {active.length} active
          </p>
          <ul className="flex flex-col">
            {active.map((transfer) => (
              <ActiveTransfer key={transfer.id} transfer={transfer} />
            ))}
          </ul>
        </div>
      )}

      {recent.length > 0 && (
        <div>
          <p className="px-3 pb-1 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">
            Recent
          </p>
          <ul className="flex flex-col">
            {recent.slice(0, 8).map((transfer) => (
              <FinishedTransfer key={transfer.id} transfer={transfer} />
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}
