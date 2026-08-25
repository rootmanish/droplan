//! OS-specific behaviour, kept behind one small interface.
//!
//! Everything platform-conditional in DropLAN lives in this module. The rest
//! of the codebase calls these functions and never uses `cfg(target_os)`.

use serde::Serialize;

use crate::network::interfaces::InterfaceKind;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as imp;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows as imp;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as imp;

// Any other Unix (FreeBSD and friends) reuses the Linux naming conventions,
// which are close enough to be useful and never worse than "Unknown".
#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
mod linux;
#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
use linux as imp;

/// Guidance shown in the UI when the OS is likely to stand between DropLAN and
/// the LAN. Purely informational: DropLAN never edits firewall rules itself.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformNotice {
    pub os: String,
    pub title: String,
    pub body: String,
    /// Optional deep link into the relevant system settings pane.
    pub action_label: Option<String>,
    pub action_url: Option<String>,
}

/// The tray image for this OS, and whether the platform treats it as a
/// *template*: a monochrome glyph the system recolours for light, dark and
/// highlighted menu bars.
pub struct TrayIcon {
    pub bytes: &'static [u8],
    pub is_template: bool,
}

/// Tray artwork appropriate to this platform.
///
/// macOS wants a template glyph; Windows and Linux would render that same
/// glyph invisible on a dark taskbar and want the coloured mark instead.
pub fn tray_icon() -> TrayIcon {
    imp::tray_icon()
}

/// Human-readable machine name, used as the browser page title and the mDNS
/// instance name. Falls back to something generic rather than failing.
pub fn device_name() -> String {
    let raw = hostname::get()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default();

    // macOS reports `Some-Name.local`; the suffix is noise in a page title.
    let trimmed = raw.trim().trim_end_matches(".local");
    if trimmed.is_empty() {
        "This computer".to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn os_label() -> &'static str {
    imp::OS_LABEL
}

/// Best guess at what an interface actually is, from its OS-level name.
pub fn classify_interface(name: &str) -> InterfaceKind {
    imp::classify_interface(name)
}

/// Nicer label for the UI, when the OS name is cryptic. Returns `None` when
/// the raw name is already the best thing to show.
pub fn friendly_interface_name(name: &str) -> Option<String> {
    imp::friendly_interface_name(name)
}

/// Firewall / permission guidance for this OS.
pub fn firewall_notice() -> PlatformNotice {
    imp::firewall_notice()
}

/// Warm any cache the platform layer keeps, off the UI thread.
pub fn prime_caches() {
    imp::prime_caches();
}

/// Shared name-matching used by every platform, applied before the
/// OS-specific rules so that well-known virtual adapters are always caught.
pub(crate) fn classify_common(lower: &str) -> Option<InterfaceKind> {
    /// Matched anywhere in the name. These strings are distinctive enough that
    /// a substring hit is not a false positive.
    const VIRTUAL_SUBSTRINGS: [&str; 12] = [
        "docker",
        "podman",
        "virbr",
        "vmnet",
        "vboxnet",
        "vmenet",
        "hyper-v",
        "vethernet",
        "kubernetes",
        "flannel",
        "weave",
        "lxcbr",
    ];
    /// Matched only at the start, because these are short and would otherwise
    /// collide with ordinary adapter names.
    const VIRTUAL_PREFIXES: [&str; 5] = ["veth", "vnic", "zt", "cni", "tap"];
    const VPN_SUBSTRINGS: [&str; 4] = ["tailscale", "wireguard", "nordlynx", "ipsec"];
    const VPN_PREFIXES: [&str; 5] = ["utun", "tun", "wg", "ppp", "ipsec"];

    if lower == "lo" || lower == "lo0" || lower.contains("loopback") {
        return Some(InterfaceKind::Loopback);
    }
    if VIRTUAL_SUBSTRINGS
        .iter()
        .any(|marker| lower.contains(marker))
        || VIRTUAL_PREFIXES
            .iter()
            .any(|marker| lower.starts_with(marker))
    {
        return Some(InterfaceKind::Virtual);
    }
    if lower.starts_with("br-") || lower.starts_with("bridge") || lower == "br0" {
        return Some(InterfaceKind::Bridge);
    }
    if VPN_SUBSTRINGS.iter().any(|marker| lower.contains(marker))
        || VPN_PREFIXES.iter().any(|marker| lower.starts_with(marker))
    {
        return Some(InterfaceKind::Vpn);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_name_is_never_empty() {
        assert!(!device_name().is_empty());
        assert!(!device_name().ends_with(".local"));
    }

    #[test]
    fn common_classification_catches_container_and_vpn_adapters() {
        assert_eq!(classify_common("docker0"), Some(InterfaceKind::Virtual));
        assert_eq!(classify_common("br-1a2b3c"), Some(InterfaceKind::Bridge));
        assert_eq!(classify_common("veth9f2a"), Some(InterfaceKind::Virtual));
        assert_eq!(classify_common("vboxnet0"), Some(InterfaceKind::Virtual));
        assert_eq!(classify_common("tailscale0"), Some(InterfaceKind::Vpn));
        assert_eq!(classify_common("utun3"), Some(InterfaceKind::Vpn));
        assert_eq!(classify_common("lo"), Some(InterfaceKind::Loopback));
        assert_eq!(classify_common("lo0"), Some(InterfaceKind::Loopback));
        assert_eq!(classify_common("wlan0"), None, "real adapters fall through");
    }

    #[test]
    fn the_tray_icon_is_a_usable_png() {
        let icon = tray_icon();
        assert!(!icon.bytes.is_empty());
        assert_eq!(&icon.bytes[..8], b"\x89PNG\r\n\x1a\n", "must be a PNG");
        // macOS is the only platform whose tray recolours a template glyph.
        assert_eq!(icon.is_template, cfg!(target_os = "macos"));
    }

    #[test]
    fn every_platform_publishes_a_firewall_notice() {
        let notice = firewall_notice();
        assert!(!notice.title.is_empty());
        assert!(!notice.body.is_empty());
        assert!(!notice.os.is_empty());
    }
}
