//! The sidebar's tabs section: client-local folds and the visible tab rows,
//! drawn from one walk of the tree.

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

use super::{Scroll, agents::icon};
use crate::ui::{agents, attention::Attention, draw::tab_label};

/// Columns left of the tree for rollup icons, drawn in the middle one so
/// they line up with the agents section's icons.
const GUTTER: u16 = 3;
const OPEN: &str = "▾ ";
const CLOSED: &str = "▸ ";
/// The marker column a row without children leaves blank.
const LEAF: &str = "  ";
/// Columns per level.
const INDENT: u16 = 2;

#[derive(Default)]
pub(in crate::ui) struct Tabs {
    collapsed: HashSet<IdOf<Tab>>,
    /// First drawn row. Never leaves blank rows below the last one, and
    /// moves to keep the selected row in view when that row changes.
    offset: usize,
    /// Where the rows were last drawn, gutter included.
    area: Rect,
    /// Every visible row as last drawn, scrolled out of view or not, for
    /// clicks, drop gaps and wheel scrolling.
    rows: Vec<Row>,
    /// The selected row as last drawn. A wheel scroll holds until the
    /// selection or its row moves.
    selected: Option<Placed>,
}

/// A tab and its index among the visible rows.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Placed {
    tab: IdOf<Tab>,
    index: usize,
}

/// One visible row, from one walk of the tree under unfolded ancestors.
struct Row {
    tab: IdOf<Tab>,
    depth: u16,
    fold: Fold,
    label: Line<'static>,
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
        self.walk(tabs).into_iter().map(|row| row.tab).collect()
    }

    fn walk(&self, tabs: &TabTree) -> Vec<Row> {
        fn walk(section: &Tabs, tabs: &TabTree, depth: u16, rows: &mut Vec<Row>) {
            for (position, tab) in tabs.values().enumerate() {
                let fold = if tab.tabs.is_empty() {
                    Fold::Leaf
                } else if section.collapsed.contains(&tab.id) {
                    Fold::Folded
                } else {
                    Fold::Open
                };
                // tmux's zoom flag, since a zoomed pane has no border to carry one.
                let mut label = Line::from(tab_label(tab, position));
                if tab.zoomed.is_some() {
                    label.push_span(Span::from(" Z").cyan().bold());
                }
                rows.push(Row {
                    tab: tab.id,
                    depth,
                    fold,
                    label,
                });
                if fold == Fold::Open {
                    walk(section, &tab.tabs, depth + 1, rows);
                }
            }
        }
        let mut rows = Vec::new();
        walk(self, tabs, 0, &mut rows);
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
            marker: row.fold != Fold::Leaf
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
        let shown = self.shown();
        let index = usize::from(at.y - self.area.y).min(shown.len());
        let below = shown.get(index);
        let above = index.checked_sub(1).and_then(|index| shown.get(index));
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
                (to, depth, self.area.y + (index - 1) as u16)
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
    /// show below the last, or the selected row is new or moved.
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
        self.rows = self.walk(tabs);
        let text = Rect {
            x: self.indent(),
            width: area.width.saturating_sub(GUTTER),
            ..area
        };
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
        if let Some(Placed { index, .. }) = placed
            && placed != self.selected
        {
            self.offset = self
                .offset
                .min(index)
                .max((index + 1).saturating_sub(height));
        }
        self.selected = placed;
        let band = Style::new().fg(Color::Reset).on_dark_gray().bold();
        let band = if renaming.is_some() && renaming == selected {
            band.cyan()
        } else {
            band
        };
        for (y, row) in (area.y..area.bottom()).zip(self.shown()) {
            let marker = match row.fold {
                Fold::Leaf => LEAF,
                Fold::Open => OPEN,
                Fold::Folded => CLOSED,
            };
            let indent = " ".repeat(usize::from(INDENT * row.depth));
            let (x, _) = buffer.set_stringn(
                text.x,
                y,
                format!("{indent}{marker}"),
                usize::from(text.width),
                Style::new(),
            );
            buffer.set_line(x, y, &row.label, text.right().saturating_sub(x));
            let gutter = Rect::new(area.x, y, GUTTER, 1).intersection(area);
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
            let display = tree::tab(tabs, row.tab)
                .ok()
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
