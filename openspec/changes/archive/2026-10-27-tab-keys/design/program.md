# Keyboard tab work in tabs mode: program

Status: Locked on 2026-10-27. Built on the locked `product.md` and `architecture.md`. Revised in Plannotator review, where Cyan asked about `row: u16` (kept, outside this change) and flagged the `collapse: bool` flag (now separate `collapse` and `expand`), then approved ("LGTM!").

## Rationale

Shape A needs two small pieces of code, both inside `ship-client`, plus bindings and docs:

- **A resolver for moves.** It turns a direction into the one `MoveTab` that KD-2 names, or `None` at an edge. `act` wraps its result in `Item::Move`, the item a mouse drop queues, so the re-check, the unfold and the quiet failure are the drop's own code.
- **A decision for `h`/`l`.** It says whether a press folds, unfolds or selects a tab. `act` folds through `sidebar.tabs` as it does today, and selects through the existing `select` with `Memory::landing`, the same way `j`/`k` do.

Neither piece adds a module. The move resolver sits next to `navigate` in `ui/mod.rs`, since it reads the tree and selection the same way. The fold check is one method on the sidebar's `Tabs`, which owns the fold set. The server, protocol, CLI, lane and drop code don't change.

This is a plan written in the document only. None of the code below exists in the working tree, and none of it has been compiled.

### Checked against `movable`

The architecture asked for each KD-2 target to be checked against `movable` (`ui/sidebar/tabs.rs:339`) at a sibling-list edge. The resolver only produces a target when the neighbor exists, so these are the cases where it does. Every one passes:

| Direction | Target | Why `movable` accepts it |
| --- | --- | --- |
| `up` from index `i` | `Before(sibling i-1)` | The target sits at `i-1`, so the no-op index is `i-2`. The tab is at `i`. |
| `down` from index `i` | `After(sibling i+1)` | The target sits at `i+1`, so the no-op index is `i+2`. The tab is at `i`. |
| `in` | `Parent(Some(sibling i-1))` | The tab's current parent is never its own sibling, so the "already last child" no-op can't apply. |
| `out` | `After(parent)` | The parent's siblings don't contain the tab, so there's no no-op index to hit. The parent is an ancestor, not inside the moved subtree. |

None of the targets is inside the moved tab's subtree, because siblings and ancestors never are. A `false` from `movable` therefore only means the tree changed between resolving and the re-check, which is the stale case the drop path already handles.

## Skeleton map

### Added

None. Every change lands in an existing file.

### Changed

#### `crates/ship-client/src/keymap.rs` (slice 2)

```rust
#[derive(Clone, Debug, Deserialize, strum::IntoStaticStr)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
#[strum(serialize_all = "snake_case")]
pub enum ClientTab {
    Next {},
    Prev {},
    Select { row: u16 },
    /// Unfold a folded row. On an open row, select its first child.
    Expand {},
    /// Fold an open row. On a folded row or a leaf, select the parent.
    Collapse {},
    /// One step among siblings or levels, through `server.tab.move`, resolved
    /// at its turn like a drop. Nothing at an edge.
    Move { direction: TabDirection },
}

/// Where `client.tab.move` takes the selected tab.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TabDirection {
    /// Just before the previous sibling.
    Up,
    /// Just after the next sibling.
    Down,
    /// Last child of the previous sibling.
    In,
    /// Just after the parent.
    Out,
}
```

This is a client-only enum, not `command::Direction`. `in` and `out` aren't screen directions, and nothing on the server reads it.

#### `crates/ship-client/src/ui/sidebar/tabs.rs` (slice 1)

```rust
impl Tabs {
    /// Whether this client folded `tab`'s row.
    pub fn folded(&self, tab: IdOf<Tab>) -> bool;
}
```

`movable` stays as it is. The table above records why it doesn't need to change.

#### `crates/ship-client/src/ui/mod.rs` (slices 1 and 2)

```rust
use crate::keymap::TabDirection; // added to the existing keymap import

/// What `client.tab.collapse` or `client.tab.expand` does on a tab's row.
enum Hop {
    /// Fold or unfold the row, keeping the selection. The value is what
    /// `sidebar.tabs.fold` takes.
    Fold { collapsed: bool },
    /// Select this tab where `Memory::landing` puts it.
    Select(IdOf<Tab>),
}

/// FR-001. Fold `tab`'s open row with children, else select its parent.
/// `None` on a top-level tab that's folded or has no children.
fn collapse(tabs: &Tabs, sidebar: &Sidebar, tab: IdOf<Tab>) -> Option<Hop>;

/// FR-002. Unfold `tab`'s folded row, else select its first child.
/// `None` on a tab with no children.
fn expand(tabs: &Tabs, sidebar: &Sidebar, tab: IdOf<Tab>) -> Option<Hop>;

/// FR-005 to FR-007. The one move that takes `tab` a step in `direction`,
/// counting siblings in the tree, folded or not. `None` at an edge.
fn shift(tabs: &Tabs, tab: IdOf<Tab>, direction: TabDirection) -> Option<MoveTab>;
```

These are the `act` arms they replace or add, shown as signatures and results rather than bodies:

```rust
// Replaces the shared arm at ui/mod.rs:813 (slice 1). Each key calls its
// own function; both results are applied the same way.
ClientAction::Tab(ClientTab::Collapse {}) => { /* viewed tab -> collapse(..) */ }
ClientAction::Tab(ClientTab::Expand {}) => { /* viewed tab -> expand(..) */ }
//   Some(Hop::Fold { collapsed }) -> self.sidebar.tabs.fold(tab, collapsed); Ok(())
//   Some(Hop::Select(tab))        -> return self.select(client, self.memory.landing(tab))
//   None                          -> Ok(())

// New arm (slice 2).
ClientAction::Tab(ClientTab::Move { direction }) => {
    // viewed tab of the selection -> shift(..)
    //   Some(to) -> return Turn::First(vec![Item::Move(TabMove { tab, to })])
    //   None     -> Ok(())   // FR-006: no request, no message
}
```

`shift` finds the tab's parent from `tree::path` and its neighbors from `tree::siblings` with `get_index_of`. `collapse` and `expand` read `tab.tabs` for children and the first child, `tree::path` for the parent, and `sidebar.tabs.folded` for the row. Neither one needs a new helper in `ship-core/src/tree.rs`.

`lane::TabMove` is `pub(super)`, and its fields are `pub`, so `ui/mod.rs` can already build one.

#### `crates/ship-core/src/defaults.toml` (slice 2)

Proposed tabs-mode keys. The `h` and `l` lines stay as they are:

```toml
[client.modes.tabs.keys]
"j"       = { action = { client.tab.next = {} } }
"k"       = { action = { client.tab.prev = {} } }
"h"       = { action = { client.tab.collapse = {} } }
"l"       = { action = { client.tab.expand = {} } }
"shift-k" = { action = { client.tab.move.direction = "up" }, label = "move up" }
"shift-j" = { action = { client.tab.move.direction = "down" }, label = "move down" }
"shift-l" = { action = { client.tab.move.direction = "in" }, label = "move in" }
"shift-h" = { action = { client.tab.move.direction = "out" }, label = "move out" }
"esc"     = { action = { client.mode = "normal" } }
```

#### Docs (slice 3)

- `docs/site/tabs.md:15`: the `alt-g` row gains `shift-j` / `shift-k` move, `shift-h` / `shift-l` move out / in, and `h` / `l` say they hop to the parent or first child.
- `docs/site/keys.md:31`: the tabs mode sentence says the same.

#### OpenSpec change artifacts (slices 1 and 2, plus attach)

- `openspec/changes/tab-keys/specs/session-observation/spec.md` (slice 1): MODIFIED "Local folding and selected-row visibility". The last sentence becomes "Folding SHALL NOT introduce a separate cursor. Only the fallbacks of `client.tab.collapse` and `client.tab.expand` SHALL change the selected tab." The "Tabs mode uses the selection" scenario gains the hop cases.
- `openspec/changes/tab-keys/specs/keymap/spec.md` (slice 2): MODIFIED the client-action requirement at `keymap/spec.md:61` to list `client.tab.move` with `client.pane.swap` and `client.pane.zoom` as actions that resolve locally and submit a server request, with no CLI form. ADDED a "Relative tab moves" requirement that carries FR-004 to FR-011, with the P1 and P2 scenarios.
- `proposal.md`, `design.md` (which points at the three papers) and `tasks.md`, one task group per slice. These were written when the paper locked.

### Moved

None.

### Untouched

- Server, protocol and CLI. `server.tab.move` and `ship tab move` stay as they are (non-goal).
- `ui/lane.rs`. `Item::Move` and `TabMove` are reused unchanged.
- The `Item::Move` turn at `ui/mod.rs:636`: re-check, unfold, send, and swallowing `not_found` and `invalid_structure`. This is the SC-004 guarantee, so it doesn't change.
- `navigate` and `j`/`k`.
- `leaves_zoom`. A tab move doesn't touch zoom, so moving a zoomed tab keeps it zoomed.
- `ui/keys.rs`. `client.tab.move` falls through to the queued lane path like every other `client.tab` action.
- Mouse drag and drop code and the tab menus (non-goal).

### Blocking unknowns

None.

## Build order

### 1. `h` and `l` hop between parent and children

- **Files:** `ui/sidebar/tabs.rs` (`folded`), `ui/mod.rs` (`Hop`, `collapse`, `expand`, the two arms), and the session-observation delta spec.
- **Delivers:** P3. `h` on a folded row or a leaf selects the parent at its landing. `l` on an open row selects the first child. Folding and unfolding behave as before.
- **Done when:**
  - `cargo build`, `cargo clippy --all-targets` and `cargo fmt --check` are clean.
  - In a running client, every P3 case gives the stated result on the product paper's example tree.
  - Every case from before the change (fold an open row, unfold a folded row) still behaves the same (SC-003).
  - `openspec validate tab-keys` passes.

### 2. `client.tab.move` and its bindings

- **Files:** `keymap.rs` (`ClientTab::Move`, `TabDirection`), `ui/mod.rs` (`shift`, the new arm), `defaults.toml`, and the keymap delta spec.
- **Delivers:** P1 and P2. `shift-k`/`shift-j`/`shift-l`/`shift-h` move the selected tab one step, the status line shows the four labels in tabs mode, and edges do nothing.
- **Done when:**
  - Build, Clippy and fmt are clean.
  - Every P1 case, checked with `ship tab list`, gives the stated tree, and the selection stays on the moved tab or pane (FR-010).
  - Every P2 case leaves `ship tab list` output byte-for-byte unchanged, with no status message (SC-002).
  - Pressing `shift-k` three times quickly moves the tab up three places (FR-008).
  - SC-001 is walked once: five flat tabs, reordered, nested two deep and un-nested, keys only.
  - For one `in` and one `out`, the same move made by mouse drop produces the same `ship tab list` output (SC-004).
  - `openspec validate tab-keys` passes.

### 3. Docs

- **Files:** `docs/site/tabs.md`, `docs/site/keys.md`.
- **Delivers:** FR-012.
- **Done when:** both pages name the new keys and the `h`/`l` fallbacks, following `docs/agents/docs.md`, and the pages stay within their scope.

All checks run on macOS, the machine this is built on. Linux is reported as unverified unless Cyan runs it there. Expected size: about 80 lines of implementation Rust and no test code, in line with `docs/agents/code.md`.

## Deviation log

- **2026-10-27, spec deltas split into smaller requirements.** `openspec validate --strict` flags requirement text over 500 characters, and no requirement in the synced specs is longer. So the session-observation delta keeps "Local folding and selected-row visibility" as a MODIFIED requirement and puts the fallbacks in a new "Parent and child hops" requirement. The keymap delta adds "Relative tab moves", "Tab moves at their turn" and "Default tabs keys" instead of one requirement. The content is FR-001 to FR-011 unchanged, so no paper reopens. The delta specs were written ahead of slices 1 and 2, at Cyan's request.
