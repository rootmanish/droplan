import { Laptop, Monitor, Smartphone, Tablet } from "lucide-react";

import { formatRelativeTime } from "@/lib/format";
import type { ClientSnapshot } from "@/types";

function DeviceIcon({ device }: { device: string }) {
  const props = { className: "size-4 shrink-0 text-muted-foreground", "aria-hidden": true };
  switch (device) {
    case "iPhone":
    case "Android":
      return <Smartphone {...props} />;
    case "iPad":
      return <Tablet {...props} />;
    case "Mac":
      return <Laptop {...props} />;
    case "Windows":
    case "Linux":
    case "ChromeOS":
      return <Monitor {...props} />;
    default:
      return <Monitor {...props} />;
  }
}

/**
 * Devices that have talked to us this session.
 *
 * Identification is a guess from the User-Agent and is labelled as such; the
 * IP address is the part that is actually reliable.
 */
export function DeviceList({ clients }: { clients: ClientSnapshot[] }) {
  if (clients.length === 0) {
    return (
      <p className="px-3 py-4 text-center text-xs text-muted-foreground">
        No devices have connected yet.
      </p>
    );
  }

  return (
    <ul className="flex flex-col">
      {clients.map((client) => (
        <li key={client.ip} className="flex items-center gap-2.5 px-3 py-1.5 text-xs">
          <DeviceIcon device={client.device} />
          <span className="font-mono">{client.ip}</span>
          <span className="min-w-0 flex-1 truncate text-muted-foreground">
            {client.device} · {client.browser}
          </span>
          <span className="shrink-0 text-muted-foreground">
            {formatRelativeTime(client.lastSeen)}
          </span>
        </li>
      ))}
    </ul>
  );
}
