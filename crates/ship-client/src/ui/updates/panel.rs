//! The updates overlay (FR-009 to FR-013).

use std::time::{Duration, SystemTime};

use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Position, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Paragraph, Wrap},
};
use semver::Version;
use ship_core::tree::Tabs;

use super::{Progress, Tracker};
use crate::{
    keymap::{Action, ClientAction, ClientUpdates},
    ui::menu::Picked,
    updates::{Method, RESTART, Release, Sync, core_version, installed},
};

const WIDTH: u16 = 72;
/// Where values start after `client`, `server` and `latest`.
const INDENT: u16 = 9;
const CHECK: &str = "[check]";
const UPDATE: &str = "[update]";
const CHECKS_OFF: &str = "update checks are off";
/// Lines the wheel moves the notes.
const WHEEL: usize = 3;

#[derive(Default)]
pub(in crate::ui) struct Panel {
    /// The first notes line shown. Scrolling moves by line, so a draw wraps
    /// only a page of lines.
    scroll: usize,
    notes: Option<Notes>,
    /// Where the last draw put the panel and its buttons; a disabled button
    /// has none.
    area: Rect,
    check: Option<Rect>,
    update: Option<Rect>,
    /// The notes area's height at the last draw, for paging.
    page: u16,
}

/// The notes parsed for one check's result, kept until the next.
struct Notes {
    checked_at: Option<SystemTime>,
    lines: Vec<Line<'static>>,
}

impl Panel {
    /// Esc and clicks outside close; the buttons, or `c` and `u`, pick their
    /// action and keep the panel open; keys and the wheel scroll the notes.
    pub fn handle(&mut self, event: &Event) -> Picked {
        let pick = |action| Picked::Stay(Action::Client(ClientAction::Updates(action)));
        let page = usize::from(self.page.max(1));
        match event {
            Event::Key(key) if key.kind != KeyEventKind::Release => {
                if key.modifiers != KeyModifiers::NONE {
                    return Picked::Open;
                }
                self.scroll = match key.code {
                    KeyCode::Esc => return Picked::Close,
                    KeyCode::Char('c') if self.check.is_some() => {
                        return pick(ClientUpdates::Check {});
                    }
                    KeyCode::Char('u') if self.update.is_some() => {
                        return pick(ClientUpdates::Update {});
                    }
                    KeyCode::Up | KeyCode::Char('k') => self.scroll.saturating_sub(1),
                    KeyCode::Down | KeyCode::Char('j') => self.scroll.saturating_add(1),
                    KeyCode::PageUp => self.scroll.saturating_sub(page),
                    KeyCode::PageDown | KeyCode::Char(' ') => self.scroll.saturating_add(page),
                    KeyCode::Home => 0,
                    KeyCode::End => usize::MAX,
                    _ => self.scroll,
                };
            }
            Event::Mouse(mouse) => {
                let at = Position::new(mouse.column, mouse.row);
                let on = |button: Option<Rect>| button.is_some_and(|button| button.contains(at));
                self.scroll = match mouse.kind {
                    MouseEventKind::Down(_) if !self.area.contains(at) => return Picked::Close,
                    MouseEventKind::Down(MouseButton::Left) if on(self.check) => {
                        return pick(ClientUpdates::Check {});
                    }
                    MouseEventKind::Down(MouseButton::Left) if on(self.update) => {
                        return pick(ClientUpdates::Update {});
                    }
                    MouseEventKind::ScrollUp => self.scroll.saturating_sub(WHEEL),
                    MouseEventKind::ScrollDown => self.scroll.saturating_add(WHEEL),
                    _ => self.scroll,
                };
            }
            _ => {}
        }
        Picked::Open
    }

    /// Versions, status and buttons from the tracker above its release notes,
    /// centered on `screen` with a row above and below when there's room.
    pub fn draw(&mut self, frame: &mut Frame, screen: Rect, tracker: &Tracker, tabs: &Tabs) {
        let installed = installed();
        let progress = tracker.progress(tabs);
        let updating = matches!(progress, Progress::Updating);
        let method = tracker.method();
        let available = tracker.available();

        let server = Line::from(vec![
            row("server", format!("v{}", tracker.server.version)).into(),
            if core_version(&tracker.server.version).as_ref() == Some(&installed) {
                Span::default()
            } else {
                Span::from(" · restart the server to match").cyan()
            },
        ]);
        let latest = match tracker.found.as_ref().map(|found| &found.latest) {
            None => "unknown".into(),
            Some(Some(latest)) => format!("v{latest}"),
            Some(None) => "no releases yet".into(),
        };
        let versions = vec![
            row("client", format!("v{installed}")).into(),
            server,
            row("latest", latest).into(),
        ];
        let version_rows = versions.len() as u16;

        let (status, style) = status(tracker, &progress, available);
        let mut guidance = match (&progress, &method) {
            (Progress::Installed(sync), _) => vec![sync.guidance(), RESTART],
            (_, Method::Unavailable) => vec!["update with your package manager"],
            _ if available.is_some() && !updating => {
                vec!["the server restarts afterwards to stay in sync"]
            }
            _ => vec![],
        };
        // A `[check]` with checks off shows its result, so say it here.
        if !tracker.settings.check && status != CHECKS_OFF {
            guidance.push(CHECKS_OFF);
        }
        let mut message = vec![Line::from(status).style(style)];
        message.extend(
            guidance
                .into_iter()
                .map(|text| Line::from(text).dark_gray()),
        );
        let message = Paragraph::new(message).wrap(Wrap { trim: false });

        let checked_at = tracker.found.as_ref().map(|found| found.checked_at);
        let notes = match &mut self.notes {
            Some(notes) if notes.checked_at == checked_at => notes,
            slot => slot.insert(Notes {
                checked_at,
                lines: notes(tracker, &installed, available),
            }),
        };
        let lines = &notes.lines;

        // Inside the borders and a one-cell margin.
        let width = screen.width.min(WIDTH);
        let text = width.saturating_sub(4);
        let message_rows = message.line_count(text.saturating_sub(INDENT)) as u16;
        // Borders, versions, message, the `[update]` row and the rule.
        let fixed = 2 + version_rows + message_rows + 2;
        let height = screen
            .height
            .saturating_sub(2)
            .max(screen.height.min(fixed + 1));
        self.area = screen.centered(Constraint::Length(width), Constraint::Length(height));

        let block = Block::new()
            .borders(Borders::ALL)
            .border_style(Style::new().cyan())
            .padding(Padding::horizontal(1))
            .title(" updates ")
            .title(Line::from(" esc ").right_aligned());
        let inner = block.inner(self.area);
        frame.render_widget(Clear, self.area);
        frame.render_widget(block, self.area);
        let [top, message_area, update_row, rule, notes_area] = Layout::vertical([
            Constraint::Length(version_rows),
            Constraint::Length(message_rows),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .areas(inner);

        frame.render_widget(Paragraph::new(versions), top);
        let latest_row = Rect {
            y: top.y + 2,
            height: 1,
            ..top
        }
        .intersection(top);
        let checking = tracker.checking.is_some();
        let check = button(frame, latest_row, CHECK, !checking);
        let checked = match checked_at {
            Some(at) => format!("checked {} ", ago(at)),
            None => "never checked ".into(),
        };
        frame.render_widget(
            Line::from(checked).dark_gray().right_aligned(),
            Rect {
                width: check.x.saturating_sub(latest_row.x),
                ..latest_row
            },
        );
        self.check = (!checking).then_some(check);
        frame.render_widget(
            message,
            Rect {
                x: message_area.x + INDENT.min(message_area.width),
                width: message_area.width.saturating_sub(INDENT),
                ..message_area
            },
        );
        let enabled = method != Method::Unavailable && !updating;
        let update = button(frame, update_row, UPDATE, enabled);
        self.update = enabled.then_some(update);
        frame.render_widget(
            Block::new()
                .borders(Borders::TOP)
                .border_style(Style::new().cyan()),
            // Out to the borders, past the margin.
            Rect {
                x: self.area.x + 1,
                width: self.area.width.saturating_sub(2),
                ..rule
            }
            .intersection(self.area),
        );

        // Every line takes a row or more, so a page of lines fills the area.
        self.page = notes_area.height;
        self.scroll = self.scroll.min(lines.len().saturating_sub(1));
        let shown = lines.iter().skip(self.scroll).take(usize::from(self.page));
        frame.render_widget(
            Paragraph::new(shown.cloned().collect::<Vec<_>>()).wrap(Wrap { trim: false }),
            notes_area,
        );
    }
}

/// The one status FR-011 names, in its style.
fn status(tracker: &Tracker, progress: &Progress, available: Option<&Version>) -> (String, Style) {
    let accent = Style::new().cyan();
    let failed = Style::new().red();
    match progress {
        Progress::Updating => return (r#"updating in tab "update"…"#.into(), accent),
        Progress::Installed(Sync::Compatible(probed) | Sync::Restart(probed)) => {
            return (format!("installed v{}", probed.version), accent);
        }
        Progress::Installed(Sync::Unknown) => return ("update finished".into(), accent),
        Progress::Failed => return (r#"update failed · see tab "update""#.into(), failed),
        Progress::Idle => {}
    }
    if tracker.checking.is_some() {
        return ("checking…".into(), Style::new());
    }
    if let Some(why) = &tracker.check_failed {
        return (format!("check failed: {why}"), failed);
    }
    if let Some(latest) = available {
        return (format!("v{latest} is available"), accent);
    }
    if !tracker.settings.check {
        return (CHECKS_OFF.into(), Style::new());
    }
    let up_to_date = tracker
        .found
        .as_ref()
        .is_some_and(|found| found.latest.is_some());
    let text = if up_to_date { "up to date" } else { "" };
    (text.into(), Style::new())
}

/// `client   v0.1.0`: the label, then the value at `INDENT`.
fn row(label: &str, value: String) -> String {
    format!("{label:<8} {value}")
}

/// Draws `label` at the right end of `row` and returns where it went.
fn button(frame: &mut Frame, row: Rect, label: &'static str, enabled: bool) -> Rect {
    let width = label.len() as u16;
    let at = Rect {
        x: row.right().saturating_sub(width),
        width,
        ..row
    }
    .intersection(row);
    let span = Span::from(label);
    let span = if enabled {
        span.cyan().bold()
    } else {
        span.dark_gray()
    };
    frame.render_widget(span, at);
    at
}

/// Every release newer than the client up to the latest, newest first, or
/// the client's own release when up to date (FR-010).
fn notes(
    tracker: &Tracker,
    installed: &Version,
    available: Option<&Version>,
) -> Vec<Line<'static>> {
    let Some(found) = &tracker.found else {
        return Vec::new();
    };
    let releases: Vec<&Release> = match available {
        Some(latest) => found
            .newer(installed)
            .filter(|release| &release.version <= latest)
            .collect(),
        None => found.release(installed).into_iter().collect(),
    };
    if releases.is_empty() {
        return vec![Line::from("no release notes").dark_gray()];
    }
    let mut lines = Vec::new();
    for release in releases {
        if !lines.is_empty() {
            lines.push(Line::default());
        }
        let mut title = format!("v{}", release.version);
        if let Some(date) = release.published.as_deref().and_then(|at| at.get(..10)) {
            title.push_str(" · ");
            title.push_str(date);
        }
        lines.push(Line::from(title).bold());
        lines.push(Line::from(release.url.clone()).dark_gray());
        lines.extend(
            tui_markdown::from_str(&release.notes)
                .lines
                .into_iter()
                .map(owned),
        );
    }
    lines
}

/// `line` with its text copied out of the release it was parsed from.
fn owned(line: Line) -> Line<'static> {
    Line {
        spans: line
            .spans
            .into_iter()
            .map(|span| Span::styled(span.content.into_owned(), span.style))
            .collect(),
        style: line.style,
        alignment: line.alignment,
    }
}

/// `2h ago`; a check stamped in the future reads as `just now`.
fn ago(at: SystemTime) -> String {
    let elapsed = at.elapsed().unwrap_or_default();
    let unit = |secs: u64, unit| format!("{}{unit} ago", elapsed.as_secs() / secs);
    match elapsed {
        elapsed if elapsed < Duration::from_secs(60) => "just now".into(),
        elapsed if elapsed < Duration::from_secs(60 * 60) => unit(60, "m"),
        elapsed if elapsed < Duration::from_secs(24 * 60 * 60) => unit(60 * 60, "h"),
        _ => unit(24 * 60 * 60, "d"),
    }
}
