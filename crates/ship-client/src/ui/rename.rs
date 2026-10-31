//! One rename field on the mode bar, with line editing owned by tui-input.

use crossterm::event::{Event, KeyCode, KeyEventKind, MouseEventKind};
use ratatui::{
    Frame,
    layout::{Position, Rect},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Clear, Paragraph},
};
use ship_core::{
    command::{
        Command, PaneTarget, TabTarget,
        pane::{self, PaneCommand},
        tab::{self, TabCommand},
    },
    id::IdOf,
    model::{Pane, Tab},
};
use tui_input::{Input, InputRequest, backend::crossterm::EventHandler};

use super::menu::Picked;
use crate::keymap::Action;

/// Fixed when the field opens, independent of later selection changes.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Target {
    Tab(IdOf<Tab>),
    Pane(IdOf<Pane>),
}

pub(super) struct Rename {
    target: Target,
    input: Input,
}

impl Rename {
    pub fn open(target: Target, stored: Option<&str>) -> Self {
        Self {
            target,
            input: Input::from(stored.unwrap_or_default()),
        }
    }

    pub fn target(&self) -> Target {
        self.target
    }

    /// Capture everything; only Enter produces an action.
    pub fn handle(&mut self, event: &Event) -> Picked {
        match event {
            Event::Key(key) if key.kind != KeyEventKind::Release => match key.code {
                KeyCode::Enter => return Picked::Run(set(self.target, name(self.input.value()))),
                KeyCode::Esc => return Picked::Close,
                _ => {
                    self.input.handle_event(event);
                }
            },
            Event::Mouse(mouse) if matches!(mouse.kind, MouseEventKind::Down(_)) => {
                return Picked::Close;
            }
            Event::Paste(text) => {
                for c in text.replace("\r\n", " ").replace(['\r', '\n'], " ").chars() {
                    self.input.handle(InputRequest::InsertChar(c));
                }
            }
            _ => {}
        }
        Picked::Open
    }

    /// Draw the pill and a horizontally scrolled field, leaving room for its cursor.
    pub fn draw(&self, frame: &mut Frame, row: Rect, placeholder: Option<&str>) -> Position {
        let pill = Line::from(match self.target {
            Target::Tab(_) => " RENAME TAB ",
            Target::Pane(_) => " RENAME PANE ",
        })
        .style(Style::new().black().bg(Color::Cyan).bold());
        let width = pill.width().min(usize::from(row.width.saturating_sub(1))) as u16;
        let field = Rect {
            x: row.x + width,
            width: row.width - width,
            ..row
        };
        frame.render_widget(Clear, row);
        frame.render_widget(Paragraph::new(pill), Rect { width, ..row });
        let scroll = self
            .input
            .visual_scroll(usize::from(field.width.saturating_sub(1)));
        let text = if self.input.value().is_empty() {
            Line::from(placeholder.unwrap_or_default()).dim()
        } else {
            Line::from(self.input.value())
        };
        frame.render_widget(
            Paragraph::new(text)
                .style(Style::new().fg(Color::Reset).bg(Color::Black))
                .scroll((0, scroll.min(usize::from(u16::MAX)) as u16)),
            field,
        );
        Position::new(
            field.x + (self.input.visual_cursor() - scroll) as u16,
            row.y,
        )
    }
}

fn name(text: &str) -> Option<String> {
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_owned())
}

/// Reuse the CLI's name updates, with an explicit target and no other fields set.
fn set(target: Target, name: Option<String>) -> Action {
    Action::Server(match target {
        Target::Tab(id) => Command::Tab(TabCommand::Set(tab::Set {
            tab: TabTarget {
                id: Some(id),
                pane: None,
            },
            name: Some(name),
            ..Default::default()
        })),
        Target::Pane(id) => Command::Pane(PaneCommand::Set(pane::Set {
            pane: PaneTarget { id: Some(id) },
            name: Some(name),
            ..Default::default()
        })),
    })
}
