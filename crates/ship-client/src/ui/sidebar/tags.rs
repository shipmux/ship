//! Tags as the sidebar draws them, shared by the tab and agent sections.

use ratatui::{style::Style, text::Span};
use ship_core::{
    model::Tab,
    tag::{Tag, TagName},
};

/// The value in its colors; `None` for an empty value, so it disappears
/// with its separator like an empty agent token (FR-017). Colors go on
/// this span only (FR-018).
pub(super) fn span(tag: &Tag) -> Option<Span<'static>> {
    let value = tag.value.as_str();
    if value.is_empty() {
        return None;
    }
    let mut style = Style::new();
    if let Some(fg) = tag.fg {
        style = style.fg(fg.into());
    }
    if let Some(bg) = tag.bg {
        style = style.bg(bg.into());
    }
    Some(Span::styled(value.to_owned(), style))
}

/// The tab's own tag, else its first pane's (FR-015), the way the tab's
/// name falls back to its first pane. Display only: `get` and `watch` show
/// the tab's own.
pub(super) fn on_tab<'a>(tab: &'a Tab, name: &TagName) -> Option<&'a Tag> {
    tab.tags
        .get(name)
        .or_else(|| tab.panes().next()?.tags.get(name))
}
