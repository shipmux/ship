use std::fmt;

use ratatui_core::{
    layout::Rect,
    style::{Color as TuiColor, Modifier},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A pane's visible grid at one moment. Full snapshot; zstd on the stream
/// supplies the savings between frames.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct Screen {
    pub size: Size,
    /// `size.rows` rows, top to bottom.
    pub rows: Vec<Row>,
    /// Absent when the cursor is hidden.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<Cursor>,
    /// Distance above the bottom, absent at the bottom.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scrolled: Option<Scrolled>,
    /// Whether the program has asked for mouse input.
    #[serde(default, skip_serializing_if = "is_false")]
    pub mouse: bool,
}

pub(crate) fn is_false(value: &bool) -> bool {
    !value
}

/// Rows above the bottom, out of the history's rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct Scrolled {
    pub offset: u32,
    pub history: u32,
}

/// `[offset/history]`, the scroll badge.
impl fmt::Display for Scrolled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}/{}]", self.offset, self.history)
    }
}

/// One row of cells as text plus style runs. Cells past `text` are blank,
/// with the default style; an empty field is omitted on the wire.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct Row {
    /// One char per cell, through the last cell that isn't a default blank. A
    /// wide character's trailing cell is its own blank.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub text: String,
    /// Cell styles from the left; cells past the last run have the default style.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub runs: Vec<Run>,
    /// Cells whose grapheme isn't one char, with a space in `text`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub multi: Vec<Multi>,
}

/// Adjacent cells sharing a style. Default-valued fields are omitted.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct Run {
    pub cells: u16,
    #[serde(default, skip_serializing_if = "Color::is_default")]
    pub fg: Color,
    #[serde(default, skip_serializing_if = "Color::is_default")]
    pub bg: Color,
    #[serde(default, skip_serializing_if = "Color::is_default")]
    pub underline_color: Color,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attrs: Vec<Attr>,
}

impl Run {
    pub fn is_default(&self) -> bool {
        self.fg.is_default()
            && self.bg.is_default()
            && self.underline_color.is_default()
            && self.attrs.is_empty()
    }
}

/// A cell's grapheme of more than one char, like a combining mark or a ZWJ
/// emoji piece.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct Multi {
    pub col: u16,
    pub symbol: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Color {
    /// The viewing terminal's own default color.
    #[default]
    Default,
    /// The viewing terminal's palette; 0 to 15 are the named ANSI colors.
    Indexed(u8),
    Rgb(u8, u8, u8),
}

impl Color {
    fn is_default(&self) -> bool {
        *self == Self::Default
    }
}

/// Named colors are palette entries 0 to 15.
impl From<TuiColor> for Color {
    fn from(color: TuiColor) -> Self {
        use TuiColor::*;
        let indexed = match color {
            Reset => return Self::Default,
            Rgb(r, g, b) => return Self::Rgb(r, g, b),
            Indexed(index) => index,
            Black => 0,
            Red => 1,
            Green => 2,
            Yellow => 3,
            Blue => 4,
            Magenta => 5,
            Cyan => 6,
            Gray => 7,
            DarkGray => 8,
            LightRed => 9,
            LightGreen => 10,
            LightYellow => 11,
            LightBlue => 12,
            LightMagenta => 13,
            LightCyan => 14,
            White => 15,
        };
        Self::Indexed(indexed)
    }
}

impl From<Color> for TuiColor {
    fn from(color: Color) -> Self {
        match color {
            Color::Default => Self::Reset,
            Color::Indexed(index) => Self::Indexed(index),
            Color::Rgb(r, g, b) => Self::Rgb(r, g, b),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Attr {
    Bold,
    Dim,
    Italic,
    Underlined,
    SlowBlink,
    RapidBlink,
    Reversed,
    Hidden,
    CrossedOut,
}

impl Attr {
    /// One table for both directions.
    const MODIFIERS: [(Attr, Modifier); 9] = [
        (Attr::Bold, Modifier::BOLD),
        (Attr::Dim, Modifier::DIM),
        (Attr::Italic, Modifier::ITALIC),
        (Attr::Underlined, Modifier::UNDERLINED),
        (Attr::SlowBlink, Modifier::SLOW_BLINK),
        (Attr::RapidBlink, Modifier::RAPID_BLINK),
        (Attr::Reversed, Modifier::REVERSED),
        (Attr::Hidden, Modifier::HIDDEN),
        (Attr::CrossedOut, Modifier::CROSSED_OUT),
    ];

    pub fn from_modifier(modifier: Modifier) -> Vec<Attr> {
        Self::MODIFIERS
            .iter()
            .filter(|(_, flag)| modifier.contains(*flag))
            .map(|(attr, _)| *attr)
            .collect()
    }

    pub fn to_modifier(attrs: &[Attr]) -> Modifier {
        Self::MODIFIERS
            .iter()
            .filter(|(attr, _)| attrs.contains(attr))
            .fold(Modifier::empty(), |modifier, (_, flag)| modifier | *flag)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct Cursor {
    pub x: u16,
    pub y: u16,
    pub shape: CursorShape,
    pub blinking: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CursorShape {
    Block,
    Underline,
    Bar,
    /// The viewing terminal's own default shape and blinking.
    Default,
}

/// Terminal size in cells. Also the viewer size and tab size.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Size {
    pub cols: u16,
    pub rows: u16,
}

impl Size {
    pub const FALLBACK: Size = Size { cols: 80, rows: 24 };
}

impl From<Rect> for Size {
    fn from(rect: Rect) -> Self {
        Self {
            cols: rect.width,
            rows: rect.height,
        }
    }
}
