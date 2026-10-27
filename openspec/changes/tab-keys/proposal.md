# Proposal

Status: Locked through its design papers. `design/product.md`, `design/architecture.md` and `design/program.md` are locked with Cyan's recorded approval, and they are the contract. This proposal and the deltas under `specs/` are views assembled from them.

## Why

Without a mouse, tabs mode can walk the tab tree and fold rows, but it can't move a tab or jump between a parent and its children. Reordering needs a drag or `ship tab move` with IDs ([#83](https://github.com/shipmux/ship/issues/83), part of milestone 5).

## What Changes

- **`h` and `l` hop.** `client.tab.collapse` on a folded row or a leaf selects the parent. `client.tab.expand` on an open row selects its first child. Both land where `j`/`k` would. Folding and unfolding are unchanged.
- **New `client.tab.move` action** with a required `direction` of `up`, `down`, `in` or `out`. It moves the selected tab one step among its siblings, into the sibling above as its last child, or out to just after its parent. At an edge it does nothing.
- **Tabs-mode bindings** `shift-k`, `shift-j`, `shift-l` and `shift-h`, labeled `move up`, `move down`, `move in` and `move out`.

Not in this change: normal-mode bindings, wrapping at edges, multi-step or absolute moves, Org-style outdenting, a CLI form, menu items, and any server or protocol change.

## Capabilities

### Modified Capabilities

- `session-observation`: Local folding allows the `h`/`l` fallbacks to change the selected tab.
- `keymap`: `client.tab.move` joins the client actions that resolve locally and submit a server request, and a new requirement covers relative tab moves and their tabs-mode bindings.

## Impact

- `ship-client`: `ClientTab::Move` and `TabDirection` in `keymap.rs`; `Hop`, `collapse`, `expand` and `shift` in `ui/mod.rs`; `Tabs::folded` in `ui/sidebar/tabs.rs`.
- `ship-core`: four bindings in `defaults.toml`.
- Docs: `docs/site/tabs.md` and `docs/site/keys.md`.
- Size: about 80 implementation lines, estimated, not measured. No tests are planned.
