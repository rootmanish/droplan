/**
 * The only place the frontend talks to Rust.
 *
 * Every command is wrapped once, with its argument and return types spelled
 * out, so component code never calls `invoke` with a string and never sees an
 * `any`. Errors arrive as `{ code, message }` and are normalised here.
 */

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";

import type {
  ActivitySnapshot,
  AddOutcome,
  AppSettings,
  CoreError,
  NetworkInterface,
  NetworkSnapshot,
  PlatformNotice,
  ShareState,
  SharedFilesPayload,
} from "@/types";

/** An error raised by the Rust core, carrying a stable code. */
export class DropLANError extends Error {
  readonly code: string;

  constructor(code: string, message: string) {
    super(message);
    this.name = "DropLANError";
    this.code = code;
  }
}

function isCoreError(value: unknown): value is CoreError {
  return (
    typeof value === "object" &&
    value !== null &&
    typeof (value as CoreError).code === "string" &&
    typeof (value as CoreError).message === "string"
  );
}

/** Normalise anything a rejected command throws into a `DropLANError`. */
export function toDropLANError(error: unknown): DropLANError {
  if (error instanceof DropLANError) return error;
  if (isCoreError(error)) return new DropLANError(error.code, error.message);
  if (error instanceof Error) return new DropLANError("unknown", error.message);
  return new DropLANError("unknown", String(error));
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw toDropLANError(error);
  }
}

// ------------------------------------------------------------------- sharing

export const getShareState = () => call<ShareState>("get_share_state");
export const startSharing = () => call<ShareState>("start_sharing");
export const stopSharing = () => call<ShareState>("stop_sharing");
export const regenerateShareSession = () => call<ShareState>("regenerate_share_session");
export const getQrSvg = (url: string) => call<string>("get_qr_svg", { url });
export const getTransferActivity = () => call<ActivitySnapshot>("get_transfer_activity");

// ------------------------------------------------------------------- network

export const getNetworkInterfaces = () => call<NetworkInterface[]>("get_network_interfaces");
export const getCurrentNetwork = () => call<NetworkSnapshot>("get_current_network");
export const refreshNetwork = () => call<NetworkSnapshot>("refresh_network");
export const getPlatformNotice = () => call<PlatformNotice>("get_platform_notice");

/** Pass `null` to go back to automatic interface selection. */
export const setPreferredInterface = (name: string | null) =>
  call<ShareState>("set_preferred_interface", { name });

export const setPreferredPort = (port: number) => call<ShareState>("set_preferred_port", { port });

// --------------------------------------------------------------------- files

export const addSharedFiles = (paths: string[]) => call<AddOutcome>("add_shared_files", { paths });
export const removeSharedFile = (id: string) => call<boolean>("remove_shared_file", { id });
export const clearSharedFiles = () => call<number>("clear_shared_files");
export const refreshSharedFiles = () => call<SharedFilesPayload>("refresh_shared_files");
export const getSharedFiles = () => call<SharedFilesPayload>("get_shared_files");

// ------------------------------------------------------------------ settings

export const getSettings = () => call<AppSettings>("get_settings");
export const updateSettings = (settings: AppSettings) =>
  call<ShareState>("update_settings", { settings });

// -------------------------------------------------------------------- events

/**
 * Subscribe to a core event. The returned promise resolves to the unlisten
 * function; callers should await it in a cleanup-safe way.
 */
export function onCoreEvent<T>(name: string, handler: (payload: T) => void): Promise<UnlistenFn> {
  return listen<T>(name, (event) => handler(event.payload));
}

export type DragDropState = "over" | "drop" | "leave";

/**
 * Native drag and drop.
 *
 * Tauri gives us real filesystem paths here rather than browser `File`
 * objects, which is what lets a 50 GB video be shared by reference instead of
 * being read into the webview.
 */
export function onFileDrop(handlers: {
  onEnter?: () => void;
  onLeave?: () => void;
  onDrop: (paths: string[]) => void;
}): Promise<UnlistenFn> {
  return getCurrentWebview().onDragDropEvent((event) => {
    switch (event.payload.type) {
      case "over":
        handlers.onEnter?.();
        break;
      case "drop":
        handlers.onDrop(event.payload.paths);
        break;
      case "leave":
        handlers.onLeave?.();
        break;
    }
  });
}
