//! Linux specifics: predictable interface names and firewall guidance.

use crate::network::interfaces::InterfaceKind;
use crate::platform::{classify_common, PlatformNotice, TrayIcon};

pub const OS_LABEL: &str = "Linux";

pub fn prime_caches() {}

pub fn friendly_interface_name(_name: &str) -> Option<String> {
    None
}

/// systemd's predictable names encode the type in the first two characters:
/// `wl*` wireless, `en*` ethernet, `ww*` mobile broadband. The legacy names
/// (`eth0`, `wlan0`) are still common enough to handle explicitly.
pub fn classify_interface(name: &str) -> InterfaceKind {
    let lower = name.to_ascii_lowercase();

    if let Some(kind) = classify_common(&lower) {
        return kind;
    }
    if lower.starts_with("wl") {
        return InterfaceKind::WiFi;
    }
    if lower.starts_with("en") || lower.starts_with("eth") || lower.starts_with("eno") {
        return InterfaceKind::Ethernet;
    }
    if lower.starts_with("ww") {
        // Mobile broadband: routable, but usually metered and not a LAN.
        return InterfaceKind::Unknown;
    }
    InterfaceKind::Unknown
}

/// Tray implementations vary across desktops and none of them recolour a
/// template image, so the coloured mark is the safe choice.
pub fn tray_icon() -> TrayIcon {
    TrayIcon {
        bytes: include_bytes!("../../icons/tray.png"),
        is_template: false,
    }
}

pub fn firewall_notice() -> PlatformNotice {
    PlatformNotice {
        os: OS_LABEL.to_string(),
        title: "A local firewall may block incoming connections".to_string(),
        body: "Most desktop distributions leave the firewall open, but if other devices cannot \
reach DropLAN you may need to allow the port. With ufw that is `sudo ufw allow <port>/tcp`, \
with firewalld `sudo firewall-cmd --add-port=<port>/tcp`. DropLAN never changes firewall \
rules for you."
            .to_string(),
        action_label: None,
        action_url: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn predictable_and_legacy_names_are_classified() {
        assert_eq!(classify_interface("wlp3s0"), InterfaceKind::WiFi);
        assert_eq!(classify_interface("wlan0"), InterfaceKind::WiFi);
        assert_eq!(classify_interface("enp0s31f6"), InterfaceKind::Ethernet);
        assert_eq!(classify_interface("eth0"), InterfaceKind::Ethernet);
        assert_eq!(classify_interface("eno1"), InterfaceKind::Ethernet);
    }

    #[test]
    fn container_and_vpn_adapters_are_demoted() {
        assert_eq!(classify_interface("docker0"), InterfaceKind::Virtual);
        assert_eq!(classify_interface("br-4f1c2d"), InterfaceKind::Bridge);
        assert_eq!(classify_interface("veth1a2b"), InterfaceKind::Virtual);
        assert_eq!(classify_interface("virbr0"), InterfaceKind::Virtual);
        assert_eq!(classify_interface("tailscale0"), InterfaceKind::Vpn);
        assert_eq!(classify_interface("wg0"), InterfaceKind::Vpn);
        assert_eq!(classify_interface("lo"), InterfaceKind::Loopback);
    }
}
