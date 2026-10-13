//! Colors and text styles of the dark trading theme.

use cranpose::prelude::*;
use cranpose::text::{FontWeight, TextAlign, TextUnit};

pub const BACKGROUND: Color = Color::from_rgb_u8(11, 14, 20);
pub const PANEL: Color = Color::from_rgb_u8(17, 21, 28);
pub const RAISED: Color = Color::from_rgb_u8(26, 31, 40);
pub const DIVIDER: Color = Color::from_rgb_u8(32, 38, 48);
pub const TEXT: Color = Color::from_rgb_u8(230, 233, 239);
pub const MUTED: Color = Color::from_rgb_u8(132, 142, 158);
pub const FAINT: Color = Color::from_rgb_u8(84, 92, 106);
pub const ACCENT: Color = Color::from_rgb_u8(142, 124, 255);
pub const UP: Color = Color::from_rgb_u8(14, 203, 129);
pub const DOWN: Color = Color::from_rgb_u8(246, 70, 93);

/// The color of a change: green up, red down.
pub fn trend(up: bool) -> Color {
    if up { UP } else { DOWN }
}

pub fn style(size: f32, color: Color) -> TextStyle {
    TextStyle {
        span_style: SpanStyle {
            font_size: TextUnit::Sp(size),
            color: Some(color),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn bold(size: f32, color: Color) -> TextStyle {
    let mut style = style(size, color);
    style.span_style.font_weight = Some(FontWeight::SEMI_BOLD);
    style
}

/// A style whose text sits at the end of its box, for number columns.
pub fn end(size: f32, color: Color) -> TextStyle {
    let mut style = style(size, color);
    style.paragraph_style.text_align = TextAlign::End;
    style
}
