//! Layout geometry has one owner: the server. Terminal shrink ignores edit minima.

use std::{collections::HashMap, sync::Arc};

use indexmap::IndexMap;
use ratatui::{
    layout::{Constraint, Layout as TuiLayout, Rect, Spacing},
    widgets::Block,
};
use ship_core::{
    geometry::{PaneGeometry, TabGeometry},
    id::{Id, IdOf},
    layout::{Axis, Layout, Ratio, Slot},
    model::{Pane, PaneStatus, Tab},
    prelude::*,
    protocol::Place,
    screen::Size,
    tree::Tabs,
};

use super::layout;

pub(crate) const MIN_CONTENT: Size = Size { cols: 1, rows: 1 };

/// Project source tabs into published tabs. Source geometry stays absent;
/// copy only viewed tabs and their ancestors, sharing all other branches.
/// One recursive walk avoids a separate geometry map or repeated ID lookups.
pub(crate) fn publish(tabs: &Tabs, sizes: &HashMap<IdOf<Tab>, Size>) -> Tabs {
    fn decorate(tabs: &Tabs, sizes: &HashMap<IdOf<Tab>, Size>) -> Option<Tabs> {
        let mut published: Option<Tabs> = None;
        for (id, source) in tabs {
            let children = decorate(&source.tabs, sizes);
            let size = sizes.get(id);
            if children.is_none() && size.is_none() {
                continue;
            }
            let mut projected = Tab::clone(source);
            projected.geometry = size.map(|size| Box::new(tab(source, *size)));
            if let Some(children) = children {
                projected.tabs = children;
            }
            published
                .get_or_insert_with(|| tabs.clone())
                .insert(*id, Arc::new(projected));
        }
        published
    }
    decorate(tabs, sizes).unwrap_or_else(|| tabs.clone())
}

/// The weight `halves` gives the two Fill constraints between them.
const TOTAL: u16 = 1 << 15;

/// Fill weights cover the whole area with overlap; Ratio constraints leave
/// a trailing cell unused. Quantize only for ratatui, never the stored share.
fn halves(area: Rect, axis: Axis, ratio: Ratio) -> [Rect; 2] {
    let first = (f32::from(ratio) * f32::from(TOTAL)).round() as u16;
    let first = first.clamp(1, TOTAL - 1);
    TuiLayout::default()
        .direction(axis.into())
        .constraints([Constraint::Fill(first), Constraint::Fill(TOTAL - first)])
        .spacing(Spacing::Overlap(1))
        .areas(area)
}

/// A ratio whose first half of `area` is `length` cells along `axis`, from
/// the middle of the weights that give it, so the edge stays put when the
/// ratio is read back and keeps its proportion as the area changes. A power
/// of two total makes each weight's ratio exact. `None` when no weight does.
pub(crate) fn ratio_for(area: Rect, axis: Axis, length: u16) -> Option<Ratio> {
    let first = |weight: u16| {
        let ratio = Ratio::try_from(f32::from(weight) / f32::from(TOTAL)).ok()?;
        let [first, _] = halves(area, axis, ratio);
        Some(axis.length(first))
    };
    // The least weight whose first half is at least `length`.
    let least = |length: u16| {
        let (mut low, mut high) = (1, TOTAL);
        while low < high {
            let middle = low + (high - low) / 2;
            if first(middle).is_some_and(|first| first >= length) {
                high = middle;
            } else {
                low = middle + 1;
            }
        }
        low
    };
    let (start, end) = (least(length), least(length.checked_add(1)?));
    let weight = start + (end - start).saturating_sub(1) / 2;
    (first(weight)? == length)
        .then(|| Ratio::try_from(f32::from(weight) / f32::from(TOTAL)).ok())?
}

pub(crate) fn tab(tab: &Tab, size: Size) -> TabGeometry {
    fn walk(
        layout: &Layout,
        frame: Rect,
        bordered: bool,
        panes: &mut IndexMap<IdOf<Pane>, PaneGeometry>,
    ) {
        match layout {
            Layout::Slot(slot) => {
                let content = content(frame, bordered, slot);
                panes.insert(slot.front().id, PaneGeometry { frame, content });
            }
            Layout::Split(split) => {
                let [first, second] = halves(frame, split.axis, split.ratio);
                walk(&split.first, first, bordered, panes);
                walk(&split.second, second, bordered, panes);
            }
        }
    }
    let mut panes = IndexMap::new();
    let frame = Rect::new(0, 0, size.cols, size.rows);
    if let Some(slot) = tab.zoomed.and_then(|id| tab.slot(id)) {
        panes.insert(
            slot.front().id,
            PaneGeometry {
                frame,
                content: content(frame, false, slot),
            },
        );
    } else if let Some(layout) = &tab.layout {
        walk(
            layout,
            frame,
            matches!(layout, Layout::Split(_)),
            &mut panes,
        );
    }
    TabGeometry { size, panes }
}

/// Only a borderless stack needs a separate title row.
fn content(frame: Rect, bordered: bool, slot: &Slot) -> Rect {
    if bordered {
        return Block::bordered().inner(frame);
    }
    let title = u16::from(slot.len().get() > 1).min(frame.height);
    Rect::new(
        frame.x,
        frame.y.saturating_add(title),
        frame.width,
        frame.height - title,
    )
}

/// The content size a new pane would get: place a placeholder in a copy of
/// `tab` at `anchor` by `place`, then read its rectangle from `tab()`. When
/// another slot's zoom hides it, read the unzoomed copy instead.
/// `InvalidStructure` when it or the split anchor's front would fall below
/// `MIN_CONTENT`. An empty tab gets `size`.
pub(crate) fn start_size(
    tab: &Tab,
    size: Size,
    anchor: Option<IdOf<Pane>>,
    place: Place,
) -> Result<Size> {
    let Some(anchor) = anchor else {
        return Ok(size);
    };
    let new = Id::random();
    let mut trial = tab.clone();
    layout::place(&mut trial, Some(anchor), place, placeholder(new))?;
    let mut geometry = self::tab(&trial, size);
    if !geometry.panes.contains_key(&new) {
        trial.zoomed = None;
        geometry = self::tab(&trial, size);
    }
    // A stack's front is the placeholder itself.
    let front = trial
        .slot(anchor)
        .expect("anchor stays in its tab")
        .front()
        .id;
    if [new, front].iter().any(|pane| {
        let content = geometry.panes[pane].content;
        content.width < MIN_CONTENT.cols || content.height < MIN_CONTENT.rows
    }) {
        return Err(err!(InvalidStructure, "no space for new pane"));
    }
    Ok(Size::from(geometry.panes[&new].content))
}

/// A pane with only an ID, for trial placements.
fn placeholder(id: IdOf<Pane>) -> Pane {
    Pane {
        id,
        name: None,
        command: Vec::new(),
        cwd: String::new(),
        dir: None,
        title: None,
        right_click_passthrough: false,
        exit_behavior: Default::default(),
        scrollback_bytes: 0,
        status: PaneStatus::Running,
        program: Default::default(),
    }
}
