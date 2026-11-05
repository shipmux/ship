//! The sidebar's agents section.

use ratatui::{
    Frame,
    layout::{Position, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::Paragraph,
};
use ship_core::{
    id::IdOf,
    model::{NodeId, Pane},
    tree::Tabs,
};

use super::{Scroll, tags};
use crate::{
    keymap::Token,
    ui::{
        agents::{self, Agent, Display},
        attention::Attention,
        draw::pane_label,
    },
};

/// Drawn before each line, lining entries up with the `tabs` heading.
const MARGIN: &str = " ";
const ELLIPSIS: &str = "…";

/// Distinct by shape and color; tab rows use the same.
pub(super) fn icon(display: Display) -> Span<'static> {
    let shape = match display {
        Display::Blocked => "▲",
        Display::Error => "✖",
        Display::Unseen => "◆",
        Display::Working => "●",
        Display::Idle => "○",
    };
    Span::styled(shape, color(display))
}

/// Shared by the icon and the `state` token, as in Herdr.
fn color(display: Display) -> Color {
    match display {
        Display::Blocked => Color::Red,
        Display::Error => Color::Magenta,
        Display::Unseen => Color::Cyan,
        Display::Working => Color::Yellow,
        Display::Idle => Color::Green,
    }
}

#[derive(Default)]
pub(in crate::ui) struct Agents {
    /// The first drawn agent.
    offset: usize,
    /// Agent entries as last drawn, for hit-testing.
    drawn: Vec<Drawn>,
}

/// One agent's lines on screen.
struct Drawn {
    pane: IdOf<Pane>,
    area: Rect,
}

impl Agents {
    /// The agent whose lines contain `at`.
    pub fn hit(&self, at: Position) -> Option<IdOf<Pane>> {
        self.drawn
            .iter()
            .find(|drawn| drawn.area.contains(at))
            .map(|drawn| drawn.pane)
    }

    /// By whole entries; drawing stops the offset once the last entry shows.
    pub fn scroll(&mut self, scroll: Scroll) {
        self.offset = match scroll {
            Scroll::Up => self.offset.saturating_sub(1),
            Scroll::Down => self.offset.saturating_add(1),
        };
    }

    /// The entries of `agents::list`, or a `no agents` placeholder. The
    /// selected pane's entry gets Herdr's active band: full-bright text on
    /// dark gray.
    pub fn draw(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        tabs: &Tabs,
        selection: Option<NodeId>,
        attention: &Attention,
        rows: &[Vec<Token>],
    ) {
        self.drawn.clear();
        if area.is_empty() {
            return;
        }
        let width = area.width.saturating_sub(Span::raw(MARGIN).width() as u16);
        let entries: Vec<_> = agents::list(tabs, attention)
            .iter()
            .map(|agent| (agent.pane.id, row(agent, rows, width)))
            .collect();
        if entries.is_empty() {
            frame.render_widget(Paragraph::new(" no agents").dark_gray(), area);
            return;
        }
        // The furthest offset that still fills the section from the bottom.
        let mut last = entries.len();
        let mut height = 0;
        while let Some(previous) = last.checked_sub(1) {
            height += entries[previous].1.len();
            if height > usize::from(area.height) {
                break;
            }
            last = previous;
        }
        self.offset = self.offset.min(last);
        let mut y = area.y;
        for (pane, lines) in entries.into_iter().skip(self.offset) {
            if y >= area.bottom() {
                break;
            }
            let entry = Rect {
                y,
                height: (lines.len() as u16).min(area.bottom() - y),
                ..area
            };
            let style = if selection == Some(NodeId::Pane(pane)) {
                Style::new().fg(Color::Reset).on_dark_gray()
            } else {
                Style::new().gray()
            };
            frame.render_widget(Paragraph::new(lines).style(style), entry);
            self.drawn.push(Drawn { pane, area: entry });
            y = entry.bottom();
        }
    }
}

/// The entry's lines, leaving out tokens without a value and lines left
/// empty. Values too wide for `width` share it, as `shares` splits it.
fn row(agent: &Agent, rows: &[Vec<Token>], width: u16) -> Vec<Line<'static>> {
    rows.iter()
        .filter_map(|tokens| {
            let values: Vec<_> = tokens
                .iter()
                .filter_map(|token| Some((token, value(agent, token)?)))
                .collect();
            let shares = shares(&values, usize::from(width));
            let mut spans = vec![Span::raw(MARGIN)];
            // A value left no cells goes with its separator.
            let fitted = values
                .into_iter()
                .zip(shares)
                .filter(|(_, share)| *share > 0);
            for (index, ((token, span), share)) in fitted.enumerate() {
                if index > 0 {
                    spans.push(Span::raw(" "));
                }
                spans.push(fit(token, span, share));
            }
            (spans.len() > 1).then(|| Line::from(spans))
        })
        .collect()
}

/// Each value's width once the line fits in `width`. The icon keeps its
/// own, and the rest take turns a cell at a time, as in Herdr.
fn shares(values: &[(&Token, Span)], width: usize) -> Vec<usize> {
    let mut shares: Vec<usize> = values
        .iter()
        .map(|(token, span)| match token {
            Token::Icon => span.width(),
            _ => 0,
        })
        .collect();
    let separators = values.len().saturating_sub(1);
    let mut room = width.saturating_sub(shares.iter().sum::<usize>() + separators);
    while room > 0 {
        let before = room;
        for (share, (_, span)) in shares.iter_mut().zip(values) {
            if room > 0 && *share < span.width() {
                *share += 1;
                room -= 1;
            }
        }
        if room == before {
            break;
        }
    }
    shares
}

/// The value cut to its share, keeping its style so a cut tag keeps its
/// colors. A path loses its start, everything else its end.
fn fit(token: &Token, span: Span<'static>, share: usize) -> Span<'static> {
    let content = match token {
        Token::Tab => shorten(&span.content, share),
        _ => truncate_end(&span.content, share),
    };
    Span::styled(content, span.style)
}

fn value(agent: &Agent, token: &Token) -> Option<Span<'static>> {
    let text = |text: &str| (!text.is_empty()).then(|| Span::from(text.to_owned()));
    match token {
        Token::Icon => Some(icon(agent.display)),
        Token::App => text(agent.name()),
        Token::State => text(agent.display.into()).map(|span| span.fg(color(agent.display))),
        Token::Tab => text(&agent.tab()),
        Token::Pane => text(pane_label(agent.pane)),
        Token::Title => text(agent.root.title.as_deref()?),
        Token::Msg => text(agent.root.msg.as_deref()?),
        Token::Tag(name) => tags::span(agent.pane.tags.get(name)?),
    }
}

/// `path` cut from the left behind an ellipsis to fit `room` cells.
fn shorten(path: &str, room: usize) -> String {
    if Span::raw(path).width() <= room {
        return path.to_owned();
    }
    let room = room.saturating_sub(Span::raw(ELLIPSIS).width());
    let start = path
        .char_indices()
        .map(|(index, _)| index)
        .find(|index| Span::raw(&path[*index..]).width() <= room)
        .unwrap_or(path.len());
    format!("{ELLIPSIS}{}", &path[start..])
}

/// `text` cut at the end behind an ellipsis to fit `room` cells.
fn truncate_end(text: &str, room: usize) -> String {
    if Span::raw(text).width() <= room {
        return text.to_owned();
    }
    let room = room.saturating_sub(Span::raw(ELLIPSIS).width());
    let end = text
        .char_indices()
        .map(|(index, _)| index)
        .take_while(|index| Span::raw(&text[..*index]).width() <= room)
        .last()
        .unwrap_or(0);
    format!("{}{ELLIPSIS}", &text[..end])
}
