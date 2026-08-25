//! Everything about "which address should other devices use?".
//!
//! [`interfaces`] enumerates and ranks candidates, [`watcher`] notices when
//! that answer changes, and [`discovery`] optionally publishes a `.local`
//! name so the address does not have to be typed at all.

pub mod discovery;
pub mod interfaces;
pub mod watcher;

pub use interfaces::{
    AddressClass, InterfaceKind, NetworkInterface, NetworkSnapshot, RawInterface,
};
pub use watcher::{NetworkEvent, NetworkWatcher};
