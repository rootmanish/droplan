//! Path handling for files the user has explicitly shared.
//!
//! The HTTP surface never accepts a path. Clients address files by opaque id
//! and the id resolves, server side, to a path that was canonicalised at the
//! moment the user added it. The helpers here exist to make that resolution
//! safe and to keep hostile filenames out of response headers.

use std::path::{Component, Path, PathBuf};

use crate::error::{Error, Result};

/// Filenames longer than this are truncated before being put in a header.
const MAX_FILENAME_LEN: usize = 180;

/// Resolve a user-selected path to an absolute, symlink-free path and confirm
/// it is a regular file we can actually open.
///
/// Canonicalisation happens once, at add time, while the user is present. A
/// symlink is followed here deliberately: the user picked it, and we then
/// remember the *resolved* target so a later swap of the link cannot redirect
/// us somewhere new.
pub fn canonicalize_shared_file(path: &Path) -> Result<PathBuf> {
    let canonical = std::fs::canonicalize(path).map_err(|err| {
        tracing::debug!(target: "droplan", "canonicalize failed for {}: {err}", path.display());
        Error::FileUnavailable
    })?;

    let metadata = std::fs::metadata(&canonical).map_err(|err| {
        tracing::debug!(target: "droplan", "metadata failed for {}: {err}", canonical.display());
        Error::FileUnavailable
    })?;

    if !metadata.is_file() {
        return Err(Error::not_a_file(&canonical));
    }
    Ok(canonical)
}

/// Same as [`canonicalize_shared_file`] but for a directory the user dropped.
pub fn canonicalize_shared_dir(path: &Path) -> Result<PathBuf> {
    let canonical = std::fs::canonicalize(path).map_err(|_| Error::FileUnavailable)?;
    if !canonical.is_dir() {
        return Err(Error::not_a_file(&canonical));
    }
    Ok(canonical)
}

/// Reduce an arbitrary string to something safe to echo back in a
/// `Content-Disposition` header or render into HTML.
///
/// Strips directory separators, control characters and the Windows-reserved
/// set, collapses whitespace, and never returns an empty string.
pub fn sanitize_filename(name: &str) -> String {
    let mut cleaned: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();

    // Leading dots and the underscores left behind by stripped separators are
    // trimmed so `../../etc/passwd` reads as `etc_passwd` rather than
    // `_.._etc_passwd`, and so nothing resolves to `.`, `..` or a dotfile.
    // Trailing dots are trimmed too: Windows silently drops them.
    cleaned = cleaned
        .trim_matches(|c: char| c == '.' || c == '_' || c.is_whitespace())
        .to_string();

    if cleaned.chars().count() > MAX_FILENAME_LEN {
        let head: String = cleaned.chars().take(MAX_FILENAME_LEN).collect();
        cleaned = head;
    }

    if cleaned.is_empty() {
        return "download".to_string();
    }
    cleaned
}

/// Build a `Content-Disposition` value that survives non-ASCII filenames.
///
/// Emits the ASCII-only `filename=` for old clients plus RFC 5987
/// `filename*=UTF-8''…` which every current browser prefers.
pub fn content_disposition(name: &str, inline: bool) -> String {
    let safe = sanitize_filename(name);
    let disposition = if inline { "inline" } else { "attachment" };

    let ascii_fallback: String = safe
        .chars()
        .map(|c| {
            if c.is_ascii() && c != '"' && c != '\\' {
                c
            } else {
                '_'
            }
        })
        .collect();

    // RFC 5987 attr-char: alphanumerics plus a small set of safe punctuation.
    // Keeping `.` and `-` unencoded leaves the extension readable.
    const ATTR_CHAR: &percent_encoding::AsciiSet = &percent_encoding::NON_ALPHANUMERIC
        .remove(b'!')
        .remove(b'#')
        .remove(b'$')
        .remove(b'&')
        .remove(b'+')
        .remove(b'-')
        .remove(b'.')
        .remove(b'^')
        .remove(b'_')
        .remove(b'`')
        .remove(b'|')
        .remove(b'~');

    let encoded = percent_encoding::utf8_percent_encode(&safe, ATTR_CHAR).to_string();

    format!("{disposition}; filename=\"{ascii_fallback}\"; filename*=UTF-8''{encoded}")
}

/// Display label for a file discovered inside a shared folder.
///
/// Produces `folder/sub/file.ext` using forward slashes on every platform so
/// the browser page looks the same everywhere. Any component that is not a
/// plain name is dropped rather than being allowed to escape the prefix.
pub fn relative_display_name(root: &Path, file: &Path) -> String {
    let Some(root_parent) = root.parent() else {
        return file
            .file_name()
            .map(|n| sanitize_filename(&n.to_string_lossy()))
            .unwrap_or_else(|| "download".to_string());
    };

    let relative = file.strip_prefix(root_parent).unwrap_or(file);
    let parts: Vec<String> = relative
        .components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(sanitize_filename(&part.to_string_lossy())),
            _ => None,
        })
        .collect();

    if parts.is_empty() {
        return "download".to_string();
    }
    parts.join("/")
}

/// Guess a MIME type from the extension, defaulting to a byte stream.
pub fn guess_mime(path: &Path) -> String {
    mime_guess::from_path(path)
        .first_raw()
        .unwrap_or("application/octet-stream")
        .to_string()
}

/// Types a browser can present in-page rather than only downloading.
pub fn is_inline_previewable(mime: &str) -> bool {
    mime.starts_with("image/")
        || mime.starts_with("video/")
        || mime.starts_with("audio/")
        || mime == "application/pdf"
        || mime.starts_with("text/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_strips_traversal_and_separators() {
        assert_eq!(sanitize_filename("../../etc/passwd"), "etc_passwd");
        assert_eq!(
            sanitize_filename("..\\..\\windows\\system32"),
            "windows_system32"
        );
        assert_eq!(sanitize_filename("/absolute/path.txt"), "absolute_path.txt");
        assert_eq!(sanitize_filename(".."), "download");
        assert_eq!(sanitize_filename("."), "download");
        assert_eq!(sanitize_filename(""), "download");
        assert_eq!(sanitize_filename("   "), "download");
        assert_eq!(sanitize_filename("..."), "download");
        assert_eq!(sanitize_filename(".hidden"), "hidden");
        assert_eq!(sanitize_filename("trailing..."), "trailing");
    }

    #[test]
    fn sanitize_removes_control_characters() {
        assert_eq!(sanitize_filename("re\nport\t.pdf"), "re_port_.pdf");
        assert_eq!(sanitize_filename("a\u{0000}b"), "a_b");
    }

    #[test]
    fn sanitize_keeps_ordinary_names_intact() {
        assert_eq!(sanitize_filename("report.pdf"), "report.pdf");
        assert_eq!(
            sanitize_filename("holiday photo (1).jpeg"),
            "holiday photo (1).jpeg"
        );
        assert_eq!(
            sanitize_filename("bericht-über-alles.txt"),
            "bericht-über-alles.txt"
        );
    }

    #[test]
    fn sanitize_bounds_length() {
        let long = "x".repeat(500);
        assert_eq!(sanitize_filename(&long).chars().count(), MAX_FILENAME_LEN);
    }

    #[test]
    fn content_disposition_quotes_and_encodes() {
        let value = content_disposition("überraschung.pdf", false);
        assert!(value.starts_with("attachment; "));
        assert!(value.contains("filename=\"_berraschung.pdf\""));
        assert!(value.contains("filename*=UTF-8''%C3%BCberraschung.pdf"));
    }

    #[test]
    fn content_disposition_cannot_break_out_of_the_quoted_string() {
        let value = content_disposition("evil\";x=\"y.txt", false);
        // Everything after the first quote must have been neutralised.
        assert_eq!(value.matches('"').count(), 2);
    }

    #[test]
    fn content_disposition_supports_inline() {
        assert!(content_disposition("clip.mp4", true).starts_with("inline; "));
    }

    #[test]
    fn relative_names_use_forward_slashes_and_stay_inside_the_root() {
        let root = Path::new("/home/u/photos");
        let file = Path::new("/home/u/photos/2026/may/pic.jpg");
        assert_eq!(relative_display_name(root, file), "photos/2026/may/pic.jpg");
    }

    #[test]
    fn relative_names_drop_non_normal_components() {
        let root = Path::new("/home/u/photos");
        let file = Path::new("/home/u/photos/../secrets/key.pem");
        // `..` is discarded rather than being allowed to climb out.
        assert_eq!(relative_display_name(root, file), "photos/secrets/key.pem");
    }

    #[test]
    fn mime_guessing_falls_back_to_octet_stream() {
        assert_eq!(guess_mime(Path::new("a.pdf")), "application/pdf");
        assert_eq!(guess_mime(Path::new("clip.mp4")), "video/mp4");
        assert_eq!(
            guess_mime(Path::new("file.unknown-ext")),
            "application/octet-stream"
        );
        assert_eq!(
            guess_mime(Path::new("noextension")),
            "application/octet-stream"
        );
    }

    #[test]
    fn inline_preview_is_limited_to_media_and_documents() {
        assert!(is_inline_previewable("video/mp4"));
        assert!(is_inline_previewable("application/pdf"));
        assert!(!is_inline_previewable("application/zip"));
        assert!(!is_inline_previewable("application/octet-stream"));
    }

    #[test]
    fn canonicalize_rejects_directories_and_missing_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(canonicalize_shared_file(dir.path()).is_err());
        assert!(canonicalize_shared_file(&dir.path().join("nope.txt")).is_err());

        let file = dir.path().join("ok.txt");
        std::fs::write(&file, b"hello").expect("write");
        let resolved = canonicalize_shared_file(&file).expect("canonicalize");
        assert!(resolved.is_absolute());
        assert!(resolved.is_file());
    }
}
