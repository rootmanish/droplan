//! Application-wide error type.
//!
//! Errors carry a stable machine-readable `code` plus a message written for a
//! person looking at the desktop UI. Technical detail is logged, not surfaced.

use std::path::Path;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("No active private network detected. Connect to Wi-Fi or Ethernet and try again.")]
    NoPrivateNetwork,

    #[error("The network interface '{0}' is no longer available.")]
    InterfaceUnavailable(String),

    #[error("Port {preferred} is unavailable and no alternative port could be allocated (tried {preferred}-{last}).")]
    NoAvailablePort { preferred: u16, last: u16 },

    #[error("Could not start sharing. {0}")]
    ServerStart(String),

    #[error("Sharing is not currently running.")]
    NotSharing,

    #[error("This file can no longer be accessed.")]
    FileUnavailable,

    #[error("'{0}' is not a regular file.")]
    NotAFile(String),

    #[error("Nothing was added. The selected items could not be read.")]
    NoFilesAdded,

    #[error("Could not save settings. {0}")]
    Settings(String),

    #[error("Could not generate a QR code for this address.")]
    QrGeneration,

    #[error(
        "The secure random number generator is unavailable, so no sharing session can be created."
    )]
    Entropy,

    #[error("{0}")]
    Io(String),

    #[error("{0}")]
    Internal(String),
}

impl Error {
    /// Stable identifier the frontend can branch on without matching strings.
    pub fn code(&self) -> &'static str {
        match self {
            Error::NoPrivateNetwork => "no_private_network",
            Error::InterfaceUnavailable(_) => "interface_unavailable",
            Error::NoAvailablePort { .. } => "no_available_port",
            Error::ServerStart(_) => "server_start",
            Error::NotSharing => "not_sharing",
            Error::FileUnavailable => "file_unavailable",
            Error::NotAFile(_) => "not_a_file",
            Error::NoFilesAdded => "no_files_added",
            Error::Settings(_) => "settings",
            Error::QrGeneration => "qr_generation",
            Error::Entropy => "entropy",
            Error::Io(_) => "io",
            Error::Internal(_) => "internal",
        }
    }

    pub fn io(context: &str, source: &std::io::Error) -> Self {
        Error::Io(format!("{context}: {source}"))
    }

    pub fn not_a_file(path: &Path) -> Self {
        Error::NotAFile(path.display().to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::Io(value.to_string())
    }
}

/// Tauri commands hand errors back to the webview as `{ code, message }`.
impl serde::Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("Error", 2)?;
        state.serialize_field("code", self.code())?;
        state.serialize_field("message", &self.to_string())?;
        state.end()
    }
}
