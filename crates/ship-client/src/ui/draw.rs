use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Clear, Paragraph, Wrap},
};
use std::{env, path::Path};

use ship_core::{
    model::{NodeId, Pane, ProcessState, Tab},
    screen::{Cursor, CursorShape, Size},
    tree,
};

use super::{
    Local, Overlay,
    observer::{Observer, Selected},
    panes,
    sidebar::Sidebar,
};
use crate::keymap::{Hint, ModeName};

/// What the status line shows besides the selection.
pub(super) struct Status<'a> {
    /// The active mode, outside normal.
    pub mode: Option<&'a ModeName>,
    /// The last error, or "not available yet".
    pub message: Option<&'a str>,
    /// An update's one-time note.
    pub note: Option<&'a str>,
    /// The mode's hints, in the file's order.
    pub hints: Vec<Hint<'a>>,
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
    Size::from(tab)
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
            local.updates.row(),
            &local.attention,
            &local.agents.rows,
            local.mouse.gap(),
            match &local.overlay {
                Some(Overlay::Rename(rename)) => match rename.target() {
                    NodeId::Tab(id) => Some(id),
                    NodeId::Pane(_) => None,
                },
                _ => None,
            },
        );
    }
    let scene = local.scene(frame.area());
    let observer = &local.observer;
    let status = super::status(&local.keys, local.message.as_deref(), local.note.as_deref());
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
    if let Some(Overlay::Rename(rename)) = &local.overlay {
        if main.is_empty() {
            return None;
        }
        let row = Rect {
            y: main.bottom() - 1,
            height: 1,
            ..main
        };
        let placeholder = placeholder(observer, rename.target());
        let at = rename.draw(frame, row, placeholder.as_deref());
        frame.set_cursor_position(at);
        return Some(Cursor {
            x: at.x,
            y: at.y,
            shape: CursorShape::Default,
            blinking: false,
        });
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
    match &mut local.overlay {
        Some(Overlay::Menu(menu)) => {
            menu.draw(frame);
            return None;
        }
        Some(Overlay::Updates(panel)) => {
            if let Some(replica) = &local.observer.replica {
                panel.draw(frame, frame.area(), &local.updates, &replica.tabs);
            }
            return None;
        }
        _ => {}
    }
    let (at, cursor) = cursor?;
    frame.set_cursor_position(at);
    Some(cursor)
}

/// "connecting" while the observer has no record; with nothing selected,
/// `3 tabs · alt-down to open one` or `no tabs · ship tab create`; with a tab
/// and no pane, `no pane · alt-| to make one`.
fn hint(observer: &Observer, selected: Option<&Selected>) -> String {
    let (Some(replica), Some(_)) = (&observer.replica, observer.record()) else {
        return "connecting".into();
    };
    match (selected, replica.tabs.len()) {
        (Some(_), _) => "no pane · alt-| to make one".into(),
        (None, 0) => "no tabs · ship tab create".into(),
        (None, 1) => "1 tab · alt-down to open one".into(),
        (None, count) => format!("{count} tabs · alt-down to open one"),
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
    if let Some(ProcessState::Exited(status)) = selected
        .and_then(|selected| selected.pane)
        .map(|pane| &pane.process)
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
    if let Some(note) = status.note {
        spans.push(Span::from(format!(" {note}")).cyan());
    }
    // Herdr's panel background, a step up from the terminal's.
    let mut bar = Line::from(spans).bg(Color::Black);
    let mut hints: Vec<_> = status
        .hints
        .iter()
        .map(|hint| {
            Line::from(vec![
                Span::from(format!("  {}", hint.chords.join("/")))
                    .cyan()
                    .bold(),
                Span::from(format!(" {}", hint.label)).gray(),
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

/// The pane's name, else its automatic label.
pub(super) fn pane_label(pane: &Pane) -> &str {
    pane.name
        .as_deref()
        .unwrap_or_else(|| pane_auto_label(pane))
}

/// The program's title, else its command's file name.
pub(super) fn pane_auto_label(pane: &Pane) -> &str {
    pane.title
        .as_deref()
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
        .map_or_else(|| tab_auto_label(tab, position), str::to_owned)
}

/// The first pane's directory or label, else its 1-based sibling position.
pub(super) fn tab_auto_label(tab: &Tab, position: usize) -> String {
    tab.panes()
        .next()
        .map(|pane| {
            pane.dir
                .as_deref()
                .map_or_else(|| pane_label(pane).to_owned(), last_component)
        })
        .unwrap_or_else(|| (position + 1).to_string())
}

/// Look up the automatic label each frame so title and directory changes show.
fn placeholder(observer: &Observer, target: NodeId) -> Option<String> {
    let tabs = observer.tabs()?;
    match target {
        NodeId::Pane(id) => Some(pane_auto_label(tree::pane(tabs, id).ok()?).to_owned()),
        NodeId::Tab(id) => {
            let siblings = tree::siblings(tabs, id)?;
            Some(tab_auto_label(
                siblings.get(&id)?,
                siblings.get_index_of(&id)?,
            ))
        }
    }
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
