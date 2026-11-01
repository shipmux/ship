//! Read-only lookups on the tab tree, shared by the server and clients.
//! Targets are found by traversal; there is no parent index to keep in sync.

use std::sync::Arc;

use indexmap::IndexMap;

use crate::{
    id::IdOf,
    model::{NodeId, Pane, Tab},
    prelude::*,
};

/// Child tabs in order, at every level including the top. `Arc` so a commit
/// copies only the tabs on an edited path.
pub type Tabs = IndexMap<IdOf<Tab>, Arc<Tab>>;

/// Ancestry of `node`, top-level tab first and `node` last. `None` if absent.
pub fn path(tabs: &Tabs, node: NodeId) -> Option<Vec<NodeId>> {
    match node {
        NodeId::Tab(tab) => {
            let mut path = Vec::new();
            tab_path(tabs, tab, &mut path).then(|| path.into_iter().map(NodeId::Tab).collect())
        }
        NodeId::Pane(pane) => {
            let mut path = path(tabs, NodeId::Tab(pane_owner(tabs, pane).ok()?))?;
            path.push(node);
            Some(path)
        }
    }
}

/// Pushes the IDs from `tabs` down to `id` onto `path`; false if absent.
fn tab_path(tabs: &Tabs, id: IdOf<Tab>, path: &mut Vec<IdOf<Tab>>) -> bool {
    for tab in tabs.values() {
        path.push(tab.id);
        if tab.id == id || tab_path(&tab.tabs, id, path) {
            return true;
        }
        path.pop();
    }
    false
}

pub fn tab(tabs: &Tabs, id: IdOf<Tab>) -> Result<&Tab> {
    fn find(tabs: &Tabs, id: IdOf<Tab>) -> Option<&Tab> {
        tabs.get(&id)
            .map(Arc::as_ref)
            .or_else(|| tabs.values().find_map(|tab| find(&tab.tabs, id)))
    }
    find(tabs, id).ok_or_else(|| not_found(NodeId::Tab(id)))
}

pub fn pane(tabs: &Tabs, id: IdOf<Pane>) -> Result<&Pane> {
    fn find(tabs: &Tabs, id: IdOf<Pane>) -> Option<&Pane> {
        tabs.values()
            .find_map(|tab| tab.pane(id).or_else(|| find(&tab.tabs, id)))
    }
    find(tabs, id).ok_or_else(|| not_found(NodeId::Pane(id)))
}

/// Every pane in the tree, in no particular order.
pub fn panes(tabs: &Tabs) -> impl Iterator<Item = &Pane> + '_ {
    fn walk(tabs: &Tabs) -> Box<dyn Iterator<Item = &Pane> + '_> {
        Box::new(
            tabs.values()
                .flat_map(|tab| tab.panes().chain(walk(&tab.tabs))),
        )
    }
    walk(tabs)
}

/// The tab that owns pane `id`.
pub fn pane_owner(tabs: &Tabs, id: IdOf<Pane>) -> Result<IdOf<Tab>> {
    pane_tab(tabs, id).map(|tab| tab.id)
}

/// The tab holding pane `id`.
pub fn pane_tab(tabs: &Tabs, id: IdOf<Pane>) -> Result<&Tab> {
    fn find(tabs: &Tabs, id: IdOf<Pane>) -> Option<&Tab> {
        tabs.values().find_map(|tab| match tab.pane(id) {
            Some(_) => Some(tab.as_ref()),
            None => find(&tab.tabs, id),
        })
    }
    find(tabs, id).ok_or_else(|| not_found(NodeId::Pane(id)))
}

/// The tab's parent, `None` at the top level or when absent.
pub fn parent(tabs: &Tabs, id: IdOf<Tab>) -> Option<IdOf<Tab>> {
    let mut path = Vec::new();
    tab_path(tabs, id, &mut path)
        .then(|| path.iter().rev().nth(1).copied())
        .flatten()
}

/// The tab itself, or a pane's tab. `None` for a node not in the tree.
pub fn viewed_tab(tabs: &Tabs, selection: NodeId) -> Option<IdOf<Tab>> {
    match selection {
        NodeId::Tab(tab) => Some(tab),
        NodeId::Pane(pane) => pane_owner(tabs, pane).ok(),
    }
}

/// The tabs `tab` sits among: its parent's children, or `tabs` at the top.
pub fn siblings(tabs: &Tabs, tab: IdOf<Tab>) -> Option<&Tabs> {
    if tabs.contains_key(&tab) {
        return Some(tabs);
    }
    tabs.values().find_map(|child| siblings(&child.tabs, tab))
}

pub fn not_found(node: NodeId) -> AppError {
    match node {
        NodeId::Tab(id) => err!(NotFound, "tab {} not found", id),
        NodeId::Pane(id) => err!(NotFound, "pane {} not found", id),
    }
}
