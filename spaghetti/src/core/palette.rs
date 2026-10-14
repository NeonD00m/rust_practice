use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use crossterm::style::Color;

/// Okabe-Ito / Wong Colorblind-Safe Palette
/// - Cyan: ANSI 75 (RGB 86, 180, 233)
/// - Orange / Amber: ANSI 214 (RGB 230, 159, 0)
/// - Royal Blue: ANSI 33 (RGB 0, 114, 178)
/// - Reddish Purple / Magenta: ANSI 175 (RGB 204, 121, 167)
/// - Bluish Green: ANSI 72 (RGB 0, 158, 115)
/// - Yellow: ANSI 220 (RGB 240, 228, 66)
/// - Vermilion: ANSI 203 (RGB 213, 94, 0)
#[allow(dead_code)]
pub const PALETTE: &[Color] = &[
    Color::AnsiValue(75),  // Sky Blue / Cyan
    Color::AnsiValue(214), // Orange
    Color::AnsiValue(33),  // Blue
    Color::AnsiValue(175), // Magenta / Reddish purple
    Color::AnsiValue(72),  // Bluish green
    Color::AnsiValue(220), // Yellow
    Color::AnsiValue(203), // Vermilion
];

#[allow(dead_code)]
pub fn get_hash_color(key: &str) -> Color {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    let hash = hasher.finish() as usize;
    PALETTE[hash % PALETTE.len()]
}

#[allow(dead_code)]
pub fn language_color(lang_name: &str) -> Color {
    match lang_name.to_ascii_lowercase().as_str() {
        "rust" => Color::AnsiValue(214),       // Orange
        "c" | "c++" | "cpp" => Color::AnsiValue(75), // Cyan
        "python" => Color::AnsiValue(220),     // Yellow
        "javascript" | "typescript" => Color::AnsiValue(33), // Blue
        "lua" | "luau" => Color::AnsiValue(175), // Magenta
        _ => Color::AnsiValue(72),             // Bluish green
    }
}
