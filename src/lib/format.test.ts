import { describe, expect, it } from "vitest";

import {
  fileKind,
  formatPercent,
  formatRelativeTime,
  formatSize,
  interfaceKindLabel,
  splitShareUrl,
} from "@/lib/format";

describe("formatSize", () => {
  it("reads the way people expect", () => {
    expect(formatSize(0)).toBe("0 B");
    expect(formatSize(512)).toBe("512 B");
    expect(formatSize(1024)).toBe("1.0 KB");
    expect(formatSize(13_002_342)).toBe("12.4 MB");
    expect(formatSize(134_951_731)).toBe("128.7 MB");
    expect(formatSize(5 * 1024 ** 3)).toBe("5.0 GB");
    expect(formatSize(50 * 1024 ** 4)).toBe("50.0 TB");
  });

  it("matches the Rust-rendered browser page exactly", () => {
    // The same file must not read as two different sizes on the two ends.
    // These expectations are copied from `server::page::tests`.
    expect(formatSize(1024)).toBe("1.0 KB");
    expect(formatSize(13_002_342)).toBe("12.4 MB");
    expect(formatSize(147_954_073)).toBe("141.1 MB");
  });

  it("does not fall apart on nonsense input", () => {
    expect(formatSize(-1)).toBe("0 B");
    expect(formatSize(Number.NaN)).toBe("0 B");
    expect(formatSize(Number.POSITIVE_INFINITY)).toBe("0 B");
  });

  it("caps at the largest unit rather than inventing one", () => {
    expect(formatSize(9999 * 1024 ** 4)).toMatch(/TB$/);
  });
});

describe("formatPercent", () => {
  it("reports progress against the requested range", () => {
    expect(formatPercent(0, 100)).toBe(0);
    expect(formatPercent(52, 100)).toBe(52);
    expect(formatPercent(100, 100)).toBe(100);
  });

  it("treats a zero-length transfer as complete", () => {
    expect(formatPercent(0, 0)).toBe(100);
  });

  it("clamps rather than exceeding 100", () => {
    expect(formatPercent(250, 100)).toBe(100);
    expect(formatPercent(-5, 100)).toBe(0);
  });

  it("handles very large files without precision surprises", () => {
    const fiftyGb = 50 * 1024 ** 3;
    expect(formatPercent(fiftyGb / 2, fiftyGb)).toBe(50);
  });
});

describe("fileKind", () => {
  it("prefers the MIME type", () => {
    expect(fileKind("application/pdf", "a.pdf")).toBe("PDF");
    expect(fileKind("video/quicktime", "clip.mov")).toBe("Video");
    expect(fileKind("image/png", "shot.png")).toBe("Image");
    expect(fileKind("audio/mpeg", "song.mp3")).toBe("Audio");
    expect(fileKind("application/zip", "build.zip")).toBe("ZIP");
  });

  it("falls back to the extension when the MIME type says nothing", () => {
    expect(fileKind("application/octet-stream", "disk.dmg")).toBe("DMG");
    expect(fileKind("application/octet-stream", "archive.tar.gz")).toBe("GZ");
  });

  it("gives up gracefully", () => {
    expect(fileKind("application/octet-stream", "noextension")).toBe("File");
    expect(fileKind("application/octet-stream", "trailing.")).toBe("File");
    expect(fileKind("application/octet-stream", ".hidden")).toBe("File");
    expect(fileKind("application/octet-stream", "a.verylongextension")).toBe("File");
  });
});

describe("splitShareUrl", () => {
  it("separates the origin from the secret path", () => {
    expect(splitShareUrl("http://192.168.1.42:8080/s/q7Fm3Ks9")).toEqual({
      origin: "http://192.168.1.42:8080",
      path: "/s/q7Fm3Ks9",
    });
  });

  it("returns the whole string when there is no session segment", () => {
    expect(splitShareUrl("http://192.168.1.42:8080")).toEqual({
      origin: "http://192.168.1.42:8080",
      path: "",
    });
  });

  it("splits on the first session segment only", () => {
    const { origin, path } = splitShareUrl("http://host:8080/s/tok/files/abc");
    expect(origin).toBe("http://host:8080");
    expect(path).toBe("/s/tok/files/abc");
  });
});

describe("formatRelativeTime", () => {
  const now = 1_700_000_000_000;

  it("describes recent moments loosely", () => {
    expect(formatRelativeTime(now, now)).toBe("just now");
    expect(formatRelativeTime(now - 5_000, now)).toBe("just now");
    expect(formatRelativeTime(now - 30_000, now)).toBe("30s ago");
    expect(formatRelativeTime(now - 5 * 60_000, now)).toBe("5m ago");
    expect(formatRelativeTime(now - 3 * 3_600_000, now)).toBe("3h ago");
    expect(formatRelativeTime(now - 2 * 86_400_000, now)).toBe("2d ago");
  });

  it("never reports a negative duration for a clock skew", () => {
    expect(formatRelativeTime(now + 10_000, now)).toBe("just now");
  });
});

describe("interfaceKindLabel", () => {
  it("covers every kind the core can send", () => {
    // Mirrors `InterfaceKind` in the Rust core.
    expect(interfaceKindLabel("wifi")).toBe("Wi-Fi");
    expect(interfaceKindLabel("ethernet")).toBe("Ethernet");
    expect(interfaceKindLabel("vpn")).toBe("VPN");
    expect(interfaceKindLabel("bridge")).toBe("Bridge");
    expect(interfaceKindLabel("virtual")).toBe("Virtual");
    expect(interfaceKindLabel("loopback")).toBe("Loopback");
    expect(interfaceKindLabel("unknown")).toBe("Network");
  });
});
