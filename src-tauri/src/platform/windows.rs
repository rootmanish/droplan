//! Windows specifics: adapter naming and the Defender Firewall prompt.

use crate::network::interfaces::InterfaceKind;
use crate::platform::{classify_common, PlatformNotice, TrayIcon};

pub const OS_LABEL: &str = "Windows";

/// Windows already reports friendly adapter names ("Wi-Fi", "Ethernet 2"),
/// so there is nothing to look up.
pub fn prime_caches() {}

pub fn friendly_interface_name(_name: &str) -> Option<String> {
    None
}

pub fn classify_interface(name: &str) -> InterfaceKind {
    let lower = name.to_ascii_lowercase();

    // Hyper-V and WSL surface as "vEthernet (…)"; WSL2 also uses "WSL".
    if lower.starts_with("vethernet") || lower.contains("wsl") || lower.contains("hyper-v") {
        return InterfaceKind::Virtual;
    }
    if lower.contains("bluetooth") || lower.contains("npcap") || lower.contains("teredo") {
        return InterfaceKind::Virtual;
    }
    if let Some(kind) = classify_common(&lower) {
        return kind;
    }

    if lower.contains("wi-fi")
        || lower.contains("wifi")
        || lower.contains("wireless")
        || lower.contains("wlan")
    {
        return InterfaceKind::WiFi;
    }
    if lower.contains("ethernet") || lower.contains("local area connection") {
        return InterfaceKind::Ethernet;
    }
    if lower.contains("vpn") || lower.contains("openvpn") {
        return InterfaceKind::Vpn;
    }
    InterfaceKind::Unknown
}

/// The notification area is not template-based, and a monochrome glyph would
/// disappear against a dark taskbar. The coloured mark carries its own
/// contrast on both light and dark themes.
pub fn tray_icon() -> TrayIcon {
    TrayIcon {
        bytes: include_bytes!("../../icons/tray.png"),
        is_template: false,
    }
}

pub fn firewall_notice() -> PlatformNotice {
    PlatformNotice {
        os: OS_LABEL.to_string(),
        title: "Windows Firewall must allow DropLAN on private networks".to_string(),
        body: "The first time you start sharing, Windows asks whether DropLAN may accept \
connections. Tick \"Private networks\" and allow access, otherwise other devices on your \
Wi-Fi cannot reach it. DropLAN never changes firewall rules for you. Leave \"Public networks\" \
unticked so the share stays off untrusted networks."
            .to_string(),
        action_label: Some("Open firewall settings".to_string()),
        action_url: Some("windowsdefender://network".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn friendly_windows_names_are_classified() {
        assert_eq!(classify_interface("Wi-Fi"), InterfaceKind::WiFi);
        assert_eq!(
            classify_interface("Wireless Network Connection"),
            InterfaceKind::WiFi
        );
        assert_eq!(classify_interface("Ethernet"), InterfaceKind::Ethernet);
        assert_eq!(classify_interface("Ethernet 2"), InterfaceKind::Ethernet);
    }

    #[test]
    fn hyper_v_wsl_and_docker_adapters_are_demoted() {
        for name in [
            "vEthernet (Default Switch)",
            "vEthernet (WSL)",
            "Hyper-V Virtual Ethernet Adapter",
            "Docker Desktop Bridge",
        ] {
            assert_eq!(classify_interface(name), InterfaceKind::Virtual, "{name}");
        }
    }

    #[test]
    fn loopback_and_vpn_adapters_are_recognised() {
        assert_eq!(
            classify_interface("Loopback Pseudo-Interface 1"),
            InterfaceKind::Loopback
        );
        assert_eq!(classify_interface("Tailscale"), InterfaceKind::Vpn);
        assert_eq!(
            classify_interface("OpenVPN TAP-Windows6"),
            InterfaceKind::Vpn
        );
    }
}
