use std::path::Path;

use gpui::SharedString;
use mtp_rs::DateTime;
use rust_i18n::t;

pub fn format_size(bytes: u64) -> SharedString {
    if bytes == 0 {
        return "0 B".into();
    }
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B").into()
    } else {
        format!("{value:.1} {}", UNITS[unit]).into()
    }
}

pub fn format_datetime(dt: Option<DateTime>) -> SharedString {
    match dt {
        None => "—".into(),
        Some(d) => format!(
            "{:04}-{:02}-{:02} {:02}:{:02}",
            d.year, d.month, d.day, d.hour, d.minute
        )
        .into(),
    }
}

pub fn format_kind(filename: &str, is_folder: bool) -> SharedString {
    if is_folder {
        return t!("kind.folder").to_string().into();
    }
    match Path::new(filename).extension().and_then(|e| e.to_str()) {
        Some(ext) => t!("kind.file_with_ext", ext = ext.to_uppercase())
            .to_string()
            .into(),
        None => t!("kind.file").to_string().into(),
    }
}
