use anyhow::{Context, Result};
use qrcode::render::unicode;
use qrcode::{EcLevel, QrCode};

/// Generate a QR code matching bank's exact settings
///
/// Settings extracted from JavaScript bundle:
/// - Error correction level: M (Medium)
/// - Margin: 0
/// - Version: Auto-selected based on input
#[cfg(not(tarpaulin_include))]
pub fn generate_qr_code(data: &str) -> Result<QrCode> {
    // Use error correction level M to match BoursoBank's settings
    QrCode::with_error_correction_level(data, EcLevel::M).context("Failed to generate QR code")
}

/// Render QR code as a Unicode string for terminal display
///
/// Uses Unicode block characters to display the QR code in the terminal.
/// Each "pixel" is represented using block characters for a compact display.
#[cfg(not(tarpaulin_include))]
pub fn render_to_terminal(qr: &QrCode) -> String {
    qr.render::<unicode::Dense1x2>()
        .dark_color(unicode::Dense1x2::Dark)
        .light_color(unicode::Dense1x2::Light)
        .build()
}
