//! Optional mDNS advertisement, so the share is reachable by name.
//!
//! Gives `http://droplan-<name>.local:<port>` alongside the numeric address.
//! Strictly a convenience: the IP form always works, and every failure here is
//! logged and swallowed rather than blocking sharing.

use std::net::{IpAddr, Ipv4Addr};

use mdns_sd::{ServiceDaemon, ServiceInfo};

use crate::error::{Error, Result};

const SERVICE_TYPE: &str = "_http._tcp.local.";

/// A live registration. Dropping it unregisters and shuts the daemon down.
pub struct MdnsAdvertiser {
    daemon: ServiceDaemon,
    fullname: String,
    hostname: String,
    port: u16,
}

impl MdnsAdvertiser {
    /// The friendly URL to show next to the numeric one.
    pub fn friendly_url(&self, path: &str) -> String {
        format!(
            "http://{}:{}{path}",
            self.hostname.trim_end_matches('.'),
            self.port
        )
    }

    pub fn hostname(&self) -> &str {
        self.hostname.trim_end_matches('.')
    }

    pub fn stop(self) {
        // Explicit teardown so the name disappears from the LAN promptly
        // instead of waiting for the TTL to lapse.
        if let Err(err) = self.daemon.unregister(&self.fullname) {
            tracing::debug!(target: "droplan", "mDNS unregister failed: {err}");
        }
        if let Err(err) = self.daemon.shutdown() {
            tracing::debug!(target: "droplan", "mDNS shutdown failed: {err}");
        }
    }
}

/// Turn a machine name into a legal DNS label.
///
/// Only ASCII letters, digits and hyphens survive; the result never starts or
/// ends with a hyphen and is capped at the 63-octet label limit.
pub fn hostname_label(device_name: &str) -> String {
    let cleaned: String = device_name
        .chars()
        .map(|c| {
            let lower = c.to_ascii_lowercase();
            if lower.is_ascii_alphanumeric() {
                lower
            } else {
                '-'
            }
        })
        .collect();

    let collapsed = cleaned
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    let base = if collapsed.is_empty() {
        "device".to_string()
    } else {
        collapsed
    };

    let label = format!("droplan-{base}");
    label
        .chars()
        .take(63)
        .collect::<String>()
        .trim_end_matches('-')
        .to_string()
}

/// Advertise the share on the LAN.
pub fn advertise(device_name: &str, ip: Ipv4Addr, port: u16, path: &str) -> Result<MdnsAdvertiser> {
    let daemon = ServiceDaemon::new()
        .map_err(|err| Error::Internal(format!("mDNS daemon could not start: {err}")))?;

    let label = hostname_label(device_name);
    let hostname = format!("{label}.local.");
    let instance = format!("DropLAN on {device_name}");
    let properties = [("path", path)];

    let service = ServiceInfo::new(
        SERVICE_TYPE,
        &instance,
        &hostname,
        IpAddr::V4(ip),
        port,
        &properties[..],
    )
    .map_err(|err| Error::Internal(format!("mDNS service info is invalid: {err}")))?;

    let fullname = service.get_fullname().to_string();
    daemon
        .register(service)
        .map_err(|err| Error::Internal(format!("mDNS registration failed: {err}")))?;

    tracing::info!(target: "droplan", "advertising {hostname} on {ip}:{port}");
    Ok(MdnsAdvertiser {
        daemon,
        fullname,
        hostname,
        port,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hostname_labels_are_legal_dns() {
        assert_eq!(hostname_label("MacBook-Pro"), "droplan-macbook-pro");
        assert_eq!(hostname_label("Anna's Laptop"), "droplan-anna-s-laptop");
        assert_eq!(hostname_label("DESKTOP_A1B2"), "droplan-desktop-a1b2");
        assert_eq!(hostname_label("büro-rechner"), "droplan-b-ro-rechner");
    }

    #[test]
    fn empty_or_symbol_only_names_still_produce_a_label() {
        assert_eq!(hostname_label(""), "droplan-device");
        assert_eq!(hostname_label("---"), "droplan-device");
        assert_eq!(hostname_label("   "), "droplan-device");
    }

    #[test]
    fn labels_respect_the_dns_length_limit() {
        let label = hostname_label(&"x".repeat(200));
        assert!(label.len() <= 63);
        assert!(!label.ends_with('-'));
        assert!(label.starts_with("droplan-"));
    }

    #[test]
    fn labels_contain_only_permitted_characters() {
        let label = hostname_label("Wörk Läptop #2 (2026)!");
        assert!(label
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'));
        assert!(!label.starts_with('-'));
        assert!(!label.ends_with('-'));
        assert!(!label.contains("--"));
    }
}
