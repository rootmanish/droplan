//! QR rendering for the share URL.
//!
//! Emitted as SVG so it stays crisp at any size in the desktop window and
//! needs no image decoding on the frontend. Colours are left to CSS via
//! `currentColor`, so the code follows the app's light/dark theme.

use qrcode::render::svg;
use qrcode::{EcLevel, QrCode};

use crate::error::{Error, Result};

/// Minimum rendered size in pixels. The frontend scales it with CSS.
const MIN_DIMENSION: u32 = 240;

/// Render `data` as an SVG document.
///
/// Uses medium error correction: a share URL is short, and phone cameras
/// tolerate a surprising amount of glare and angle at this level.
pub fn render_svg(data: &str) -> Result<String> {
    if data.is_empty() {
        return Err(Error::QrGeneration);
    }

    let code = QrCode::with_error_correction_level(data.as_bytes(), EcLevel::M).map_err(|err| {
        tracing::warn!(target: "droplan", "QR encoding failed: {err}");
        Error::QrGeneration
    })?;

    let document = code
        .render()
        .min_dimensions(MIN_DIMENSION, MIN_DIMENSION)
        .quiet_zone(true)
        .dark_color(svg::Color("currentColor"))
        .light_color(svg::Color("transparent"))
        .build();

    // The renderer prefixes an XML declaration. Strip it so the result can be
    // inlined into the page as a plain SVG element.
    Ok(strip_xml_declaration(&document))
}

/// Drop a leading `<?xml … ?>` prologue, if present.
fn strip_xml_declaration(document: &str) -> String {
    let trimmed = document.trim_start();
    match trimmed.strip_prefix("<?xml") {
        Some(rest) => rest
            .find("?>")
            .map(|end| {
                trimmed[end + "<?xml".len() + "?>".len()..]
                    .trim_start()
                    .to_string()
            })
            .unwrap_or_else(|| trimmed.to_string()),
        None => trimmed.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_share_url_renders_to_svg() {
        let svg = render_svg("http://192.168.1.42:8080/s/q7Fm3Ks9abcdefghijklmn").expect("qr");
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("viewBox") || svg.contains("width"));
        assert!(
            svg.contains("currentColor"),
            "the code should follow the theme"
        );
        assert!(svg.len() > 200);
    }

    #[test]
    fn the_xml_prologue_is_stripped_so_the_svg_can_be_inlined() {
        assert_eq!(
            strip_xml_declaration("<?xml version=\"1.0\" standalone=\"yes\"?><svg a=\"1\"/>"),
            "<svg a=\"1\"/>"
        );
        assert_eq!(strip_xml_declaration("<svg/>"), "<svg/>");
        assert_eq!(strip_xml_declaration("  <svg/>"), "<svg/>");
    }

    #[test]
    fn empty_input_is_rejected_rather_than_producing_a_blank_code() {
        assert!(render_svg("").is_err());
    }

    #[test]
    fn long_urls_still_encode() {
        let long = format!("http://192.168.100.200:65535/s/{}", "A".repeat(64));
        assert!(render_svg(&long).is_ok());
    }

    #[test]
    fn different_urls_produce_different_codes() {
        let a = render_svg("http://192.168.1.42:8080/s/aaaa").expect("qr");
        let b = render_svg("http://192.168.1.42:8080/s/bbbb").expect("qr");
        assert_ne!(a, b);
    }

    #[test]
    fn the_output_contains_no_raw_user_text() {
        // The URL is encoded into modules, never written into the document.
        let svg = render_svg("http://192.168.1.42:8080/s/SECRETTOKEN").expect("qr");
        assert!(!svg.contains("SECRETTOKEN"));
    }
}
