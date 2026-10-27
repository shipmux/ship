use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Clear, Paragraph, Wrap},
};
use std::{env, path::Path};

use ship_core::{
    model::{Pane, PaneStatus, Tab},
    screen::{Cursor, Size},
};

use super::{
    Local,
    observer::{Observer, Selected},
    panes,
    sidebar::Sidebar,
};
use crate::keymap::ModeName;

/// What the status line shows besides the selection.
pub(super) struct Status<'a> {
    /// The active mode, outside normal.
    pub mode: Option<&'a ModeName>,
    /// The last error, or "not available yet".
    pub message: Option<&'a str>,
    /// The mode's `chord label` hints, in the file's order.
    pub hints: Vec<&'a str>,
}

/// `area` split into the sidebar's columns and the pane area beside them,
/// for drawing, hit-testing and the size the server gives panes.
pub(super) fn split(area: Rect, sidebar_width: u16) -> (Rect, Rect) {
    let width = if sidebar_width == 0 {
        0
    } else {
        Sidebar::clamp(sidebar_width, area.width)
    };
    (
        Rect { width, ..area },
        Rect {
            x: area.x + width,
            width: area.width - width,
            ..area
        },
    )
}

/// The area a terminal of `cols` by `rows` leaves for the tab. The server
/// sizes panes to it. The status line overlays the tab, so it takes none.
pub(super) fn tab_area(cols: u16, rows: u16, sidebar_width: u16) -> Size {
    let (_, tab) = split(Rect::new(0, 0, cols, rows), sidebar_width);
    Size {
        cols: tab.width,
        rows: tab.height,
    }
}

/// The selected tab's panes beside the sidebar, or a hint when it has none
/// or nothing is selected. The status line, when it has something to say,
/// over the pane area's bottom rows.
pub(super) fn draw(frame: &mut Frame, local: &mut Local) -> Option<Cursor> {
    let (sidebar_area, main) = split(frame.area(), local.sidebar.width());
    if local.sidebar.shown {
        local.sidebar.draw(
            frame,
            sidebar_area,
            &local.observer,
            &local.attention,
            &local.agents.rows,
            local.mouse.gap(),
        );
    }
    let scene = local.scene(frame.area());
    let observer = &local.observer;
    let status = super::status(&local.keys, local.message.as_deref());
    let selected = observer.selected();
    let mut cursor = None;
    match selected.as_ref().and_then(|selected| {
        let geometry = selected.tab.geometry.as_ref()?;
        (!geometry.panes.is_empty()).then_some((selected, geometry))
    }) {
        Some(_) => {
            cursor = panes::draw(frame, &scene, local.mouse.held());
        }
        None => frame.render_widget(
            Paragraph::new(hint(observer, selected.as_ref()))
                .centered()
                .wrap(Wrap { trim: true }),
            Rect {
                y: main.y + main.height / 2,
                height: main.height - main.height / 2,
                ..main
            },
        ),
    }
    if let Some(mut lines) = status_line(selected.as_ref(), observer.connected, &status, main.width)
    {
        // The mode's row is last, so a short pane area loses hint rows above it.
        let height = lines.len().min(usize::from(main.height));
        if let Some(kept) = height.checked_sub(1) {
            lines.drain(kept..lines.len() - 1);
        }
        let overlay = Rect {
            y: main.bottom().saturating_sub(height as u16),
            height: height as u16,
            ..main
        };
        frame.render_widget(Clear, overlay);
        frame.render_widget(Paragraph::new(lines).bg(Color::Black), overlay);
        // A cursor under the overlay would sit on top of it.
        cursor = cursor.filter(|(at, _)| !overlay.contains(*at));
    }
    if let Some(menu) = &mut local.menu {
        menu.draw(frame);
        return None;
    }
    let (at, cursor) = cursor?;
    frame.set_cursor_position(at);
    Some(cursor)
}

/// "connecting" while the observer has no record; with nothing selected,
/// `3 tabs · alt-right to open one` or `no tabs · ship tab create`; with a tab
/// and no pane, `no pane: ship pane create --tab <tab-id>`.
fn hint(observer: &Observer, selected: Option<&Selected>) -> String {
    let (Some(replica), Some(_)) = (&observer.replica, observer.record()) else {
        return "connecting".into();
    };
    match (selected, replica.tabs.len()) {
        (Some(selected), _) => format!("no pane: ship pane create --tab {}", selected.tab.id),
        (None, 0) => "no tabs · ship tab create".into(),
        (None, 1) => "1 tab · alt-right to open one".into(),
        (None, count) => format!("{count} tabs · alt-right to open one"),
    }
}

/// Herdr's mode bar: the mode's name as a pill, then the selected pane's exit
/// status, the connection and the last message. Hints share that row when
/// they fit, else sit above it in columns, so the pill stays on the bottom row
/// either way. `None` when there is nothing to say.
fn status_line<'a>(
    selected: Option<&Selected>,
    connected: bool,
    status: &Status<'a>,
    width: u16,
) -> Option<Vec<Line<'a>>> {
    let pill = |text: String, color| Span::styled(text, Style::new().black().bg(color).bold());
    let mut spans = Vec::new();
    if let Some(mode) = status.mode {
        spans.push(pill(format!(" {} ", mode.0.to_uppercase()), Color::Cyan));
    }
    if let Some(PaneStatus::Exited(status)) = selected
        .and_then(|selected| selected.pane)
        .map(|pane| &pane.status)
    {
        let code = status.signal.clone().unwrap_or(status.code.to_string());
        spans.push(Span::from(format!(" exited ({code})")).yellow());
    }
    if !connected {
        spans.push(Span::from(" disconnected, reconnecting").red());
    }
    // A config error spans several lines; its first says where it is.
    if let Some(message) = status.message.and_then(|message| message.lines().next()) {
        spans.push(Span::from(format!(" {message}")).red());
    }
    // Herdr's panel background, a step up from the terminal's.
    let mut bar = Line::from(spans).bg(Color::Black);
    let mut hints: Vec<_> = status
        .hints
        .iter()
        .map(|hint| {
            let (chord, label) = hint.split_once(' ').unwrap_or((hint, ""));
            Line::from(vec![
                Span::from(format!("  {chord}")).cyan().bold(),
                Span::from(format!(" {label}")).gray(),
            ])
        })
        .collect();
    let width = usize::from(width);
    if bar.width() + hints.iter().map(Line::width).sum::<usize>() <= width {
        bar.spans
            .extend(hints.into_iter().flat_map(|hint| hint.spans));
        return (!bar.spans.is_empty()).then_some(vec![bar]);
    }
    // Over-wide hints are omitted whole, without hiding the hints after them.
    hints.retain(|hint| hint.width() <= width);
    let cell_width = hints.iter().map(Line::width).max().unwrap_or(1);
    let columns = (width / cell_width).max(1);
    let mut rows = Vec::new();
    for chunk in hints.chunks_mut(columns) {
        let mut spans = Vec::new();
        for hint in chunk {
            let padding = cell_width - hint.width();
            spans.append(&mut hint.spans);
            spans.push(Span::raw(" ".repeat(padding)));
        }
        rows.push(Line::from(spans).bg(Color::Black));
    }
    rows.push(bar);
    Some(rows)
}

/// The pane's name, else its program's title, else its command's file name.
pub(super) fn pane_label(pane: &Pane) -> &str {
    pane.name
        .as_deref()
        .or(pane.title.as_deref())
        .or_else(|| {
            let program = Path::new(pane.command.first()?).file_name()?;
            program.to_str()
        })
        .unwrap_or_default()
}

/// The tab's name, else its first pane's directory's last component, else
/// that pane's label, else its 1-based `position` among its siblings.
pub(super) fn tab_label(tab: &Tab, position: usize) -> String {
    tab.name
        .as_deref()
        .map(str::to_owned)
        .or_else(|| {
            tab.panes().next().map(|pane| {
                pane.dir
                    .as_deref()
                    .map_or_else(|| pane_label(pane).to_owned(), last_component)
            })
        })
        .unwrap_or_else(|| (position + 1).to_string())
}

/// `~` for the home directory, else the path's last component, else the path
/// itself, as for `/`.
fn last_component(dir: &str) -> String {
    let path = Path::new(dir);
    if env::home_dir().is_some_and(|home| home == path) {
        return "~".to_owned();
    }
    path.file_name()
        .map_or(dir, |name| name.to_str().unwrap_or(dir))
        .to_owned()
}
