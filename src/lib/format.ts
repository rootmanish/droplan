/** Presentation helpers. No side effects, no framework dependencies. */

const SIZE_UNITS = ["B", "KB", "MB", "GB", "TB"] as const;

/**
 * Binary units, one decimal above bytes. Matches the browser-facing page so
 * a file reads the same on both ends.
 */
export function formatSize(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return "0 B";
  if (bytes < 1024) return `${Math.round(bytes)} B`;

  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < SIZE_UNITS.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value.toFixed(1)} ${SIZE_UNITS[unit]}`;
}

export function formatPercent(transferred: number, total: number): number {
  if (total <= 0) return 100;
  return Math.min(100, Math.max(0, Math.round((transferred / total) * 100)));
}

/** Short label for the file type column. */
export function fileKind(mimeType: string, name: string): string {
  if (mimeType.startsWith("video/")) return "Video";
  if (mimeType.startsWith("image/")) return "Image";
  if (mimeType.startsWith("audio/")) return "Audio";
  if (mimeType.startsWith("text/")) return "Text";
  if (mimeType === "application/pdf") return "PDF";
  if (mimeType === "application/zip" || mimeType === "application/x-zip-compressed") return "ZIP";

  const dot = name.lastIndexOf(".");
  if (dot > 0 && dot < name.length - 1) {
    const extension = name.slice(dot + 1).toUpperCase();
    if (extension.length <= 5) return extension;
  }
  return "File";
}

export function formatRelativeTime(timestamp: number, now = Date.now()): string {
  const seconds = Math.max(0, Math.round((now - timestamp) / 1000));
  if (seconds < 10) return "just now";
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  return `${Math.round(hours / 24)}d ago`;
}

/**
 * Split a share URL so the unguessable part can be de-emphasised without
 * hiding it. Returns the whole string as `origin` if it does not parse.
 */
export function splitShareUrl(url: string): { origin: string; path: string } {
  const marker = url.indexOf("/s/");
  if (marker === -1) return { origin: url, path: "" };
  return { origin: url.slice(0, marker), path: url.slice(marker) };
}

export function interfaceKindLabel(kind: string): string {
  switch (kind) {
    case "wifi":
      return "Wi-Fi";
    case "ethernet":
      return "Ethernet";
    case "vpn":
      return "VPN";
    case "bridge":
      return "Bridge";
    case "virtual":
      return "Virtual";
    case "loopback":
      return "Loopback";
    default:
      return "Network";
  }
}
