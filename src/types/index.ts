/**
 * Mirrors the serde representation of the Rust core.
 *
 * Every struct on the Rust side is `#[serde(rename_all = "camelCase")]`, so
 * these names line up one for one. Keep the two in step when either changes.
 */

export type InterfaceKind =
  | "wifi"
  | "ethernet"
  | "bridge"
  | "vpn"
  | "virtual"
  | "loopback"
  | "unknown";

export type AddressClass = "private" | "cgnat" | "link-local" | "loopback" | "public";

export interface NetworkInterface {
  name: string;
  label: string;
  address: string;
  netmask: string | null;
  kind: InterfaceKind;
  addressClass: AddressClass;
  isDefaultRoute: boolean;
  usable: boolean;
  score: number;
}

export interface NetworkSnapshot {
  interfaces: NetworkInterface[];
  selected: NetworkInterface | null;
  defaultRoute: string | null;
  detectedAt: number;
}

export interface SessionInfo {
  token: string;
  basePath: string;
  pin: string | null;
  startedAt: number;
}

export interface ShareItem {
  id: string;
  displayName: string;
  mimeType: string;
  size: number;
  addedAt: number;
  available: boolean;
}

export interface RegistryTotals {
  fileCount: number;
  totalBytes: number;
  unavailableCount: number;
}

export interface AppSettings {
  preferredPort: number;
  preferredInterface: string | null;
  startSharingOnLaunch: boolean;
  enableMdns: boolean;
  requirePin: boolean;
  closeToTray: boolean;
  portScanRange: number;
}

export interface PlatformNotice {
  os: string;
  title: string;
  body: string;
  actionLabel: string | null;
  actionUrl: string | null;
}

export interface ShareState {
  sharing: boolean;
  deviceName: string;
  network: NetworkSnapshot;
  session: SessionInfo | null;
  port: number | null;
  shareUrl: string | null;
  friendlyUrl: string | null;
  files: ShareItem[];
  totals: RegistryTotals;
  settings: AppSettings;
  platformNotice: PlatformNotice;
}

export interface AddOutcome {
  added: ShareItem[];
  skippedDuplicates: number;
  skippedUnreadable: number;
  truncated: boolean;
}

export interface SharedFilesPayload {
  files: ShareItem[];
  totals: RegistryTotals;
}

export type TransferStatus = "active" | "completed" | "failed";

export interface TransferSnapshot {
  id: string;
  fileId: string;
  fileName: string;
  /** Bytes this response delivers — the range length, not the file size. */
  totalBytes: number;
  fileBytes: number;
  transferredBytes: number;
  isRangeRequest: boolean;
  clientIp: string;
  userAgent: string | null;
  status: TransferStatus;
  startedAt: number;
  finishedAt: number | null;
}

export interface ClientSnapshot {
  ip: string;
  userAgent: string | null;
  device: string;
  browser: string;
  requests: number;
  firstSeen: number;
  lastSeen: number;
}

export interface ActivitySnapshot {
  active: TransferSnapshot[];
  recent: TransferSnapshot[];
  clients: ClientSnapshot[];
  totalBytesServed: number;
}

/** What a failing Tauri command rejects with. */
export interface CoreError {
  code: string;
  message: string;
}

export interface Notice {
  code: string;
  message: string;
}

/** Event names emitted by the Rust core. Keep in sync with `events.rs`. */
export const EVENTS = {
  networkChanged: "network-changed",
  sharingStarted: "sharing-started",
  sharingStopped: "sharing-stopped",
  sharedFilesChanged: "shared-files-changed",
  transferStarted: "transfer-started",
  transferProgress: "transfer-progress",
  transferCompleted: "transfer-completed",
  transferFailed: "transfer-failed",
  clientsChanged: "clients-changed",
  systemResumed: "system-resumed",
  notice: "notice",
} as const;
