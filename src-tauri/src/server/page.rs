//! The page other devices see.
//!
//! Rendered server-side from Rust so the LAN endpoint stays tiny and the
//! desktop bundle is never exposed. Downloads are plain `<a href>`: the page
//! works with JavaScript disabled, in a phone's in-app browser, and in text
//! browsers. The optional script only watches for list changes.

use crate::security::paths;
use crate::sharing::ShareItem;

/// Escape text for insertion into HTML element content or a quoted attribute.
pub fn escape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 16);
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Human-readable size using binary units, matching what the desktop shows.
pub fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64;
    let mut unit = 0usize;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

/// Short type label for the list ("PDF", "Video", "ZIP").
pub fn type_label(mime: &str, name: &str) -> String {
    if mime.starts_with("video/") {
        return "Video".to_string();
    }
    if mime.starts_with("image/") {
        return "Image".to_string();
    }
    if mime.starts_with("audio/") {
        return "Audio".to_string();
    }
    if mime.starts_with("text/") {
        return "Text".to_string();
    }
    match mime {
        "application/pdf" => return "PDF".to_string(),
        "application/zip" | "application/x-zip-compressed" => return "ZIP".to_string(),
        "application/x-tar" | "application/gzip" | "application/x-7z-compressed" => {
            return "Archive".to_string()
        }
        _ => {}
    }
    name.rsplit_once('.')
        .map(|(_, ext)| ext.to_ascii_uppercase())
        .filter(|ext| !ext.is_empty() && ext.len() <= 5)
        .unwrap_or_else(|| "File".to_string())
}

const STYLE: &str = r#"
:root{color-scheme:light dark;--bg:#f6f7f9;--panel:#ffffff;--fg:#16181d;--muted:#6b7280;--line:#e5e7eb;--accent:#1d4ed8;--accent-fg:#ffffff;--warn:#b45309}
@media (prefers-color-scheme:dark){:root{--bg:#0d0f13;--panel:#161a21;--fg:#e8eaed;--muted:#9aa3b2;--line:#252b35;--accent:#4f7cf7;--accent-fg:#0d0f13;--warn:#f0b429}}
*{box-sizing:border-box}
body{margin:0;padding:24px 16px 56px;background:var(--bg);color:var(--fg);font:16px/1.5 -apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,Ubuntu,Cantarell,"Helvetica Neue",Arial,sans-serif;-webkit-text-size-adjust:100%}
.wrap{max-width:720px;margin:0 auto}
header{margin-bottom:22px}
.brand{display:flex;align-items:center;gap:9px;margin-bottom:16px}
.brand svg{width:24px;height:24px;display:block;flex:none}
.brand span{font-weight:640;font-size:.98rem;letter-spacing:-.01em}
.brand .lan{color:var(--accent)}
.hero{margin-bottom:14px}
.hero svg{width:56px;height:56px;display:block}
h1 .lan{color:var(--accent)}
h1{font-size:1.35rem;margin:0 0 4px;font-weight:650;letter-spacing:-.01em}
.sub{color:var(--muted);font-size:.9rem;margin:0}
ul{list-style:none;margin:0;padding:0;display:flex;flex-direction:column;gap:10px}
li{background:var(--panel);border:1px solid var(--line);border-radius:12px;padding:14px 16px;display:flex;align-items:center;gap:14px;flex-wrap:wrap}
.meta{flex:1 1 240px;min-width:0}
.name{font-weight:560;word-break:break-word;margin:0 0 2px}
.facts{color:var(--muted);font-size:.82rem;display:flex;gap:8px;flex-wrap:wrap;align-items:center}
.tag{border:1px solid var(--line);border-radius:999px;padding:1px 8px;font-size:.72rem;letter-spacing:.02em;text-transform:uppercase;color:var(--muted)}
.actions{display:flex;gap:8px;align-items:center;margin-left:auto}
a.btn{display:inline-block;background:var(--accent);color:var(--accent-fg);text-decoration:none;font-size:.88rem;font-weight:560;padding:9px 16px;border-radius:9px;white-space:nowrap}
a.btn:hover{filter:brightness(1.08)}
a.ghost{background:transparent;color:var(--accent);border:1px solid var(--line)}
.empty,.note{background:var(--panel);border:1px solid var(--line);border-radius:12px;padding:28px 20px;text-align:center;color:var(--muted)}
.gone{color:var(--warn);font-size:.82rem}
footer{margin-top:26px;text-align:center;color:var(--muted);font-size:.78rem;line-height:1.7}
form{background:var(--panel);border:1px solid var(--line);border-radius:12px;padding:24px;display:flex;flex-direction:column;gap:14px;max-width:320px;margin:0 auto}
input[type=text]{font-size:1.5rem;letter-spacing:.34em;text-align:center;padding:12px;border-radius:9px;border:1px solid var(--line);background:var(--bg);color:var(--fg);width:100%}
button{background:var(--accent);color:var(--accent-fg);border:0;border-radius:9px;padding:12px;font-size:.95rem;font-weight:560;cursor:pointer}
.err{color:var(--warn);font-size:.85rem;text-align:center;margin:0}
@media (max-width:480px){.actions{margin-left:0;width:100%}a.btn{flex:1;text-align:center}}
"#;

/// The DropLAN mark, inlined so the page needs no second request and works
/// with no internet connection at all. Single-quoted attributes keep it valid
/// both as an element and as a `data:` URI.
const BRAND_MARK: &str = "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 32 32' width='32' height='32'><rect width='32' height='32' rx='7.1' fill='#2B4FC9'/><g fill='none' stroke='#fff' stroke-width='2.2' stroke-linecap='round' stroke-linejoin='round'><path d='M16 6.7V16.8'/><path d='M11.9 13.3 16 17.3 20.1 13.3'/></g><g stroke='#fff' stroke-width='0.9' stroke-linecap='round' stroke-opacity='.5'><path d='M16 23.2 8.2 19.4'/><path d='M16 23.2 23.8 19.4'/></g><circle cx='8.2' cy='19.4' r='1.45' fill='#fff'/><circle cx='23.8' cy='19.4' r='1.45' fill='#fff'/><circle cx='16' cy='23.2' r='1.95' fill='#5CE1FF'/></svg>";

/// Favicon for the browser tab, as the same mark in a `data:` URI.
const FAVICON: &str = "data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 32 32' width='32' height='32'><rect width='32' height='32' rx='7.1' fill='%232B4FC9'/><g fill='none' stroke='%23fff' stroke-width='2.2' stroke-linecap='round' stroke-linejoin='round'><path d='M16 6.7V16.8'/><path d='M11.9 13.3 16 17.3 20.1 13.3'/></g><g stroke='%23fff' stroke-width='0.9' stroke-linecap='round' stroke-opacity='.5'><path d='M16 23.2 8.2 19.4'/><path d='M16 23.2 23.8 19.4'/></g><circle cx='8.2' cy='19.4' r='1.45' fill='%23fff'/><circle cx='23.8' cy='19.4' r='1.45' fill='%23fff'/><circle cx='16' cy='23.2' r='1.95' fill='%235CE1FF'/></svg>";

/// Small brand row above the page heading. Deliberately secondary: the
/// receiving device cares first about whose files these are.
fn brand_row() -> String {
    format!(
        "<div class=\"brand\">{BRAND_MARK}<span>Drop<span class=\"lan\">LAN</span></span></div>"
    )
}

fn document(title: &str, body: &str, tail: &str) -> String {
    format!(
        "<!doctype html>\n<html lang=\"en\"><head><meta charset=\"utf-8\">\
<meta name=\"viewport\" content=\"width=device-width,initial-scale=1,viewport-fit=cover\">\
<meta name=\"robots\" content=\"noindex,nofollow\">\
<link rel=\"icon\" href=\"{FAVICON}\">\
<title>{title}</title><style>{STYLE}</style></head>\
<body><div class=\"wrap\">{body}</div>{tail}</body></html>",
        title = escape_html(title),
        body = body,
        tail = tail,
    )
}

/// Progressive enhancement only: notice when the desktop side adds or removes
/// a file and reload. Everything on the page already works without it.
fn live_reload_script(api_path: &str, signature: &str) -> String {
    format!(
        "<script>(function(){{var sig={sig};var url={url};\
function check(){{if(document.hidden)return;fetch(url,{{cache:'no-store'}})\
.then(function(r){{return r.ok?r.json():null}})\
.then(function(d){{if(d&&d.signature&&d.signature!==sig)location.reload()}})\
.catch(function(){{}})}}\
setInterval(check,7000);document.addEventListener('visibilitychange',check);}})();</script>",
        sig = serde_json::to_string(signature).unwrap_or_else(|_| "\"\"".into()),
        url = serde_json::to_string(api_path).unwrap_or_else(|_| "\"\"".into()),
    )
}

/// The share page: the list of files another device can download.
pub fn render_share_page(
    device_name: &str,
    base_path: &str,
    items: &[ShareItem],
    signature: &str,
) -> String {
    let available: Vec<&ShareItem> = items.iter().filter(|item| item.available).collect();
    let total_bytes: u64 = available.iter().map(|item| item.size).sum();

    let summary = match available.len() {
        0 => "Nothing shared yet".to_string(),
        1 => format!("1 file · {}", format_size(total_bytes)),
        count => format!("{count} files · {}", format_size(total_bytes)),
    };

    let list = if available.is_empty() {
        "<div class=\"empty\"><p>No files are being shared right now.</p>\
<p style=\"font-size:.85rem\">Drop files into DropLAN on the other computer and this page will update.</p></div>"
            .to_string()
    } else {
        let rows: String = available
            .iter()
            .map(|item| render_row(base_path, item))
            .collect();
        format!("<ul>{rows}</ul>")
    };

    let unavailable = items.len() - available.len();
    let unavailable_note = if unavailable > 0 {
        format!(
            "<p class=\"gone\">{unavailable} file{} became unavailable on the host computer.</p>",
            if unavailable == 1 { "" } else { "s" }
        )
    } else {
        String::new()
    };

    let body = format!(
        "<header>{brand}<h1>Files from {device}</h1><p class=\"sub\">{summary}</p></header>\
{list}{unavailable_note}\
<footer>Served directly over your local network by DropLAN.<br>Nothing here touches the internet.</footer>",
        brand = brand_row(),
        device = escape_html(device_name),
        summary = escape_html(&summary),
    );

    let script = live_reload_script(&format!("{base_path}/api/files"), signature);
    document(&format!("Files from {device_name}"), &body, &script)
}

fn render_row(base_path: &str, item: &ShareItem) -> String {
    let href = format!("{base_path}/files/{}", item.id);
    let label = type_label(&item.mime_type, &item.display_name);
    let preview = if paths::is_inline_previewable(&item.mime_type) {
        format!("<a class=\"btn ghost\" href=\"{href}\">Open</a>")
    } else {
        String::new()
    };

    format!(
        "<li><div class=\"meta\"><p class=\"name\">{name}</p>\
<div class=\"facts\"><span>{size}</span><span class=\"tag\">{label}</span></div></div>\
<div class=\"actions\">{preview}<a class=\"btn\" href=\"{href}?dl=1\" download>Download</a></div></li>",
        name = escape_html(&item.display_name),
        size = escape_html(&format_size(item.size)),
        label = escape_html(&label),
    )
}

/// The PIN gate, shown before the file list when a PIN is set.
pub fn render_pin_page(device_name: &str, base_path: &str, failed: bool) -> String {
    let error = if failed {
        "<p class=\"err\">That PIN did not match. Check the app on the other computer.</p>"
    } else {
        ""
    };
    let body = format!(
        "<header>{brand}<h1>Files from {device}</h1><p class=\"sub\">Enter the PIN shown in DropLAN to continue.</p></header>\
<form method=\"post\" action=\"{base}/unlock\">{error}\
<input type=\"text\" name=\"pin\" inputmode=\"numeric\" pattern=\"[0-9]*\" autocomplete=\"one-time-code\" \
maxlength=\"6\" aria-label=\"PIN\" autofocus required>\
<button type=\"submit\">Unlock</button></form>",
        brand = brand_row(),
        device = escape_html(device_name),
        base = escape_html(base_path),
    );
    document(&format!("Files from {device_name}"), &body, "")
}

/// Shown at `/`, where no session token is present. Reveals nothing.
pub fn render_root_page() -> String {
    let body = format!(
        "<header><div class=\"hero\">{BRAND_MARK}</div>\
<h1>Drop<span class=\"lan\">LAN</span></h1>\
<p class=\"sub\">This computer is sharing files on the local network.</p></header>\
<div class=\"note\"><p>Open the full link shown in the DropLAN window, or scan its QR code.</p>\
<p style=\"font-size:.85rem\">The address includes a one-time code, so this page on its own cannot list or serve anything.</p></div>"
    );
    document("DropLAN", &body, "")
}

/// Anything that is not a live session or a live file.
pub fn render_not_found_page() -> String {
    let body = format!("<header>{brand}<h1>Not available</h1><p class=\"sub\">This link is no longer valid.</p></header>\
<div class=\"note\"><p>The sharing session may have ended, the file may have been removed, \
or the link may be out of date.</p><p style=\"font-size:.85rem\">Ask for a fresh link from the DropLAN window.</p></div>",
        brand = brand_row());
    document("Not available", &body, "")
}

/// A file that was shared but is no longer readable on disk.
pub fn render_gone_page() -> String {
    let body = format!("<header>{brand}<h1>File unavailable</h1><p class=\"sub\">This file can no longer be accessed.</p></header>\
<div class=\"note\"><p>It was moved, renamed or deleted on the host computer after it was shared.</p></div>",
        brand = brand_row());
    document("File unavailable", &body, "")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn item(id: &str, name: &str, size: u64, mime: &str, available: bool) -> ShareItem {
        ShareItem {
            id: id.to_string(),
            display_name: name.to_string(),
            absolute_path: PathBuf::from("/tmp").join(name),
            mime_type: mime.to_string(),
            size,
            added_at: 0,
            available,
        }
    }

    #[test]
    fn sizes_are_formatted_the_way_people_read_them() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(13_002_342), "12.4 MB");
        assert_eq!(format_size(134_951_731), "128.7 MB");
        assert_eq!(format_size(5 * 1024 * 1024 * 1024), "5.0 GB");
        assert_eq!(format_size(50 * 1024_u64.pow(4)), "50.0 TB");
    }

    #[test]
    fn html_escaping_neutralises_markup_and_attributes() {
        assert_eq!(
            escape_html("<script>alert(1)</script>"),
            "&lt;script&gt;alert(1)&lt;/script&gt;"
        );
        assert_eq!(escape_html("a\"b'c&d"), "a&quot;b&#39;c&amp;d");
        assert_eq!(escape_html("plain name.pdf"), "plain name.pdf");
    }

    #[test]
    fn a_hostile_filename_cannot_inject_markup_into_the_page() {
        let hostile = item(
            "abc123",
            "<img src=x onerror=alert(1)>\" onmouseover=\"evil()",
            10,
            "application/octet-stream",
            true,
        );
        let html = render_share_page("Laptop", "/s/tok", &[hostile], "sig");

        // No live tag, and no way out of the quoted attribute it tried to close.
        assert!(!html.contains("<img"));
        assert!(!html.contains("onmouseover=\""));
        assert!(html.contains("&lt;img src=x onerror=alert(1)&gt;"));
        assert!(html.contains("&quot; onmouseover=&quot;evil()"));
    }

    #[test]
    fn the_share_page_lists_files_with_working_links() {
        let items = vec![
            item("f1", "report.pdf", 13_002_342, "application/pdf", true),
            item("f2", "demo.mp4", 134_951_731, "video/mp4", true),
        ];
        let html = render_share_page("MacBook-Pro", "/s/q7Fm3Ks9", &items, "sig");

        assert!(html.contains("Files from MacBook-Pro"));
        assert!(html.contains("2 files · 141.1 MB"));
        assert!(html.contains("href=\"/s/q7Fm3Ks9/files/f1?dl=1\""));
        assert!(html.contains("href=\"/s/q7Fm3Ks9/files/f2?dl=1\""));
        assert!(html.contains("report.pdf"));
        assert!(html.contains("12.4 MB"));
        assert!(html.contains(">PDF<"));
        assert!(html.contains(">Video<"));
        // Video is previewable, so it also gets an inline "Open" link.
        assert!(html.contains("href=\"/s/q7Fm3Ks9/files/f2\">Open</a>"));
    }

    #[test]
    fn downloads_do_not_depend_on_javascript() {
        let items = vec![item("f1", "a.zip", 100, "application/zip", true)];
        let html = render_share_page("Box", "/s/tok", &items, "sig");

        let without_script = html
            .split("<script>")
            .next()
            .expect("content before any script");
        assert!(without_script.contains("href=\"/s/tok/files/f1?dl=1\""));
        assert!(without_script.contains("Download</a>"));
    }

    #[test]
    fn unavailable_files_are_hidden_and_counted() {
        let items = vec![
            item("f1", "here.txt", 10, "text/plain", true),
            item("f2", "gone.txt", 10, "text/plain", false),
        ];
        let html = render_share_page("Box", "/s/tok", &items, "sig");

        assert!(html.contains("here.txt"));
        assert!(!html.contains("gone.txt"));
        assert!(html.contains("1 file became unavailable"));
        assert!(html.contains("1 file · 10 B"));
    }

    #[test]
    fn the_empty_state_is_friendly() {
        let html = render_share_page("Box", "/s/tok", &[], "sig");
        assert!(html.contains("Nothing shared yet"));
        assert!(html.contains("No files are being shared right now"));
        assert!(!html.contains("<ul>"));
    }

    #[test]
    fn the_root_page_reveals_no_session_and_no_files() {
        let html = render_root_page();
        assert!(html.contains("DropLAN"));
        assert!(!html.contains("/s/"));
        assert!(!html.contains("files/"));
    }

    #[test]
    fn every_page_is_mobile_ready_and_not_indexable() {
        for html in [
            render_share_page("Box", "/s/tok", &[], "sig"),
            render_pin_page("Box", "/s/tok", false),
            render_root_page(),
            render_not_found_page(),
            render_gone_page(),
        ] {
            assert!(html.starts_with("<!doctype html>"));
            assert!(html.contains("width=device-width"));
            assert!(html.contains("noindex"));
            assert!(html.contains("prefers-color-scheme"));
        }
    }

    #[test]
    fn the_pin_page_posts_back_to_the_session() {
        let html = render_pin_page("Box", "/s/tok", true);
        assert!(html.contains("action=\"/s/tok/unlock\""));
        assert!(html.contains("name=\"pin\""));
        assert!(html.contains("That PIN did not match"));
        assert!(!render_pin_page("Box", "/s/tok", false).contains("did not match"));
    }

    #[test]
    fn every_page_carries_the_droplan_mark_and_a_favicon() {
        for html in [
            render_share_page(
                "Box",
                "/s/tok",
                &[item("f1", "a.zip", 10, "application/zip", true)],
                "sig",
            ),
            render_share_page("Box", "/s/tok", &[], "sig"),
            render_pin_page("Box", "/s/tok", false),
            render_root_page(),
            render_not_found_page(),
            render_gone_page(),
        ] {
            assert!(html.contains("rel=\"icon\""), "every page needs a tab icon");
            assert!(
                html.contains("data:image/svg+xml,"),
                "the favicon is inlined, not fetched"
            );
            assert!(html.contains(">LAN<"), "the wordmark is present");
        }
    }

    #[test]
    fn the_favicon_uri_escapes_its_colour_literals() {
        // A raw '#' would end the data URI early and leave the tab iconless.
        assert!(!FAVICON.contains('#'));
        assert!(FAVICON.contains("%232B4FC9"));
        assert!(FAVICON.starts_with("data:image/svg+xml,"));
    }

    #[test]
    fn no_page_mentions_the_previous_product_name() {
        for html in [
            render_share_page("Box", "/s/tok", &[], "sig"),
            render_pin_page("Box", "/s/tok", true),
            render_root_page(),
            render_not_found_page(),
            render_gone_page(),
        ] {
            assert!(!html.to_ascii_lowercase().contains("lantern"));
        }
    }

    #[test]
    fn type_labels_are_short_and_recognisable() {
        assert_eq!(type_label("application/pdf", "a.pdf"), "PDF");
        assert_eq!(type_label("video/quicktime", "a.mov"), "Video");
        assert_eq!(type_label("image/png", "a.png"), "Image");
        assert_eq!(type_label("application/zip", "a.zip"), "ZIP");
        assert_eq!(type_label("application/octet-stream", "backup.dmg"), "DMG");
        assert_eq!(
            type_label("application/octet-stream", "noextension"),
            "File"
        );
        assert_eq!(
            type_label("application/octet-stream", "weird.verylongextension"),
            "File"
        );
    }

    #[test]
    fn the_live_reload_script_is_safely_quoted() {
        let html = render_share_page("Box", "/s/to\"k", &[], "si\"g");
        assert!(!html.contains("var sig=\"si\"g\""));
        assert!(html.contains(r#"var sig="si\"g""#));
    }
}
