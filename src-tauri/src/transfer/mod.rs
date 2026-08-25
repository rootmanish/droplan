//! Download activity: what is moving right now, and who asked for it.
//!
//! Purely observational. Nothing in here can change what is shared or who may
//! reach it, and none of it is persisted.

pub mod tracker;

pub use tracker::{
    ActivitySnapshot, ClientSnapshot, TransferGuard, TransferSnapshot, TransferStart,
    TransferStatus, TransferTracker,
};
