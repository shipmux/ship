//! Where navigation keys and tab-tree actions go, read from one view of
//! the replica per turn. No requests; callers turn results into lane items.

use ship_core::{
    command::{
        Command, Direction, TabTarget,
        tab::{self, TabCommand},
    },
    id::IdOf,
    layout::{CycleDirection, Layout},
    model::{NodeId, Pane, Switch, Tab},
    protocol::MoveTab,
    tree::{self, Tabs},
};

use super::{lane::TabMove, memory::Memory, observer::Selected, sidebar::Sidebar};
use crate::keymap::{Action, ClientAction, ClientPane, ClientTab, TabDirection};

/// Where a navigation key moves the selection.
#[derive(Clone, Copy)]
pub(super) enum Step {
    /// Around the visible sidebar rows.
    Tab(CycleDirection),
    /// Around the viewed tab's slot fronts in layout order.
    Pane(CycleDirection),
    /// To the selected pane's neighbor on that side.
    Focus(Direction),
}

impl Step {
    /// The step a navigation action takes; `None` for any other action.
    pub fn of(action: &Action) -> Option<Self> {
        Some(match action {
            Action::Client(ClientAction::Tab(ClientTab::Next {})) => {
                Self::Tab(CycleDirection::Next)
            }
            Action::Client(ClientAction::Tab(ClientTab::Prev {})) => {
                Self::Tab(CycleDirection::Prev)
            }
            Action::Client(ClientAction::Pane(ClientPane::Next {})) => {
                Self::Pane(CycleDirection::Next)
            }
            Action::Client(ClientAction::Pane(ClientPane::Prev {})) => {
                Self::Pane(CycleDirection::Prev)
            }
            Action::Client(ClientAction::Pane(ClientPane::Focus { direction })) => {
                Self::Focus(*direction)
            }
            _ => return None,
        })
    }
}

/// What `client.tab.collapse` or `client.tab.expand` does on the viewed
/// tab's row.
pub(super) enum Hop {
    /// Fold the row, keeping the selection.
    Collapse(IdOf<Tab>),
    /// Unfold the row, keeping the selection.
    Expand(IdOf<Tab>),
    /// Select this node, a tab's landing node.
    Select(NodeId),
}

/// The replica's tabs, this client's selection and the client-local state
/// navigation reads. Built once per turn by `Local::view`.
pub(super) struct View<'a> {
    pub tabs: &'a Tabs,
    pub selection: Option<NodeId>,
    pub memory: &'a Memory,
    pub sidebar: &'a Sidebar,
}

impl<'a> View<'a> {
    /// The selection resolved in `tabs`.
    pub fn selected(&self) -> Option<Selected<'a>> {
        Selected::resolve(self.tabs, self.selection?)
    }

    /// The selected tab, or the selected pane's tab.
    pub fn viewed_tab(&self) -> Option<IdOf<Tab>> {
        self.selected().map(|selected| selected.tab.id)
    }

    /// Where `step` moves the selection. Visible tab rows cycle in order and
    /// land where `memory` says; from nothing selected they go to the first or
    /// last. Panes cycle within their tab; from a tab, to its first or last
    /// pane. Focus goes to the neighbor in the tab's published geometry, ties
    /// to the panes `memory` selected most recently. `None` when there is
    /// nowhere to go.
    pub fn navigate(&self, step: Step) -> Option<NodeId> {
        let tabs = self.tabs;
        match step {
            Step::Tab(direction) => {
                let rows = self.sidebar.tabs.visible(tabs);
                let index = match self.selection {
                    Some(_) => {
                        let tab = self.viewed_tab()?;
                        self::step(
                            rows.iter().position(|id| *id == tab)?,
                            rows.len(),
                            direction,
                        )
                    }
                    None => match direction {
                        CycleDirection::Next => 0,
                        CycleDirection::Prev => rows.len().checked_sub(1)?,
                    },
                };
                let tab = tree::tab(tabs, *rows.get(index)?).ok()?;
                Some(self.memory.landing(tab))
            }
            Step::Pane(direction) => {
                let from = self.selection?;
                let tab = self.selected()?.tab;
                let panes: Vec<_> = tab
                    .layout
                    .iter()
                    .flat_map(Layout::slots)
                    .map(|slot| slot.front().id)
                    .collect();
                let index = match (from, direction) {
                    (NodeId::Pane(pane), _) => self::step(
                        panes.iter().position(|id| *id == pane)?,
                        panes.len(),
                        direction,
                    ),
                    (NodeId::Tab(_), CycleDirection::Next) => 0,
                    (NodeId::Tab(_), CycleDirection::Prev) => panes.len().checked_sub(1)?,
                };
                Some(NodeId::Pane(*panes.get(index)?))
            }
            Step::Focus(direction) => {
                let Some(NodeId::Pane(from)) = self.selection else {
                    return None;
                };
                let tab = tree::pane_tab(tabs, from).ok()?;
                let to = tab
                    .geometry
                    .as_ref()?
                    .neighbor(from, direction, self.memory.recent())?;
                Some(NodeId::Pane(to))
            }
        }
    }

    /// Fold an open row with children, else select its parent. Nothing on a
    /// top-level tab that's folded or has no children.
    pub fn collapse(&self) -> Option<Hop> {
        let row = self.selected()?.tab;
        let tab = row.id;
        if !row.tabs.is_empty() && !self.sidebar.tabs.folded(tab) {
            return Some(Hop::Collapse(tab));
        }
        self.land(NodeId::Tab(tree::parent(self.tabs, tab)?))
            .map(Hop::Select)
    }

    /// Unfold a folded row, else select its first child. Nothing on a leaf.
    pub fn expand(&self) -> Option<Hop> {
        let row = self.selected()?.tab;
        let tab = row.id;
        let (child, _) = row.tabs.first()?;
        if self.sidebar.tabs.folded(tab) {
            Some(Hop::Expand(tab))
        } else {
            self.land(NodeId::Tab(*child)).map(Hop::Select)
        }
    }

    /// The viewed tab's step among siblings or levels, counting folded
    /// siblings too. Nothing at an edge.
    pub fn shift(&self, direction: TabDirection) -> Option<TabMove> {
        let tab = self.viewed_tab()?;
        let siblings = tree::siblings(self.tabs, tab)?;
        let index = siblings.get_index_of(&tab)?;
        let to = match direction {
            TabDirection::Up => MoveTab::Before(*siblings.get_index(index.checked_sub(1)?)?.0),
            TabDirection::Down => MoveTab::After(*siblings.get_index(index.checked_add(1)?)?.0),
            TabDirection::In => {
                MoveTab::Parent(Some(*siblings.get_index(index.checked_sub(1)?)?.0))
            }
            TabDirection::Out => MoveTab::After(tree::parent(self.tabs, tab)?),
        };
        Some(TabMove { tab, to })
    }

    /// A clicked node, or a tab's landing node from `Memory::landing`.
    /// `None` when the node is gone.
    pub fn land(&self, node: NodeId) -> Option<NodeId> {
        match node {
            NodeId::Tab(id) => Some(self.memory.landing(tree::tab(self.tabs, id).ok()?)),
            NodeId::Pane(_) => tree::path(self.tabs, node).map(|_| node),
        }
    }

    /// Where typing goes: the selected pane, or a selected tab's landing
    /// pane without changing the tab selection.
    pub fn typing_pane(&self) -> Option<IdOf<Pane>> {
        let selected = self.selected()?;
        match selected.pane {
            Some(pane) => Some(pane.id),
            None => match self.memory.landing(selected.tab) {
                NodeId::Pane(pane) => Some(pane),
                NodeId::Tab(_) => None,
            },
        }
    }

    /// The selected pane, when its tab is zoomed onto it.
    pub fn zoomed_pane(&self) -> Option<IdOf<Pane>> {
        let selected = self.selected()?;
        let pane = selected.pane?.id;
        (selected.tab.zoomed == Some(pane)).then_some(pane)
    }

    /// The tab edit for `client.pane.zoom`, against whether the selected
    /// pane's tab is zoomed on it. `None` when no pane is selected.
    pub fn zoom(&self, zoomed: Switch) -> Option<Command> {
        let Some(NodeId::Pane(pane)) = self.selection else {
            return None;
        };
        let current = self.zoomed_pane().is_some();
        Some(set_zoom(pane, zoomed.apply(current).then_some(pane)))
    }
}

/// Zoom `pane`'s tab onto `zoomed`, or unzoom it with `None`.
pub(super) fn set_zoom(pane: IdOf<Pane>, zoomed: Option<IdOf<Pane>>) -> Command {
    Command::Tab(TabCommand::Set(tab::Set {
        tab: TabTarget {
            id: None,
            pane: Some(pane),
        },
        name: None,
        zoomed: Some(zoomed),
    }))
}

/// Actions that move off the selected pane, so a zoomed tab unzooms first.
pub(super) fn leaves_zoom(action: &Action) -> bool {
    matches!(
        action,
        Action::Client(ClientAction::Pane(
            ClientPane::Next {}
                | ClientPane::Prev {}
                | ClientPane::Focus { .. }
                | ClientPane::Swap { .. }
        ))
    )
}

/// The index after or before `index` among `len`, wrapping around.
fn step(index: usize, len: usize, direction: CycleDirection) -> usize {
    match direction {
        CycleDirection::Next => (index + 1) % len,
        CycleDirection::Prev => (index + len - 1) % len,
    }
}
