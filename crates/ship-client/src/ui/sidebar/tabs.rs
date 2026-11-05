//! The sidebar's tabs section: client-local folds and the visible tab
//! entries, drawn from one walk of the tree, one row per line (KD-5).

use std::collections::HashSet;

use ratatui::{
    Frame,
    layout::{Position, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
};
use ship_core::{
    id::IdOf,
    model::{NodeId, Tab},
    protocol::MoveTab,
    tree::{self, Tabs as TabTree},
};

use super::{
    Scroll,
    agents::icon,
    fit::{self, Piece},
    tags,
};
use crate::{
    keymap::{TabSettings, TabToken},
    ui::{agents, attention::Attention, draw::tab_label},
};

/// Columns left of the tree for rollup icons, drawn in the middle one so
/// they line up with the agents section's icons.
const GUTTER: u16 = 3;
const OPEN: &str = "▾ ";
const CLOSED: &str = "▸ ";
/// The marker column a row without children leaves blank.
const LEAF: &str = "  ";
/// Columns per level.
const INDENT: u16 = 2;
/// Between the items of a line (C5).
const SEPARATOR: &str = " · ";

pub(in crate::ui) struct Tabs {
    collapsed: HashSet<IdOf<Tab>>,
    /// First drawn row. Never leaves blank rows below the last one, and
    /// moves to keep the selected row in view when that row changes.
    offset: usize,
    /// Where the rows were last drawn, gutter included.
    area: Rect,
    /// Every visible line as last drawn, one or more per entry, scrolled
    /// out of view or not, for clicks, drop gaps and wheel scrolling.
    rows: Vec<Row>,
    /// The selected entry as last drawn. A wheel scroll holds until the
    /// selection or its entry moves.
    selected: Option<Placed>,
    /// `[client.tabs]`, from `Sidebar::configure`.
    settings: TabSettings,
}

/// A tab and the index of its entry's head among the visible rows.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Placed {
    tab: IdOf<Tab>,
    index: usize,
}

/// One visible tab, from one walk of the tree under unfolded ancestors.
struct Entry {
    tab: IdOf<Tab>,
    depth: u16,
    fold: Fold,
    /// Among its siblings, for the automatic name.
    position: usize,
}

/// One drawn line of an entry.
struct Row {
    tab: IdOf<Tab>,
    depth: u16,
    part: Part,
    text: Line<'static>,
}

/// Which line of its entry a row is.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Part {
    /// The first: carries the fold marker and the rollup icon.
    Head(Fold),
    /// A later line, indented to the head's text.
    Tail,
}

/// A row's fold marker. Was two flags, one meaningless without the other.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Fold {
    /// No children, no marker.
    Leaf,
    Open,
    Folded,
}

/// A valid destination and the start of its underlined row.
#[derive(Clone, Copy)]
pub(in crate::ui) struct Gap {
    pub to: MoveTab,
    line: Position,
}

pub(in crate::ui) enum Hit {
    Row(Pointed),
    /// Below the last row.
    Below,
}

/// A tab row under the pointer.
pub(in crate::ui) struct Pointed {
    pub tab: IdOf<Tab>,
    /// On the fold marker, which only rows with children have.
    pub marker: bool,
}

impl Tabs {
    pub fn new(settings: TabSettings) -> Self {
        Self {
            collapsed: HashSet::new(),
            offset: 0,
            area: Rect::ZERO,
            rows: Vec::new(),
            selected: None,
            settings,
        }
    }

    pub fn configure(&mut self, settings: TabSettings) {
        self.settings = settings;
    }

    /// The sidebar has already checked that `at` is above the split and off
    /// the divider.
    pub fn hit(&self, at: Position) -> Option<Hit> {
        if let Some(row) = self.row_at(at) {
            return Some(Hit::Row(row));
        }
        (at.y >= self.area.y).then_some(Hit::Below)
    }

    pub fn scroll(&mut self, scroll: Scroll) {
        match scroll {
            Scroll::Up => self.offset = self.offset.saturating_sub(1),
            // Stop once the last row reaches the bottom.
            Scroll::Down => {
                if self.offset + usize::from(self.area.height) < self.rows.len() {
                    self.offset += 1;
                }
            }
        }
    }

    /// Forget folds for removed tabs or tabs without children.
    pub fn reconcile(&mut self, tabs: &TabTree) {
        self.collapsed
            .retain(|id| tree::tab(tabs, *id).is_ok_and(|tab| !tab.tabs.is_empty()));
    }

    /// Open the selection's ancestors, leaving its own row's fold unchanged.
    pub fn reveal(&mut self, tabs: &TabTree, selection: Option<NodeId>) {
        let path = selection
            .and_then(|node| tree::path(tabs, node))
            .unwrap_or_default();
        let selected = selection.and_then(|node| tree::viewed_tab(tabs, node));
        for node in path {
            if let NodeId::Tab(id) = node
                && Some(id) != selected
            {
                self.collapsed.remove(&id);
            }
        }
    }

    /// Tree order under unfolded ancestors, even while the sidebar is hidden.
    pub fn visible(&self, tabs: &TabTree) -> Vec<IdOf<Tab>> {
        self.walk(tabs).into_iter().map(|entry| entry.tab).collect()
    }

    fn walk(&self, tabs: &TabTree) -> Vec<Entry> {
        fn walk(section: &Tabs, tabs: &TabTree, depth: u16, entries: &mut Vec<Entry>) {
            for (position, tab) in tabs.values().enumerate() {
                let fold = if tab.tabs.is_empty() {
                    Fold::Leaf
                } else if section.collapsed.contains(&tab.id) {
                    Fold::Folded
                } else {
                    Fold::Open
                };
                entries.push(Entry {
                    tab: tab.id,
                    depth,
                    fold,
                    position,
                });
                if fold == Fold::Open {
                    walk(section, &tab.tabs, depth + 1, entries);
                }
            }
        }
        let mut entries = Vec::new();
        walk(self, tabs, 0, &mut entries);
        entries
    }

    /// Each entry's lines in `width` cells, after its indent and marker.
    fn rows(&self, tabs: &TabTree, width: u16) -> Vec<Row> {
        let mut rows = Vec::new();
        for entry in self.walk(tabs) {
            let Ok(tab) = tree::tab(tabs, entry.tab) else {
                continue;
            };
            let room = width.saturating_sub(INDENT.saturating_mul(entry.depth + 1));
            let lines = lines(tab, entry.position, &self.settings.rows, room);
            for (index, text) in lines.into_iter().enumerate() {
                rows.push(Row {
                    tab: entry.tab,
                    depth: entry.depth,
                    part: if index == 0 {
                        Part::Head(entry.fold)
                    } else {
                        Part::Tail
                    },
                    text,
                });
            }
        }
        rows
    }

    /// Whether this client folded `tab`'s row.
    pub fn folded(&self, tab: IdOf<Tab>) -> bool {
        self.collapsed.contains(&tab)
    }

    pub fn collapse(&mut self, tab: IdOf<Tab>) {
        self.collapsed.insert(tab);
    }

    pub fn expand(&mut self, tab: IdOf<Tab>) {
        self.collapsed.remove(&tab);
    }

    /// How many tabs are folded; each fold or unfold changes it.
    pub fn fold_count(&self) -> usize {
        self.collapsed.len()
    }

    pub fn toggle(&mut self, tab: IdOf<Tab>) {
        if !self.collapsed.remove(&tab) {
            self.collapsed.insert(tab);
        }
    }

    /// The rows on screen, top first, one per line from `area.y`.
    fn shown(&self) -> &[Row] {
        let start = self.offset.min(self.rows.len());
        let end = (self.offset + usize::from(self.area.height)).min(self.rows.len());
        &self.rows[start..end]
    }

    /// The row drawn at `position`, gutter included, if any.
    fn row_at(&self, position: Position) -> Option<Pointed> {
        let index = position.y.checked_sub(self.area.y)?;
        let row = self.shown().get(usize::from(index))?;
        let start = self
            .indent()
            .saturating_add(INDENT.saturating_mul(row.depth));
        Some(Pointed {
            tab: row.tab,
            marker: matches!(row.part, Part::Head(Fold::Open | Fold::Folded))
                && (start..start.saturating_add(INDENT)).contains(&position.x),
        })
    }

    /// Resolve the gap above the pointer's row, or after the last drawn row.
    /// The pointer must be on the section's rows, the divider column included.
    pub fn gap(&self, tabs: &TabTree, tab: IdOf<Tab>, at: Position) -> Option<Gap> {
        let section = Rect {
            width: self.area.width.saturating_add(1),
            ..self.area
        };
        if !section.contains(at) {
            return None;
        }
        let start = self.offset.min(self.rows.len());
        let end = (self.offset + usize::from(self.area.height)).min(self.rows.len());
        let mut index = (start + usize::from(at.y - self.area.y)).min(end);
        // Drops land between entries: a later line stands for its head.
        while index > 0
            && self
                .rows
                .get(index)
                .is_some_and(|row| row.part == Part::Tail)
        {
            index -= 1;
        }
        let below = self.rows.get(index);
        let above = (index > start).then(|| &self.rows[index - 1]);
        let indent = self.indent();
        let (to, depth, y) = match above {
            None => {
                let below = below?;
                (
                    MoveTab::Before(below.tab),
                    below.depth,
                    self.area.y.saturating_sub(1),
                )
            }
            Some(above) => {
                let deepest = above.depth.saturating_add(1);
                let depth = (at.x.saturating_sub(indent) / INDENT)
                    .clamp(below.map_or(0, |row| row.depth), deepest);
                let to = if depth == deepest {
                    match below.filter(|row| row.depth == deepest) {
                        Some(below) => MoveTab::Before(below.tab),
                        None => MoveTab::Parent(Some(above.tab)),
                    }
                } else {
                    let path = tree::path(tabs, NodeId::Tab(above.tab))?;
                    let NodeId::Tab(ancestor) = *path.get(usize::from(depth))? else {
                        return None;
                    };
                    MoveTab::After(ancestor)
                };
                (to, depth, self.area.y + (index - 1 - start) as u16)
            }
        };
        movable(tabs, tab, to).then_some(Gap {
            to,
            line: Position::new(indent.saturating_add(INDENT.saturating_mul(depth)), y),
        })
    }

    fn indent(&self) -> u16 {
        self.area.x.saturating_add(GUTTER)
    }

    /// The rows in `area`, and the drop line on its rows or the blank row
    /// above them. Keeps the first row in place unless blank rows would
    /// show below the last, or the selected entry is new or moved.
    #[expect(
        clippy::too_many_arguments,
        reason = "Keep the explicit drawing inputs prescribed by the locked UI design"
    )]
    pub fn draw(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        tabs: &TabTree,
        selection: Option<NodeId>,
        gap: Option<&Gap>,
        attention: &Attention,
        renaming: Option<IdOf<Tab>>,
    ) {
        self.area = area;
        let text = Rect {
            x: self.indent(),
            width: area.width.saturating_sub(GUTTER),
            ..area
        };
        self.rows = self.rows(tabs, text.width);
        let buffer = frame.buffer_mut();
        buffer.set_style(text, Style::new().gray());
        if text.is_empty() {
            self.rows.clear();
            return;
        }
        let selected = selection.and_then(|node| tree::viewed_tab(tabs, node));
        let placed = selected.and_then(|tab| {
            let index = self.rows.iter().position(|row| row.tab == tab)?;
            Some(Placed { tab, index })
        });
        let height = usize::from(area.height);
        self.offset = self.offset.min(self.rows.len().saturating_sub(height));
        if let Some(Placed { tab, index }) = placed
            && placed != self.selected
        {
            // The whole entry, or its head when it's taller than the section.
            let lines = self.rows[index..]
                .iter()
                .take_while(|row| row.tab == tab)
                .count();
            self.offset = self
                .offset
                .max((index + lines).saturating_sub(height))
                .min(index);
        }
        self.selected = placed;
        let band = Style::new().fg(Color::Reset).on_dark_gray().bold();
        let band = if renaming.is_some() && renaming == selected {
            band.cyan()
        } else {
            band
        };
        for (y, row) in (area.y..area.bottom()).zip(self.shown()) {
            let gutter = Rect::new(area.x, y, GUTTER, 1).intersection(area);
            // The band goes under the text, so a tag's `bg` stays on top.
            if Some(row.tab) == selected {
                buffer.set_style(
                    Rect {
                        y,
                        height: 1,
                        ..text
                    },
                    band,
                );
                buffer.set_style(gutter, band);
            }
            let marker = match row.part {
                Part::Head(Fold::Leaf) | Part::Tail => LEAF,
                Part::Head(Fold::Open) => OPEN,
                Part::Head(Fold::Folded) => CLOSED,
            };
            let indent = " ".repeat(usize::from(INDENT * row.depth));
            let (x, _) = buffer.set_stringn(
                text.x,
                y,
                format!("{indent}{marker}"),
                usize::from(text.width),
                Style::new(),
            );
            buffer.set_line(x, y, &row.text, text.right().saturating_sub(x));
            let display = matches!(row.part, Part::Head(_))
                .then(|| tree::tab(tabs, row.tab).ok())
                .flatten()
                .and_then(|tab| agents::rollup(tab, attention));
            if let Some(display) = display
                && gutter.width > 1
            {
                let icon = icon(display);
                buffer[(gutter.x + 1, y)]
                    .set_symbol(&icon.content)
                    .set_style(icon.style);
            }
        }
        if let Some(gap) = gap {
            let rows = Rect {
                y: area.y.saturating_sub(1),
                height: area.height.saturating_add(1),
                ..area
            };
            let line = Rect::new(
                gap.line.x,
                gap.line.y,
                area.right().saturating_sub(gap.line.x),
                1,
            )
            .intersection(rows);
            for at in line.positions() {
                buffer[at].set_style(Style::new().underlined());
            }
        }
    }
}

/// The entry's lines: one per `rows` line with any value, items joined by
/// ` · ` (C5) and fitted to `width` (amendment P4). At least one line,
/// blank if nothing has a value, so every visible tab can still be clicked
/// and stepped to.
fn lines(tab: &Tab, position: usize, rows: &[Vec<TabToken>], width: u16) -> Vec<Line<'static>> {
    let mut lines: Vec<_> = rows
        .iter()
        .filter_map(|tokens| {
            let values = tokens
                .iter()
                .filter_map(|token| value(tab, position, token))
                .collect();
            let spans = fit::line(values, SEPARATOR, usize::from(width));
            (!spans.is_empty()).then(|| Line::from(spans))
        })
        .collect();
    if lines.is_empty() {
        lines.push(Line::default());
    }
    lines
}

/// One item's pieces: the name with ` Z` when zoomed, or a tag through
/// `tags::on_tab`.
fn value(tab: &Tab, position: usize, token: &TabToken) -> Option<Vec<Piece>> {
    match token {
        TabToken::Name => {
            let mut pieces = vec![Piece::end(Span::from(tab_label(tab, position)))];
            // tmux's zoom flag, since a zoomed pane has no border to carry one.
            if tab.zoomed.is_some() {
                pieces.push(Piece::keep(Span::from(" Z").cyan().bold()));
            }
            Some(pieces)
        }
        TabToken::Tag(name) => Some(vec![Piece::end(tags::span(tags::on_tab(tab, name)?)?)]),
    }
}

/// Shared by drag-time feedback and the lane's turn-time re-check.
pub(in crate::ui) fn movable(tabs: &TabTree, tab: IdOf<Tab>, to: MoveTab) -> bool {
    if tree::tab(tabs, tab).is_err() {
        return false;
    }
    let target = match to {
        MoveTab::Parent(None) => None,
        MoveTab::Parent(Some(id)) | MoveTab::Before(id) | MoveTab::After(id) => Some(id),
    };
    if let Some(target) = target
        && !tree::path(tabs, NodeId::Tab(target))
            .is_some_and(|path| !path.contains(&NodeId::Tab(tab)))
    {
        return false;
    }
    match to {
        MoveTab::Parent(parent) => {
            tree::parent(tabs, tab) != parent
                || tree::siblings(tabs, tab)
                    .and_then(|tabs| tabs.last())
                    .map(|(id, _)| *id)
                    != Some(tab)
        }
        MoveTab::Before(target) | MoveTab::After(target) => {
            let Some((siblings, at)) = tree::siblings(tabs, target)
                .and_then(|siblings| Some((siblings, siblings.get_index_of(&target)?)))
            else {
                return false;
            };
            let unchanged = match to {
                MoveTab::Before(_) => at.checked_sub(1),
                MoveTab::After(_) => at.checked_add(1),
                MoveTab::Parent(_) => unreachable!(),
            };
            siblings
                .get_index_of(&tab)
                .is_none_or(|index| Some(index) != unchanged)
        }
    }
}
