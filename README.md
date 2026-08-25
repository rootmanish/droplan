<img src="assets/brand/logo.svg" alt="DropLAN" width="260">

# DropLAN

**Drop files. Share over LAN.**

Share files from your computer with any other device on the same network. Drop
files into DropLAN, scan the QR code from another device, and download them
directly in the browser.

No cloud. No account. No upload. No internet required. Your computer *is* the
server, for exactly as long as you leave it running.

```text
DropLAN (macOS / Windows / Linux)
          |
      Wi-Fi / LAN
          |
    +-----+-----+--------+
    |           |        |
  phone      laptop    tablet
    |
  browser → http://192.168.1.42:8080/s/q7Fm3Ks9
```

---

## What it does

1. Detects your active network interface and its private LAN address.
2. Starts an embedded HTTP server on the first free port from 8080 up.
3. Shows the address to hand out, plus a QR code for it.
4. Accepts files by drag and drop or from a file picker.
5. Serves exactly those files — nothing else on the disk — to devices on your LAN.
6. Streams downloads, with HTTP range support, so a 50 GB video works and can
   be resumed or seeked in a browser video player.

Files are **referenced, not copied**. Adding a 50 GB file costs a struct in
memory, and the bytes are read from where they already live.

---

## Installing DropLAN

Grab the installer for your platform from the
[Releases page](https://github.com/rootmanish/droplan/releases/latest). No
account, no installer sign-up — just download and run.

Current builds are **not code-signed** (that needs a paid Apple Developer
account and a Windows code-signing certificate, neither of which this project
has yet), so both macOS and Windows show an extra warning on first launch.
That's expected — skip to the platform steps below.

### macOS

1. Download `DropLAN_<version>_aarch64.dmg` (Apple Silicon — M1 and newer) or
   `DropLAN_<version>_x64.dmg` (Intel). Not sure which you have: Apple menu →
   About This Mac.
2. Open the `.dmg` and drag **DropLAN** into **Applications**.
3. Because the build is unsigned, double-clicking it straight away will say
   it's "damaged" or from an "unidentified developer" and refuse to open.
   Do **one** of these instead, just the first time:
   - Right-click (or Control-click) **DropLAN.app** in Applications → **Open**
     → confirm **Open** in the dialog that appears, **or**
   - Run in Terminal:
     ```bash
     xattr -dr com.apple.quarantine /Applications/DropLAN.app
     ```
4. Open DropLAN normally from now on.
5. The first time you share a file, macOS asks for **Local Network**
   permission — allow it, or no other device on your Wi-Fi can reach the app.

### Windows

1. Download the `.exe` (`DropLAN_<version>_x64-setup.exe`) or the `.msi`.
2. Run it. **Windows SmartScreen** will likely warn that this is from an
   unrecognised publisher, since it's unsigned — click **More info → Run
   anyway**.
3. Windows Defender Firewall prompts on first launch. Tick **Private
   networks** and allow; leave **Public networks** unticked.

### Linux

- **AppImage**: `chmod +x DropLAN_<version>_amd64.AppImage`, then run it
  directly.
- **.deb** (Debian/Ubuntu): `sudo dpkg -i DropLAN_<version>_amd64.deb`

No release has been published yet for this repository — the Releases page
will list installers as soon as a `v*` tag is pushed, or you can build it
yourself (see "Building it" below).

---

## Security model

The assumption is that a private LAN is *private*, not *trusted* — a café
network or an office Wi-Fi has other people on it.

| Control | What it does |
| --- | --- |
| Session token | Every URL is `/s/<22-character token>`, ~127 bits from the OS CSPRNG. `/` reveals nothing. |
| Fresh session per launch | Restarting the app invalidates every link handed out before. Shared files are never restored on startup. |
| Explicit sharing only | The HTTP layer never accepts a path. Clients name files by opaque id; ids resolve to paths canonicalised when *you* added them. |
| Immediate revocation | Removing a file or stopping sharing kills its URL at once, including transfers already in flight. |
| Constant-time comparison | Token and PIN checks do not leak their contents through timing. |
| Optional PIN | A 6-digit code in front of the share page, with a deliberate delay on each wrong guess. |
| No referrer leakage | `Referrer-Policy: no-referrer` on every response, so the token cannot escape via a `Referer` header. |
| Nothing persisted | Transfer history and client records live in memory and die with the process. |

A wrong token gets the same generic 404 as a nonexistent path, so probing
cannot distinguish "wrong token" from "no session".

---

## Technology

| Layer | Choice |
| --- | --- |
| Desktop shell | Tauri 2 |
| Frontend | React 19, TypeScript (strict), Vite |
| Styling | Tailwind CSS v4, shadcn/ui-style components on Radix |
| Core | Rust |
| Async | Tokio |
| HTTP server | Axum + tower-http |
| Interfaces | `if-addrs`, plus a routing-table probe |
| Discovery | `mdns-sd` (optional) |
| QR | `qrcode` |
| MIME | `mime_guess` |

There is no backend service, no database and no external API. Node.js is used
only as build tooling for the frontend.

---

## Architecture

```text
                     Tauri Desktop App
                            |
        +-------------------+-------------------+
        |                                       |
  React / TypeScript                          Rust
        |                                       |
  drag & drop                          network detection
  file list                            HTTP server (Axum)
  QR + share URL                       file streaming
  network picker                       session management
  transfer activity                    OS integration
        |                                       |
        +------ commands ──▶ ◀── events ────────+
                            |
                            v
                      Private LAN
```

The Rust core is deliberately free of Tauri types except in `app/` and
`commands/`. Core modules publish onto a broadcast bus; a single bridge task
forwards those events to the webview. That keeps the network, server and
transfer code unit-testable without a running desktop shell — and leaves the
door open to a headless or mobile front end later without a redesign.

### Why the listener binds `0.0.0.0`

The socket binds `0.0.0.0:<port>` while the UI shows the address of the
*selected* interface. A DHCP renewal, a Wi-Fi reconnect or waking from sleep
then changes only the address on screen, not the socket — no rebind, no dropped
downloads. `127.0.0.1` is never presented as a share address.

### Picking the right interface

A developer machine easily has a dozen IPv4 addresses: Docker bridges, VM
host-only networks, VPN tunnels, Hyper-V switches, WSL, plus the one a phone can
actually reach. DropLAN scores every candidate rather than taking the first:

1. **Default route** — found by opening a connected UDP socket, which performs a
   routing-table lookup and sends no packet. Works with no internet connection.
2. **Address class** — RFC 1918 preferred; CGNAT (Tailscale) and link-local are
   offered but ranked lower; public and loopback are never share addresses.
3. **Interface type** — Ethernet and Wi-Fi over bridges, VPNs and virtual
   adapters. Docker's `172.17.0.0/16` is demoted by address as well as by name.

Every usable interface stays visible in the picker, and your choice is
remembered. On macOS the friendly names come from `networksetup`, cached once at
startup, so `en0` is labelled by what it actually is.

---

## Prerequisites

| Tool | Version |
| --- | --- |
| Node.js | 20.19+ or 22.12+ |
| npm | 10+ |
| Rust | 1.88+ (stable) |

The Rust floor comes from the dependency graph (`image`, `icu_*`), not from
anything in this crate.

Plus the platform toolchain:

**macOS** — Xcode Command Line Tools:

```bash
xcode-select --install
```

**Windows** — Microsoft C++ Build Tools and the WebView2 runtime (preinstalled
on Windows 11 and current Windows 10).

**Linux** (Debian/Ubuntu):

```bash
sudo apt update
sudo apt install -y libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev patchelf
```

---

## Running it

```bash
npm install
npm run tauri dev
```

## Building it

```bash
npm run tauri build
```

Artifacts land in `src-tauri/target/release/bundle/`:

| Platform | Output |
| --- | --- |
| macOS | `dmg/DropLAN_<version>_<arch>.dmg`, `macos/DropLAN.app` |
| Windows | `nsis/DropLAN_<version>_x64-setup.exe`, `msi/DropLAN_<version>_x64_en-US.msi` |
| Linux | `appimage/DropLAN_<version>_amd64.AppImage`, `deb/DropLAN_<version>_amd64.deb` |

Cross-platform installers are produced by `.github/workflows/release.yml` on
any `v*` tag. `.github/workflows/ci.yml` runs lint, typecheck, clippy and the
test suite on all three platforms for every push.

## Checks

```bash
npm run lint        # ESLint, zero warnings tolerated
npm run typecheck   # tsc --noEmit, strict mode
npm test            # Vitest, the pure frontend logic
npm run rust:fmt    # cargo fmt
npm run rust:lint   # cargo clippy -D warnings
npm run rust:test   # unit + HTTP integration tests
```

---

## Firewall and permissions

DropLAN **never modifies firewall rules**. When the OS stands in the way it
explains why and links to the right settings pane.

**macOS** shows a Local Network permission prompt the first time another device
connects. Allow it, or nothing on your Wi-Fi can reach the app. If it was
dismissed: System Settings → Privacy & Security → Local Network → enable
DropLAN. `NSLocalNetworkUsageDescription` and `NSBonjourServices` are declared
in `src-tauri/Info.plist`.

**Windows** shows a Defender Firewall prompt on first start. Tick **Private
networks** and allow. Leave *Public networks* unticked so the share never
follows you onto an untrusted network.

**Linux** desktops usually leave the firewall open. If devices cannot connect:

```bash
sudo ufw allow 8080/tcp                      # ufw
sudo firewall-cmd --add-port=8080/tcp        # firewalld
```

### macOS signing and notarisation

Unsigned builds are quarantined by Gatekeeper. Either right-click → Open the
first time, or:

```bash
xattr -dr com.apple.quarantine /Applications/DropLAN.app
```

For distribution, set `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`,
`APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD` and `APPLE_TEAM_ID` as
repository secrets; the release workflow picks them up automatically.

---

## Project structure

```text
src-tauri/src/
  main.rs                  three lines; everything lives in lib.rs
  lib.rs                   module map
  error.rs                 one error type, with codes the UI can branch on
  events.rs                broadcast bus: core → UI, no Tauri types

  app/
    mod.rs                 the Tauri shell: window, commands, event bridge
    state.rs               the single owner of "what is DropLAN doing"
    tray.rs                system tray icon and menu

  network/
    interfaces.rs          enumeration, address classification, scoring
    watcher.rs             change and sleep/resume detection
    discovery.rs           mDNS advertisement

  server/
    mod.rs                 lifecycle, port selection, shared context
    routes.rs              the entire HTTP surface
    middleware.rs          session token and PIN gate
    files.rs               streaming, ETags, transfer tracking
    range.rs               RFC 9110 Range parsing
    page.rs                the server-rendered browser page

  sharing/
    registry.rs            opaque id → canonical path
    session.rs             the share token and optional PIN
    qr.rs                  QR rendering

  transfer/tracker.rs      live downloads and recent clients
  security/tokens.rs       CSPRNG tokens, constant-time comparison
  security/paths.rs        canonicalisation, filename hygiene
  settings/mod.rs          the few preferences that survive a restart
  platform/                every cfg(target_os) in the project
  commands/                the narrow React ↔ Rust boundary

src-tauri/tests/
  http_server.rs           end-to-end tests over a real socket

src/
  components/              DropZone, FileList, ShareAddress, QrCode, BrandMark, …
  components/ui/           shadcn/ui-style primitives
  pages/                   Home, Settings
  hooks/                   useSharing, useNetwork, useFiles, useTransfers
  lib/                     the typed Tauri bridge, formatting helpers (+ tests)
  types/                   mirrors the Rust serde shapes

assets/brand/              logo, icon and favicon sources (SVG masters)
public/favicon.svg         tab icon for the desktop webview
src-tauri/icons/           generated app icons, .icns and .ico
  tray-macos.png           monochrome menu-bar template
  tray.png                 coloured tray icon for Windows and Linux
.github/assets/            GitHub social preview
```

### Brand assets

Everything is generated from one master, `assets/brand/icon.svg`. To change the
icon, edit that file and re-run:

```bash
rsvg-convert -w 1024 -h 1024 assets/brand/icon.svg -o assets/brand/icon.png
npx tauri icon assets/brand/icon.png
```

The tray icon is deliberately not the app icon. macOS menu bars expect a
*template* image — pure black plus alpha, which the OS recolours for light,
dark and highlighted bars — so `tray-macos.png` is a monochrome glyph. Windows
and Linux trays are not template-based and would render that glyph invisible on
a dark taskbar, so they get the coloured mark instead. `platform::tray_icon()`
picks the right one.

### HTTP surface

```text
GET  /                          neutral landing page, reveals nothing
GET  /health                    "ok", and nothing about the machine
GET  /s/{token}                 the share page
POST /s/{token}/unlock          PIN form submission
GET  /s/{token}/api/files       JSON list; the page polls it to self-refresh
GET  /s/{token}/files/{file_id} download, with Range support
```

### Tests

217 tests — 200 in Rust, 17 in the frontend — all runnable without a network
peer:

```bash
npm run rust:test
npm test
```

Covered: private-IP classification, interface filtering and selection, session
token validation, registry behaviour, invalid file ids, path traversal
attempts, missing files, filename sanitisation, HTTP downloads, range requests
(closed, open-ended, suffix, unsatisfiable, resumed, multi-range), 304s,
concurrent downloads, large-file streaming, PIN flow, port selection and server
start/stop. The frontend tests cover the formatting helpers, including that a
file reads as the same size in the desktop window and on the browser page.

Worth doing by hand before a release, since no test can replace a real radio:
Mac → iPhone, Mac → Windows, Windows → Android, Windows → Mac, Linux → phone,
Linux → Windows.

---

## Known limitations

- **IPv4 only.** IPv6 addresses are enumerated and deliberately not offered yet.
  The address classification is written so adding them is a filter change.
- **HTTP, not HTTPS.** Traffic on your LAN is unencrypted. A trusted
  certificate for a private IP is not something a desktop app can arrange, and
  a self-signed one would train users to click through warnings. Treat the LAN
  as you would any unencrypted local network.
- **No "download all as ZIP".** Files are served individually.
- **Dropped folders are bounded** at 2000 files and 12 levels deep; symlinked
  directories are not followed.
- **Only single Range requests** are honoured. A multi-range request gets the
  whole file, which RFC 9110 permits.
- **Device identification is a guess** from the User-Agent. The IP address is
  the part that is reliable.
- **Sleep/resume detection is heuristic** — a tick arriving far later than
  scheduled. Where a platform freezes the monotonic clock across suspend, the
  regular 4-second poll catches the change a moment later anyway.
- **No mobile or headless build.** The architecture leaves room for both; the
  core has no desktop-framework dependencies.

## Licence

MIT — see [LICENSE](LICENSE).
