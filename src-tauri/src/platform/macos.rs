//! macOS specifics: hardware-port names and the Local Network prompt.

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

use crate::network::interfaces::InterfaceKind;
use crate::platform::{classify_common, PlatformNotice, TrayIcon};

pub const OS_LABEL: &str = "macOS";

/// `en0` on a laptop is Wi-Fi, on a desktop it may be Ethernet. Rather than
/// guessing, we ask `networksetup` once and cache the answer; the heuristic
/// below is only the fallback for when that lookup is unavailable.
static HARDWARE_PORTS: OnceLock<RwLock<HashMap<String, String>>> = OnceLock::new();

fn cache() -> &'static RwLock<HashMap<String, String>> {
    HARDWARE_PORTS.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Parse the output of `networksetup -listallhardwareports`, which looks like:
///
/// ```text
/// Hardware Port: Wi-Fi
/// Device: en0
/// Ethernet Address: ...
/// ```
fn parse_hardware_ports(output: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut current_port: Option<String> = None;

    for line in output.lines() {
        let line = line.trim();
        if let Some(port) = line.strip_prefix("Hardware Port:") {
            current_port = Some(port.trim().to_string());
        } else if let Some(device) = line.strip_prefix("Device:") {
            if let Some(port) = current_port.take() {
                map.insert(device.trim().to_string(), port);
            }
        }
    }
    map
}

/// Populate the cache. Called on a background thread at startup; a failure
/// just means we fall back to name heuristics.
pub fn prime_caches() {
    let output = std::process::Command::new("/usr/sbin/networksetup")
        .arg("-listallhardwareports")
        .output();

    let Ok(output) = output else {
        tracing::debug!(target: "droplan", "networksetup unavailable; using interface name heuristics");
        return;
    };
    if !output.status.success() {
        return;
    }
    let parsed = parse_hardware_ports(&String::from_utf8_lossy(&output.stdout));
    if let Ok(mut guard) = cache().write() {
        *guard = parsed;
    }
}

fn hardware_port(name: &str) -> Option<String> {
    cache().read().ok()?.get(name).cloned()
}

pub fn friendly_interface_name(name: &str) -> Option<String> {
    hardware_port(name)
}

pub fn classify_interface(name: &str) -> InterfaceKind {
    let lower = name.to_ascii_lowercase();

    // Apple's own link-local peer-to-peer radios: never a LAN route.
    if lower.starts_with("awdl") || lower.starts_with("llw") || lower.starts_with("anpi") {
        return InterfaceKind::Virtual;
    }
    // The Wi-Fi hotspot interface when Internet Sharing is on.
    if lower.starts_with("ap") && lower.len() <= 4 {
        return InterfaceKind::Virtual;
    }
    if let Some(kind) = classify_common(&lower) {
        return kind;
    }

    if let Some(port) = hardware_port(name) {
        let port_lower = port.to_ascii_lowercase();
        if port_lower.contains("wi-fi")
            || port_lower.contains("wifi")
            || port_lower.contains("airport")
        {
            return InterfaceKind::WiFi;
        }
        if port_lower.contains("bridge") {
            return InterfaceKind::Bridge;
        }
        if port_lower.contains("ethernet")
            || port_lower.contains("lan")
            || port_lower.contains("thunderbolt")
        {
            return InterfaceKind::Ethernet;
        }
    }

    // Fallback heuristic: on Apple laptops en0 is the Wi-Fi radio.
    if lower == "en0" {
        return InterfaceKind::WiFi;
    }
    if lower.starts_with("en") {
        return InterfaceKind::Ethernet;
    }
    InterfaceKind::Unknown
}

/// Menu-bar glyph. Pure black plus alpha so macOS can recolour it for light,
/// dark and highlighted menu bars; a coloured icon there looks out of place.
pub fn tray_icon() -> TrayIcon {
    TrayIcon {
        bytes: include_bytes!("../../icons/tray-macos.png"),
        is_template: true,
    }
}

pub fn firewall_notice() -> PlatformNotice {
    PlatformNotice {
        os: OS_LABEL.to_string(),
        title: "macOS may ask for Local Network access".to_string(),
        body: "The first time another device connects, macOS shows a Local Network prompt. \
Allow it, otherwise phones and laptops on your Wi-Fi cannot reach DropLAN. If you dismissed \
the prompt, re-enable DropLAN under Privacy & Security → Local Network."
            .to_string(),
        action_label: Some("Open Local Network settings".to_string()),
        action_url: Some(
            "x-apple.systempreferences:com.apple.preference.security?Privacy_LocalNetwork"
                .to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hardware_port_output_is_parsed() {
        let sample = "\
Hardware Port: Wi-Fi
Device: en0
Ethernet Address: aa:bb:cc:dd:ee:ff

Hardware Port: Thunderbolt Ethernet
Device: en5
Ethernet Address: 11:22:33:44:55:66

VLAN Configurations
===================
";
        let parsed = parse_hardware_ports(sample);
        assert_eq!(parsed.get("en0").map(String::as_str), Some("Wi-Fi"));
        assert_eq!(
            parsed.get("en5").map(String::as_str),
            Some("Thunderbolt Ethernet")
        );
        assert_eq!(parsed.len(), 2);
    }

    #[test]
    fn parsing_tolerates_empty_or_garbage_output() {
        assert!(parse_hardware_ports("").is_empty());
        assert!(parse_hardware_ports("nothing useful here").is_empty());
        assert!(
            parse_hardware_ports("Device: en0").is_empty(),
            "a device with no port is ignored"
        );
    }

    #[test]
    fn apple_virtual_radios_are_never_treated_as_lan() {
        for name in ["awdl0", "llw0", "anpi0", "ap1"] {
            assert_eq!(classify_interface(name), InterfaceKind::Virtual, "{name}");
        }
    }

    #[test]
    fn vpn_and_bridge_adapters_are_recognised() {
        assert_eq!(classify_interface("utun0"), InterfaceKind::Vpn);
        assert_eq!(classify_interface("bridge100"), InterfaceKind::Bridge);
        assert_eq!(classify_interface("lo0"), InterfaceKind::Loopback);
    }

    #[test]
    fn en_devices_fall_back_to_sensible_kinds() {
        // Without a primed cache these use the name heuristic.
        assert_eq!(classify_interface("en0"), InterfaceKind::WiFi);
        assert_eq!(classify_interface("en7"), InterfaceKind::Ethernet);
    }
}
