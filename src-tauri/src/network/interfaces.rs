//! Interface enumeration, address classification and "which one is the LAN?".
//!
//! A typical developer machine has a dozen IPv4 addresses: Docker bridges,
//! VM host-only networks, VPN tunnels, Hyper-V switches, plus the one address
//! a phone on the same Wi-Fi can actually reach. Picking the first entry the
//! OS returns is almost always wrong, so every candidate is scored.

use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::platform;
use crate::sharing::registry::now_millis;

/// What an interface appears to be, which drives both the score and the label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InterfaceKind {
    WiFi,
    Ethernet,
    Bridge,
    Vpn,
    Virtual,
    Loopback,
    Unknown,
}

impl InterfaceKind {
    /// Baseline desirability as a LAN sharing address.
    fn base_score(self) -> i32 {
        match self {
            InterfaceKind::Ethernet => 90,
            InterfaceKind::WiFi => 85,
            InterfaceKind::Unknown => 50,
            InterfaceKind::Bridge => 25,
            InterfaceKind::Vpn => 20,
            InterfaceKind::Virtual => 10,
            InterfaceKind::Loopback => -1000,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            InterfaceKind::WiFi => "Wi-Fi",
            InterfaceKind::Ethernet => "Ethernet",
            InterfaceKind::Bridge => "Bridge",
            InterfaceKind::Vpn => "VPN",
            InterfaceKind::Virtual => "Virtual",
            InterfaceKind::Loopback => "Loopback",
            InterfaceKind::Unknown => "Network",
        }
    }
}

/// Where an IPv4 address sits in the address space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AddressClass {
    /// RFC 1918: 10/8, 172.16/12, 192.168/16.
    Private,
    /// RFC 6598 shared address space, 100.64/10. Tailscale lives here.
    Cgnat,
    /// RFC 3927 self-assigned, 169.254/16. No DHCP, but two directly
    /// connected machines can still talk.
    LinkLocal,
    Loopback,
    /// Routable on the internet. Never used as the share address.
    Public,
}

impl AddressClass {
    pub fn of(ip: Ipv4Addr) -> Self {
        let [a, b, _, _] = ip.octets();
        if ip.is_loopback() {
            AddressClass::Loopback
        } else if a == 10 || (a == 172 && (16..=31).contains(&b)) || (a == 192 && b == 168) {
            AddressClass::Private
        } else if a == 100 && (64..=127).contains(&b) {
            AddressClass::Cgnat
        } else if a == 169 && b == 254 {
            AddressClass::LinkLocal
        } else {
            AddressClass::Public
        }
    }

    /// Whether another device on the same LAN could plausibly reach this.
    pub fn is_shareable(self) -> bool {
        matches!(
            self,
            AddressClass::Private | AddressClass::Cgnat | AddressClass::LinkLocal
        )
    }

    fn score_bonus(self) -> i32 {
        match self {
            AddressClass::Private => 60,
            AddressClass::Cgnat => 10,
            AddressClass::LinkLocal => -25,
            AddressClass::Loopback => -1000,
            AddressClass::Public => -1000,
        }
    }
}

/// Raw input to the scoring logic, so it can be exercised without real NICs.
#[derive(Debug, Clone)]
pub struct RawInterface {
    pub name: String,
    pub ip: Ipv4Addr,
    pub netmask: Option<Ipv4Addr>,
    pub is_loopback: bool,
}

/// A candidate address, as presented to the UI.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NetworkInterface {
    /// OS-level name (`en0`, `wlp3s0`, `Wi-Fi`). Stable enough to pin to.
    pub name: String,
    /// What the user sees.
    pub label: String,
    pub address: String,
    pub netmask: Option<String>,
    pub kind: InterfaceKind,
    pub address_class: AddressClass,
    /// True for the interface the OS would use for outbound traffic.
    pub is_default_route: bool,
    /// Whether DropLAN is willing to share on it at all.
    pub usable: bool,
    pub score: i32,
}

impl NetworkInterface {
    /// Identity used to pin a preference and to detect changes.
    pub fn key(&self) -> String {
        format!("{}|{}", self.name, self.address)
    }

    pub fn ipv4(&self) -> Option<Ipv4Addr> {
        self.address.parse().ok()
    }
}

/// A point-in-time view of the machine's networking.
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetworkSnapshot {
    pub interfaces: Vec<NetworkInterface>,
    pub selected: Option<NetworkInterface>,
    /// The address the OS picks for outbound traffic, when there is a route.
    pub default_route: Option<String>,
    pub detected_at: u64,
}

impl NetworkSnapshot {
    pub fn usable(&self) -> impl Iterator<Item = &NetworkInterface> {
        self.interfaces.iter().filter(|iface| iface.usable)
    }

    pub fn has_usable_interface(&self) -> bool {
        self.interfaces.iter().any(|iface| iface.usable)
    }

    /// Compact signature of "what the network looks like right now". The
    /// watcher compares these to decide whether anything actually changed,
    /// which keeps it from emitting events on every poll.
    pub fn fingerprint(&self) -> String {
        let mut keys: Vec<String> = self
            .interfaces
            .iter()
            .map(|iface| format!("{}={}", iface.name, iface.address))
            .collect();
        keys.sort();
        format!(
            "{}#{}",
            keys.join(","),
            self.default_route.as_deref().unwrap_or("-")
        )
    }
}

/// Which address the OS would source outbound traffic from.
///
/// Opening a connected UDP socket performs a routing-table lookup and nothing
/// else: no packet is sent, no name is resolved, and it works with no internet
/// connection. It is the most portable "do I have a default route, and via
/// which address?" probe available without per-OS routing APIs.
pub fn default_route_ipv4() -> Option<Ipv4Addr> {
    const PROBES: [Ipv4Addr; 3] = [
        Ipv4Addr::new(1, 1, 1, 1),
        Ipv4Addr::new(192, 168, 1, 1),
        Ipv4Addr::new(10, 0, 0, 1),
    ];

    for target in PROBES {
        let Ok(socket) = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)) else {
            continue;
        };
        if socket.connect(SocketAddrV4::new(target, 53)).is_err() {
            continue;
        }
        if let Ok(SocketAddr::V4(local)) = socket.local_addr() {
            let ip = *local.ip();
            if !ip.is_unspecified() && !ip.is_loopback() {
                return Some(ip);
            }
        }
    }
    None
}

/// Read the machine's IPv4 interfaces.
///
/// IPv6 addresses are collected but not yet offered as share addresses; the
/// shape here leaves room to add them without reworking callers.
pub fn read_raw_interfaces() -> Result<Vec<RawInterface>> {
    let mut raw = Vec::new();
    for iface in if_addrs::get_if_addrs()? {
        let is_loopback = iface.is_loopback();
        match iface.addr {
            if_addrs::IfAddr::V4(v4) => raw.push(RawInterface {
                name: iface.name,
                ip: v4.ip,
                netmask: Some(v4.netmask),
                is_loopback,
            }),
            // IPv6 is enumerated so future support is a filter change, not a
            // rewrite. Nothing consumes it yet.
            if_addrs::IfAddr::V6(_) => {}
        }
    }
    Ok(raw)
}

/// Score and label every candidate, best first.
pub fn build_interfaces(
    raw: Vec<RawInterface>,
    default_route: Option<Ipv4Addr>,
) -> Vec<NetworkInterface> {
    let mut built: Vec<NetworkInterface> = raw
        .into_iter()
        .map(|entry| {
            let kind = if entry.is_loopback {
                InterfaceKind::Loopback
            } else {
                platform::classify_interface(&entry.name)
            };
            let address_class = AddressClass::of(entry.ip);
            let is_default_route = default_route == Some(entry.ip);

            let mut score = kind.base_score() + address_class.score_bonus();
            if is_default_route {
                // Having the default route is the strongest single signal that
                // this is the network the user is actually on.
                score += 1000;
            }
            // 172.17/16 is Docker's default bridge. Even when the adapter name
            // was not recognisable, the address gives it away.
            if entry.ip.octets()[0] == 172 && entry.ip.octets()[1] == 17 {
                score -= 40;
            }

            let usable = address_class.is_shareable() && kind != InterfaceKind::Loopback;
            let label = platform::friendly_interface_name(&entry.name)
                .unwrap_or_else(|| kind.label().to_string());

            NetworkInterface {
                name: entry.name,
                label,
                address: entry.ip.to_string(),
                netmask: entry.netmask.map(|mask| mask.to_string()),
                kind,
                address_class,
                is_default_route,
                usable,
                score,
            }
        })
        .collect();

    built.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.address.cmp(&b.address))
    });
    built
}

/// Choose the interface to share on.
///
/// An explicit user preference wins whenever that interface is still present
/// and usable; a preference for something that has gone away is ignored rather
/// than being treated as an error, so unplugging a cable does not brick the app.
pub fn select_interface<'a>(
    interfaces: &'a [NetworkInterface],
    preferred_name: Option<&str>,
) -> Option<&'a NetworkInterface> {
    if let Some(preferred) = preferred_name {
        if let Some(found) = interfaces
            .iter()
            .find(|iface| iface.usable && iface.name == preferred)
        {
            return Some(found);
        }
    }
    // `build_interfaces` already sorted by score.
    interfaces.iter().find(|iface| iface.usable)
}

/// Full detection pass: enumerate, probe the default route, score, select.
pub fn detect(preferred_name: Option<&str>) -> Result<NetworkSnapshot> {
    let default_route = default_route_ipv4();
    let interfaces = build_interfaces(read_raw_interfaces()?, default_route);
    let selected = select_interface(&interfaces, preferred_name).cloned();

    Ok(NetworkSnapshot {
        interfaces,
        selected,
        default_route: default_route.map(|ip| ip.to_string()),
        detected_at: now_millis(),
    })
}

/// Convenience for logs and tests.
pub fn is_private_ipv4(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => AddressClass::of(v4) == AddressClass::Private,
        IpAddr::V6(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(name: &str, ip: &str) -> RawInterface {
        RawInterface {
            name: name.to_string(),
            ip: ip.parse().expect("ip"),
            netmask: Some(Ipv4Addr::new(255, 255, 255, 0)),
            is_loopback: ip.starts_with("127."),
        }
    }

    fn addresses(interfaces: &[NetworkInterface]) -> Vec<&str> {
        interfaces.iter().map(|i| i.address.as_str()).collect()
    }

    #[test]
    fn rfc1918_ranges_are_private() {
        for ip in [
            "10.0.0.1",
            "10.255.255.254",
            "172.16.0.1",
            "172.31.255.254",
            "192.168.0.1",
            "192.168.1.42",
        ] {
            assert_eq!(
                AddressClass::of(ip.parse().expect("ip")),
                AddressClass::Private,
                "{ip} should be private"
            );
        }
    }

    #[test]
    fn addresses_just_outside_the_private_ranges_are_public() {
        for ip in [
            "9.255.255.255",
            "11.0.0.1",
            "172.15.255.255",
            "172.32.0.1",
            "192.167.1.1",
            "192.169.1.1",
        ] {
            assert_eq!(
                AddressClass::of(ip.parse().expect("ip")),
                AddressClass::Public,
                "{ip} should not be private"
            );
        }
    }

    #[test]
    fn special_ranges_are_classified_separately() {
        assert_eq!(
            AddressClass::of("127.0.0.1".parse().expect("ip")),
            AddressClass::Loopback
        );
        assert_eq!(
            AddressClass::of("169.254.10.4".parse().expect("ip")),
            AddressClass::LinkLocal
        );
        assert_eq!(
            AddressClass::of("100.101.102.103".parse().expect("ip")),
            AddressClass::Cgnat
        );
        assert_eq!(
            AddressClass::of("8.8.8.8".parse().expect("ip")),
            AddressClass::Public
        );
    }

    #[test]
    fn only_lan_reachable_classes_are_shareable() {
        assert!(AddressClass::Private.is_shareable());
        assert!(AddressClass::Cgnat.is_shareable());
        assert!(AddressClass::LinkLocal.is_shareable());
        assert!(!AddressClass::Loopback.is_shareable());
        assert!(!AddressClass::Public.is_shareable());
    }

    #[test]
    fn loopback_is_never_usable_and_never_selected() {
        let built = build_interfaces(
            vec![raw("lo0", "127.0.0.1"), raw("en0", "192.168.1.42")],
            None,
        );
        let loopback = built.iter().find(|i| i.address == "127.0.0.1").expect("lo");
        assert!(!loopback.usable);
        assert_eq!(loopback.kind, InterfaceKind::Loopback);

        let selected = select_interface(&built, None).expect("selection");
        assert_eq!(selected.address, "192.168.1.42");
    }

    #[test]
    fn a_public_address_is_never_offered_as_the_share_address() {
        let built = build_interfaces(vec![raw("eth0", "203.0.113.10")], None);
        assert!(!built[0].usable);
        assert!(select_interface(&built, None).is_none());
    }

    #[test]
    fn the_default_route_interface_wins_over_everything_else() {
        let built = build_interfaces(
            vec![
                raw("eth0", "10.0.0.15"),
                raw("en0", "192.168.1.42"),
                raw("docker0", "172.17.0.1"),
            ],
            Some("192.168.1.42".parse().expect("ip")),
        );
        let selected = select_interface(&built, None).expect("selection");
        assert_eq!(selected.address, "192.168.1.42");
        assert!(selected.is_default_route);
        assert_eq!(addresses(&built)[0], "192.168.1.42");
    }

    #[test]
    fn virtual_adapters_lose_to_real_ones() {
        let built = build_interfaces(
            vec![
                raw("docker0", "172.17.0.1"),
                raw("vboxnet0", "192.168.56.1"),
                raw("br-9f8e7d", "172.18.0.1"),
                raw("wlp3s0", "192.168.1.77"),
            ],
            None,
        );
        let selected = select_interface(&built, None).expect("selection");
        assert_eq!(selected.address, "192.168.1.77");
    }

    #[test]
    fn a_vpn_is_listed_but_ranks_below_wifi_and_ethernet() {
        let built = build_interfaces(
            vec![
                raw("tailscale0", "100.101.102.103"),
                raw("en0", "192.168.1.42"),
            ],
            None,
        );
        assert_eq!(
            built.len(),
            2,
            "the VPN stays visible so the user can pick it"
        );
        assert_eq!(
            select_interface(&built, None).expect("selection").address,
            "192.168.1.42"
        );

        let vpn = built.iter().find(|i| i.name == "tailscale0").expect("vpn");
        assert_eq!(vpn.kind, InterfaceKind::Vpn);
        assert!(
            vpn.usable,
            "a Tailscale address is still reachable by peers"
        );
    }

    #[test]
    fn a_virtual_adapter_is_used_when_it_is_the_only_option() {
        let built = build_interfaces(vec![raw("docker0", "172.17.0.1")], None);
        let selected = select_interface(&built, None).expect("selection");
        assert_eq!(selected.address, "172.17.0.1");
    }

    #[test]
    fn an_explicit_preference_overrides_the_score() {
        let built = build_interfaces(
            vec![raw("en0", "192.168.1.42"), raw("tailscale0", "100.64.0.9")],
            Some("192.168.1.42".parse().expect("ip")),
        );
        let selected = select_interface(&built, Some("tailscale0")).expect("selection");
        assert_eq!(selected.address, "100.64.0.9");
    }

    #[test]
    fn a_preference_for_a_vanished_interface_falls_back_instead_of_failing() {
        let built = build_interfaces(vec![raw("en0", "192.168.1.42")], None);
        let selected = select_interface(&built, Some("eth7")).expect("selection");
        assert_eq!(selected.address, "192.168.1.42");
    }

    #[test]
    fn a_preference_for_an_unusable_interface_is_ignored() {
        let built = build_interfaces(
            vec![raw("lo0", "127.0.0.1"), raw("en0", "192.168.1.42")],
            None,
        );
        let selected = select_interface(&built, Some("lo0")).expect("selection");
        assert_eq!(selected.address, "192.168.1.42");
    }

    #[test]
    fn link_local_is_a_last_resort_but_still_works() {
        let built = build_interfaces(
            vec![raw("en0", "169.254.3.4"), raw("en1", "192.168.1.42")],
            None,
        );
        assert_eq!(
            select_interface(&built, None).expect("selection").address,
            "192.168.1.42"
        );

        let only_link_local = build_interfaces(vec![raw("en0", "169.254.3.4")], None);
        assert_eq!(
            select_interface(&only_link_local, None)
                .expect("selection")
                .address,
            "169.254.3.4"
        );
    }

    #[test]
    fn nothing_is_selected_when_there_is_no_lan() {
        let built = build_interfaces(vec![raw("lo0", "127.0.0.1")], None);
        assert!(select_interface(&built, None).is_none());
        assert!(!NetworkSnapshot {
            interfaces: built,
            ..Default::default()
        }
        .has_usable_interface());
    }

    #[test]
    fn ordering_is_deterministic_for_equally_scored_interfaces() {
        let a = build_interfaces(
            vec![raw("en2", "192.168.1.9"), raw("en1", "192.168.1.8")],
            None,
        );
        let b = build_interfaces(
            vec![raw("en1", "192.168.1.8"), raw("en2", "192.168.1.9")],
            None,
        );
        assert_eq!(addresses(&a), addresses(&b));
    }

    #[test]
    fn fingerprints_change_only_when_the_network_does() {
        let make = |ip: &str, route: Option<&str>| NetworkSnapshot {
            interfaces: build_interfaces(vec![raw("en0", ip)], None),
            selected: None,
            default_route: route.map(str::to_string),
            detected_at: 0,
        };

        assert_eq!(
            make("192.168.1.42", Some("192.168.1.42")).fingerprint(),
            make("192.168.1.42", Some("192.168.1.42")).fingerprint()
        );
        assert_ne!(
            make("192.168.1.42", Some("192.168.1.42")).fingerprint(),
            make("192.168.0.18", Some("192.168.0.18")).fingerprint()
        );
        // The timestamp must not participate.
        let mut later = make("192.168.1.42", None);
        later.detected_at = 999_999;
        assert_eq!(
            later.fingerprint(),
            make("192.168.1.42", None).fingerprint()
        );
    }

    #[test]
    fn multiple_addresses_on_one_interface_are_all_offered() {
        let built = build_interfaces(
            vec![raw("en0", "192.168.1.42"), raw("en0", "10.0.0.15")],
            None,
        );
        assert_eq!(built.len(), 2);
        assert_ne!(built[0].key(), built[1].key());
    }

    #[test]
    fn reading_the_real_interfaces_of_this_machine_works() {
        let raw = read_raw_interfaces().expect("enumerate interfaces");
        assert!(
            raw.iter().any(|entry| entry.is_loopback),
            "every machine has a loopback interface"
        );
        // Whatever this machine has, scoring must not panic and must never
        // promote loopback.
        let built = build_interfaces(raw, default_route_ipv4());
        if let Some(selected) = select_interface(&built, None) {
            assert!(selected.usable);
            assert_ne!(selected.address, "127.0.0.1");
        }
    }

    #[test]
    fn is_private_ipv4_matches_the_documented_ranges() {
        assert!(is_private_ipv4("192.168.1.1".parse().expect("ip")));
        assert!(is_private_ipv4("10.1.2.3".parse().expect("ip")));
        assert!(!is_private_ipv4("127.0.0.1".parse().expect("ip")));
        assert!(!is_private_ipv4("100.64.0.1".parse().expect("ip")));
        assert!(!is_private_ipv4("::1".parse().expect("ip")));
    }
}
