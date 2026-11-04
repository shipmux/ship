//! Terminal mouse events to gestures, selections, folds and pane input.

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Position, Rect};
use ship_core::{
    geometry::TabGeometry,
    id::IdOf,
    layout::Axis,
    model::{NodeId, Pane, Tab},
    protocol::{InputFrame, MouseInput},
    tree::Tabs,
};

use super::{
    Scene,
    lane::{Edge, EdgeAt, Item, MenuAt, TabMove},
    menu::{self, Target},
    panes,
    sidebar::{
        Gap, Hit, Scroll, movable,
        tabs::{self, Pointed},
    },
};
use crate::keymap::{Action, ClientAction, ClientUpdates};

/// Where a mouse event goes.
pub(super) enum Routed {
    Lane(Item),
    /// Border drags: one `Item::Resize` per edge, merged by the lane.
    Resize(Vec<EdgeAt>),
    Sidebar(SidebarEdit),
}

/// A client-local change to the sidebar, applied at once.
pub(super) enum SidebarEdit {
    /// The right-hand border, dragged to a width.
    Width(u16),
    /// The divider row between the sidebar's sections, dragged to a row.
    Split(u16),
    /// A wheel step over the sidebar.
    Scroll { at: Position, scroll: Scroll },
}

#[derive(Default)]
pub(super) struct Mouse {
    /// Program drags and releases follow the pane that received the press.
    held: Option<IdOf<Pane>>,
    gesture: Option<Gesture>,
}

enum Gesture {
    Pressed {
        at: Position,
        row: Pointed,
    },
    Row {
        tab: IdOf<Tab>,
        gap: Option<Gap>,
    },
    Border(Vec<Grab>),
    Divider {
        at: Position,
        moved: bool,
    },
    Split,
    /// Selects the entry's pane on release, unless the pointer moved.
    Agent {
        at: Position,
        pane: IdOf<Pane>,
    },
    /// Opens the updates panel on release, unless the pointer moved.
    Version {
        at: Position,
    },
}

struct Grab {
    edge: Edge,
    last: u16,
}

impl Mouse {
    pub fn dragging(&self) -> bool {
        self.gesture.is_some()
    }

    /// Drop feedback whose source or destination disappeared or became invalid.
    pub fn reconcile(&mut self, tabs: &Tabs) {
        if let Some(Gesture::Row { tab, gap }) = &mut self.gesture {
            *gap = gap.filter(|gap| movable(tabs, *tab, gap.to));
        }
    }

    pub fn gap(&self) -> Option<&Gap> {
        match &self.gesture {
            Some(Gesture::Row { gap, .. }) => gap.as_ref(),
            _ => None,
        }
    }

    pub fn held(&self) -> impl Iterator<Item = Edge> + '_ {
        match &self.gesture {
            Some(Gesture::Border(grabs)) => grabs.as_slice(),
            _ => &[],
        }
        .iter()
        .map(|grab| grab.edge)
    }

    pub fn route(&mut self, event: MouseEvent, scene: &Scene, busy: bool) -> Option<Routed> {
        let at = Position::new(event.column, event.row);
        let relative = scene.relative(at);
        let content = |id| Some(scene.on_screen(scene.geometry?.panes.get(&id)?.content));
        let select = |node| Routed::Lane(Item::Select(node));
        let frame = |pane, content| {
            Routed::Lane(Item::Mouse(InputFrame::Mouse(input(event, pane, content))))
        };
        // A target's menu, when it has any items.
        let menu_for = |target| {
            (!menu::items(scene.menus, target).is_empty())
                .then_some(Routed::Lane(Item::Menu(MenuAt { target, at })))
        };
        let down = matches!(event.kind, MouseEventKind::Down(_));
        if down {
            self.held = None;
            self.gesture = None;
        }
        if matches!(
            event.kind,
            MouseEventKind::Drag(MouseButton::Left) | MouseEventKind::Up(MouseButton::Left)
        ) && let Some(mut gesture) = self.gesture.take()
        {
            let release = matches!(event.kind, MouseEventKind::Up(_));
            if let Gesture::Agent { at: origin, .. } | Gesture::Version { at: origin } = &gesture
                && at != *origin
            {
                return None;
            }
            if let Gesture::Pressed { at: origin, row } = &gesture
                && at != *origin
            {
                gesture = Gesture::Row {
                    tab: row.tab,
                    gap: None,
                };
            }
            let routed = match &mut gesture {
                Gesture::Pressed { row, .. } => release.then_some(Routed::Lane(if row.marker {
                    Item::Fold(row.tab)
                } else {
                    Item::Select(NodeId::Tab(row.tab))
                })),
                Gesture::Row { tab, gap } => {
                    *gap = scene
                        .tabs
                        .filter(|_| scene.sidebar.shown)
                        .and_then(|tabs| scene.sidebar.tabs.gap(tabs, *tab, at));
                    let tab = *tab;
                    gap.filter(|_| release)
                        .map(|gap| Routed::Lane(Item::Move(TabMove { tab, to: gap.to })))
                }
                Gesture::Divider { at: origin, moved } => {
                    *moved |= at != *origin;
                    (*moved && scene.sidebar.shown)
                        .then_some(Routed::Sidebar(SidebarEdit::Width(at.x.saturating_add(1))))
                }
                Gesture::Split => scene
                    .sidebar
                    .shown
                    .then_some(Routed::Sidebar(SidebarEdit::Split(at.y))),
                Gesture::Agent { pane, .. } => release.then(|| select(NodeId::Pane(*pane))),
                Gesture::Version { .. } => release.then_some(Routed::Lane(Item::Action(
                    Action::Client(ClientAction::Updates(ClientUpdates::Open {})),
                ))),
                Gesture::Border(grabs) => {
                    if grabs.iter().any(|grab| {
                        !scene
                            .geometry
                            .is_some_and(|geometry| geometry.panes.contains_key(&grab.edge.pane))
                    }) {
                        return None;
                    }
                    let positions: Vec<_> = grabs
                        .iter_mut()
                        .filter_map(|grab| {
                            let to = grab.edge.axis.along(relative);
                            if to == grab.last {
                                return None;
                            }
                            grab.last = to;
                            Some(EdgeAt {
                                edge: grab.edge,
                                to,
                            })
                        })
                        .collect();
                    (!positions.is_empty()).then_some(Routed::Resize(positions))
                }
            };
            if !release {
                self.gesture = Some(gesture);
            }
            return routed;
        }
        match event.kind {
            MouseEventKind::Up(_) | MouseEventKind::Drag(_) => {
                let id = match event.kind {
                    MouseEventKind::Up(_) => self.held.take()?,
                    _ => self.held?,
                };
                Some(frame(id, content(id)?))
            }
            MouseEventKind::Down(button) if !scene.panes.contains(at) => {
                match (button, scene.sidebar.hit(at)?) {
                    (MouseButton::Left, Hit::Button)
                    | (
                        MouseButton::Right,
                        Hit::Heading | Hit::Button | Hit::Tabs(tabs::Hit::Below),
                    ) => menu_for(Target::Global),
                    (MouseButton::Right, Hit::Tabs(tabs::Hit::Row(row))) => {
                        menu_for(Target::Tab(row.tab))
                    }
                    (MouseButton::Left, Hit::Tabs(tabs::Hit::Row(row))) => {
                        self.gesture = Some(Gesture::Pressed { at, row });
                        None
                    }
                    (MouseButton::Left, Hit::Divider) => {
                        self.gesture = Some(Gesture::Divider { at, moved: false });
                        None
                    }
                    (MouseButton::Left, Hit::Split) => {
                        self.gesture = Some(Gesture::Split);
                        None
                    }
                    (MouseButton::Left, Hit::Agent(pane)) => {
                        self.gesture = Some(Gesture::Agent { at, pane });
                        None
                    }
                    (MouseButton::Left, Hit::Version) => {
                        self.gesture = Some(Gesture::Version { at });
                        None
                    }
                    _ => None,
                }
            }
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown if !scene.panes.contains(at) => {
                scene.sidebar.hit(at)?;
                Some(Routed::Sidebar(SidebarEdit::Scroll {
                    at,
                    scroll: match event.kind {
                        MouseEventKind::ScrollUp => Scroll::Up,
                        _ => Scroll::Down,
                    },
                }))
            }
            _ if !scene.panes.contains(at) => None,
            kind => {
                let geometry = scene.geometry?;
                if kind == MouseEventKind::Down(MouseButton::Right)
                    && let Some(pane) = owner(geometry, relative)
                {
                    if let Some(content) = content(pane)
                        && content.contains(at)
                        && passes(scene, pane)
                    {
                        self.held = Some(pane);
                        return Some(frame(pane, content));
                    }
                    if let Some(menu) = menu_for(Target::Pane(pane)) {
                        return Some(menu);
                    }
                }
                if kind == MouseEventKind::Down(MouseButton::Left) {
                    if let Some(pane) = title_at(scene, at) {
                        return Some(select(NodeId::Pane(pane)));
                    }
                    let edges = edges_at(geometry, relative);
                    if !edges.is_empty() {
                        self.gesture = Some(Gesture::Border(
                            edges
                                .into_iter()
                                .map(|edge| Grab {
                                    last: edge.axis.along(relative),
                                    edge,
                                })
                                .collect(),
                        ));
                        return None;
                    }
                }
                let (&id, pane) = geometry
                    .panes
                    .iter()
                    .find(|(_, pane)| scene.on_screen(pane.frame).contains(at))?;
                if down && (busy || Some(id) != scene.selected) {
                    return Some(select(NodeId::Pane(id)));
                }
                let wheel = matches!(kind, MouseEventKind::ScrollUp | MouseEventKind::ScrollDown);
                let content = scene.on_screen(pane.content);
                if !content.contains(at) || (!wheel && Some(id) != scene.selected) {
                    return None;
                }
                if down {
                    self.held = Some(id);
                }
                Some(frame(id, content))
            }
        }
    }
}

/// A pane behind the front whose visible title covers `at`.
fn title_at(scene: &Scene, at: Position) -> Option<IdOf<Pane>> {
    let front = owner(scene.geometry?, scene.relative(at))?;
    let slot = scene.tab?.slot(front)?;
    let row = panes::title_row(scene, front)?;
    panes::titles(slot, row)
        .into_iter()
        .find(|(pane, cells)| *pane != front && cells.contains(at))
        .map(|(pane, _)| pane)
}

/// Shared frame cells, never outer edges or content. Layout order breaks ties
/// between the panes ending at a junction, with at most one edge per axis.
fn edges_at(geometry: &TabGeometry, at: Position) -> Vec<Edge> {
    if geometry.panes.len() < 2
        || geometry
            .panes
            .values()
            .any(|pane| pane.content.contains(at))
        || geometry
            .panes
            .values()
            .filter(|pane| pane.frame.contains(at))
            .count()
            < 2
    {
        return Vec::new();
    }
    [Axis::Horizontal, Axis::Vertical]
        .into_iter()
        .filter_map(|axis| {
            geometry
                .panes
                .iter()
                .find(|(_, pane)| {
                    pane.frame.contains(at)
                        && match axis {
                            Axis::Horizontal => {
                                pane.frame.right().saturating_sub(1) == at.x
                                    && geometry.panes.values().any(|other| {
                                        other.frame.contains(at)
                                            && other.frame.x == at.x
                                            && other.frame.x > pane.frame.x
                                    })
                            }
                            Axis::Vertical => {
                                pane.frame.bottom().saturating_sub(1) == at.y
                                    && geometry.panes.values().any(|other| {
                                        other.frame.contains(at)
                                            && other.frame.y == at.y
                                            && other.frame.y > pane.frame.y
                                    })
                            }
                        }
                })
                .map(|(pane, _)| Edge { pane: *pane, axis })
        })
        .collect()
}

/// Shared borders belong to the lowest, then rightmost containing frame.
/// The same rule gives each label and outer edge to its own pane.
fn owner(geometry: &TabGeometry, at: Position) -> Option<IdOf<Pane>> {
    geometry
        .panes
        .iter()
        .filter(|(_, pane)| pane.frame.contains(at))
        .max_by_key(|(_, pane)| (pane.frame.y, pane.frame.x))
        .map(|(id, _)| *id)
}

/// Decide passthrough at press; the held pane owns its drag and release.
fn passes(scene: &Scene, pane: IdOf<Pane>) -> bool {
    scene
        .tab
        .and_then(|tab| tab.panes().find(|candidate| candidate.id == pane))
        .is_some_and(|pane| pane.right_click_passthrough)
        && scene.screens.get(&pane).is_some_and(|screen| screen.mouse)
}

/// `event` for `pane`, relative to its `content`, negative above or left.
fn input(event: MouseEvent, pane: IdOf<Pane>, content: Rect) -> MouseInput {
    MouseInput {
        pane,
        kind: event.kind,
        modifiers: event.modifiers,
        x: i32::from(event.column) - i32::from(content.x),
        y: i32::from(event.row) - i32::from(content.y),
    }
}
