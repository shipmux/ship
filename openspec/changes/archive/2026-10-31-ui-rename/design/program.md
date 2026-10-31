# Rename tabs and panes from the UI, and a panes mode: program

Status: Locked on 2026-10-30. Built on the locked `product.md` and `architecture.md`. Drafting it narrowed the architecture's KD-5, which was re-opened and approved again first. Then approved in Plannotator review ("LGTM").

## Approved amendment: tabs shortcut

Before the slice-4 commit, Cyan approved replacing `alt-g` with `alt-t` in this worktree: `alt-t` enters tabs mode and `alt-g` is unbound by default, with no compatibility alias. This is a defaults-only change; mode behavior and input ordering stay the same. Update the current keymap and terminal-client deltas and the two guide key tables. Earlier `alt-g` examples in these papers now mean `alt-t`; historical verification records remain unchanged. This approval applies to the product, architecture and program papers; the remaining docs slice stays separate.

## Approved amendment: selection changes cancel rename

During slice 4, Cyan approved the behavior and the proposed extension to `Local::reconcile_rename` in chat ("alright go for it then, i'd ignore the lint for now"), re-approving this paper for the product and architecture amendment. Keep the fixed target, check for deletion first, then cancel without saving if this attachment's viewed tab or selected pane no longer matches it. Run this check before queued input as the implementation already does for deletion. Verify same-tab pane changes, other-client navigation, absent selection, deletion messages and the delayed input path. Cyan subsequently confirmed cancellation on automatic selection repair, superseding the old persistent rear-pane accent check. The existing selected-pane title accent covers the target while the field is open, so the separate `Scene::renaming` field and pane-title branch are unnecessary. Keep the sidebar's rename target input because its ordinary selected-row style is not the rename accent. Style the selected band's foreground directly; the amended invariant makes separate target styling through the recursive `items` function unnecessary. The skeleton below remains the original plan; its reconciliation behavior, separate pane drawing input and persistent rear-pane check are superseded.

## Summary

Four code slices and a docs slice. The first changes how the client reads input and ships on its own: keys and pastes wait in the lane unread, the menu opens at its turn, and mode, sidebar and reload actions lose their press-time path. Nothing about rename exists yet, and every existing workflow should behave the same on an idle lane. The second is panes mode, which is only `defaults.toml`. The third adds the rename field and its actions on top of slice 1, and the fourth adds the accent and the menu items.

Drafting this paper narrowed the architecture's KD-5, which was re-opened and approved again for that one decision. Only clicks close an overlay that opened before their turn; wheel turns, drags, motion for a program and border-drag positions that find one open are dropped.

This is a plan written in the document only. None of the code below exists in the working tree, and none of it has been compiled.

## Rationale

Shape D has two halves, and the skeleton keeps them apart:

- **Input at its turn** lives in `ui/keys.rs`, `ui/lane.rs` and `ui/mod.rs`. `Keys::handle` stops deciding when an action runs and only says what an event means. The lane gains `Item::Event` for an unread key, paste or overlay mouse event, and `Item::Menu` for a right-click. `Local::read` is the one interpretation function: the open overlay, else the mode machine. The lane's turn calls it, and so does disconnected input (KD-6), so there's no second copy. Press-time client actions become `Turn::Local`, which the loop runs with the terminal guard and config path that `Local::now` already takes.
- **The rename field** is one new module, `ui/rename.rs`, which owns the target, the `tui-input` buffer, the event handling and the field row. `Local::overlay` holds either it or a `Menu`. `act` opens it, a check after each publish closes it when its target goes, and drawing reads the placeholder from the replica each frame (KD-8).

Labels split into automatic and named halves in `ui/draw.rs`, so the placeholder and the drawn label come from the same function (KD-9).

## Skeleton map

### Added

#### `crates/ship-client/src/ui/rename.rs` (slice 3)

```rust
//! The rename field: one line on the mode bar's bottom row, for one tab or
//! pane. While open, it takes every terminal event (FR-006).

use crossterm::event::Event;
use ratatui::{Frame, layout::{Position, Rect}};
use ship_core::{
    command::{Command, PaneTarget, TabTarget, pane::{self, PaneCommand}, tab::{self, TabCommand}},
    id::IdOf,
    model::{Pane, Tab},
};
use tui_input::{Input, backend::crossterm::EventHandler};

use super::menu::Picked;
use crate::keymap::Action;

/// What the field renames, fixed when it opens (FR-002).
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
    /// Holds `stored`, cursor at the end, or empty (FR-005).
    pub fn open(target: Target, stored: Option<&str>) -> Self;

    pub fn target(&self) -> Target;

    /// Enter: `Picked::Run` with the set command (FR-010). Esc and a mouse
    /// press: `Picked::Close` (FR-008, FR-011). Paste: inserted with line
    /// breaks as spaces (FR-007). Other keys: `tui-input`. Mouse motion:
    /// ignored.
    pub fn handle(&mut self, event: &Event) -> Picked;

    /// The pill, the text or the dimmed `placeholder`, on `row`. Returns
    /// where the terminal cursor goes.
    pub fn draw(&self, frame: &mut Frame, row: Rect, placeholder: Option<&str>) -> Position;
}

/// Trimmed `text` as the name to set: `None` clears it (FR-010).
fn name(text: &str) -> Option<String>;

/// `server.tab.set` or `server.pane.set` naming `target` by ID, with only
/// `name` present, as the CLI's `--name` builds it.
fn set(target: Target, name: Option<String>) -> Action;
```

`Picked` stays in `ui/menu.rs` and both overlays return it. The pill reads `RENAME TAB` or `RENAME PANE` (FR-004).

### Changed

#### `Cargo.toml` and `crates/ship-client/Cargo.toml` (slice 3)

Proposed settings:

```toml
# workspace Cargo.toml, [workspace.dependencies]
tui-input = { version = "0.15", default-features = false, features = ["crossterm"] }

# crates/ship-client/Cargo.toml, [dependencies]
tui-input.workspace = true
```

Checked with `cargo info tui-input@0.15.5 -v`: the published crate depends on crossterm 0.29.0 and ratatui 0.30.2 (optional), with `unicode-segmentation` and `unicode-width` always on. That settles the architecture's risk about the published crate against its repository. Slice 3 still runs `cargo tree -d` to confirm one copy of crossterm, and `cargo tree -e features -i crossterm` to see whether tui-input's crossterm dependency turns on default features Ship leaves off.

#### `crates/ship-client/src/ui/keys.rs` (slice 1)

```rust
/// What a key or paste means in the active mode. When it runs is the
/// lane's business, not this one's.
pub(super) enum Handled {
    Action(Action),
    /// Unbound in normal: for the selected pane's program.
    Key(KeyEvent),
    Paste(String),
    /// Unbound outside normal, or not a key or paste.
    None,
}

impl Keys {
    pub fn handle(&mut self, event: Event) -> Handled; // body unchanged apart from the variants
}
```

Removed: `Handled::Queue`, `Handled::Now` and `fn handled`, the press-time split.

#### `crates/ship-client/src/ui/lane.rs` (slice 1)

```rust
pub(super) enum Item {
    Action(Action),
    /// A key, paste or overlay mouse event, read at its turn (KD-1, KD-5).
    Event(Event),
    /// Interpretation's results, for the selected pane's program.
    Key(KeyEvent),
    Paste(String),
    Select(NodeId),
    Fold(IdOf<Tab>),
    Mouse(InputFrame),
    Resize(EdgeAt),
    Move(TabMove),
    /// A right-click: the menu opens, and its selection is made, at its turn.
    Menu(menu::Target, Position),
}

pub(super) enum Turn<'a> {
    Done,
    Wait(Request<'a>),
    First(Vec<Item>),
    Detach,
    /// Mode, sidebar and reload actions, run by the loop with the terminal
    /// guard, config path and width that `Local::now` takes.
    Local(ClientAction),
}
```

#### `crates/ship-client/src/ui/mod.rs` (slices 1 and 3)

```rust
mod rename; // slice 3

/// At most one, checked before the mode machine (KD-7).
enum Overlay {
    Menu(Menu),
    Rename(Rename), // slice 3
}

struct Local<'a> {
    // `menu: Option<Menu>` becomes:
    overlay: Option<Overlay>,
    // other fields unchanged
}

struct Scene<'a> {
    // added in slice 4, for the title row's accent:
    renaming: Option<IdOf<Pane>>,
    // other fields unchanged
}

impl<'a> Local<'a> {
    /// The one interpretation of a key, paste or overlay mouse event: the open
    /// overlay, else `Keys::handle`. A picked or bound action clears the last
    /// message, as a queued action does today.
    fn read(&mut self, event: Event) -> Handled;

    /// KD-6: input while disconnected, read at once. Mode, sidebar and reload
    /// actions apply, detach exits, and everything else is dropped, including
    /// a rename's Enter, which has already closed the field (FR-014).
    fn offline(&mut self, event: Event, guard: &mut TerminalGuard, config: &Path, cols: u16)
        -> Option<Exit>;

    /// Now reached through `Turn::Local` instead of `Handled::Now`. Body unchanged.
    fn now(&mut self, action: ClientAction, guard: &mut TerminalGuard, config: &Path, cols: u16)
        -> Result<()>;

    /// Gains `screen` for opening a menu at its turn.
    fn turn(&mut self, client: &'a Client, item: Item, screen: Rect) -> Turn<'a>;

    /// The `Item::Select` arm's body, shared with `Item::Menu`, which makes its
    /// selection itself instead of queueing one that would close the menu.
    fn land(&mut self, client: &'a Client, node: NodeId) -> Turn<'a>;

    /// FR-012, after each publish: a field whose target is gone closes with
    /// `tab closed` or `pane closed`. True when it closed.
    fn reconcile_rename(&mut self) -> bool; // slice 3
}
```

How the turn arms change, shown as results rather than bodies:

```rust
// slice 1
Item::Event(event) => match self.read(event) {
    Handled::Action(action) => Turn::First(vec![Item::Action(action)]),
    Handled::Key(key)       => Turn::First(vec![Item::Key(key)]),
    Handled::Paste(text)    => Turn::First(vec![Item::Paste(text)]),
    Handled::None           => Turn::Done,
},
Item::Menu(target, at) => { /* close any overlay, open Menu at `at` within `screen`,
                               then self.land(client, node) for a pane or tab target */ }
Item::Select(_) | Item::Fold(_) | Item::Move(_) => { /* close any open overlay, then as today (KD-5) */ }
Item::Mouse(_) | Item::Resize(_) if self.overlay.is_some() => Turn::Done, // KD-5: dropped

// act, slice 1: replaces the `unreachable!` for press-time actions
ClientAction::Mode(_) | ClientAction::Sidebar(_) | ClientAction::Config(_) => return Turn::Local(action),

// act, slice 3
ClientAction::Tab(ClientTab::Rename {}) => { /* viewed tab of the selection, else nothing (FR-001);
                                              overlay = Rename::open(Target::Tab(id), tab.name) */ }
ClientAction::Pane(ClientPane::Rename {}) => { /* selected pane, else nothing */ }
```

The event loop, as a diff of what it does:

```diff
 terminal key or paste
-  overlay open? menu.handle, else keys.handle → Handled::{Queue, Now, None}
-  Queue → lane.push(item);  Now → local.now(..);  disconnected → only detach
+  connected    → lane.push(Item::Event(event))
+  disconnected → local.offline(event, ..)

 terminal mouse
-  menu open → menu.handle
+  overlay open → lane.push(Item::Event(event))          (read at its turn, KD-5)
   no overlay   → Mouse::route at press, as today
-    Clicked::Menu → Menu::open now, lane.push(Item::Select(node))
+    Clicked::Menu → lane.push(Item::Menu(target, at))

 terminal resize
-  menu = None
+  close a Menu overlay; a Rename stays (KD-12)

 lane turn
+  Turn::Local(action) → local.now(action, guard, config, cols)
+  visible: also for Item::Event when an overlay was open before or after
+           the turn, or the shown mode changed (a one-shot mode's return)

 after the turns
   sidebar.tabs.reconcile, mouse.reconcile
+  reconcile_rename                                       (slice 3)
```

`Local::cut` keeps closing a `Menu` overlay and leaves a `Rename` open (KD-12). `Local::reload` closes either. The automatic reload on a config save stays immediate: it isn't user input, so it isn't ordered with keys.

#### `crates/ship-client/src/ui/draw.rs` (slices 3 and 4)

```rust
/// The program's title, else its command's file name. The pane's label
/// without a name, and its rename placeholder.
pub(super) fn pane_auto_label(pane: &Pane) -> &str;

/// The pane's name, else `pane_auto_label`.
pub(super) fn pane_label(pane: &Pane) -> &str;

/// The first pane's directory's last component, else that pane's label,
/// else `position + 1`. The tab's label without a name.
pub(super) fn tab_auto_label(tab: &Tab, position: usize) -> String;

/// The tab's name, else `tab_auto_label`.
pub(super) fn tab_label(tab: &Tab, position: usize) -> String;

/// The placeholder for an open field, looked up in the replica each frame
/// (KD-8). `None` once the target is gone.
fn placeholder(observer: &Observer, target: rename::Target) -> Option<String>;
```

In `draw` (slice 3), a `Rename` overlay replaces `status_line` with the field on the pane area's bottom row and sets the cursor there (KD-11). A `Menu` overlay draws last, as now. Slice 4 passes the target to the sidebar and to `Scene::renaming`.

#### `crates/ship-client/src/ui/sidebar/mod.rs` and `sidebar/tabs.rs` (slice 4)

```rust
impl Sidebar {
    pub fn draw(&mut self, frame: &mut Frame, area: Rect, observer: &Observer,
        attention: &Attention, rows: &[Vec<Token>], gap: Option<&Gap>,
        renaming: Option<IdOf<Tab>>);
}

impl Tabs {
    pub fn draw(&mut self, frame: &mut Frame, area: Rect, tabs: &TabTree,
        selection: Option<NodeId>, gap: Option<&Gap>, attention: &Attention,
        renaming: Option<IdOf<Tab>>);
}
```

`items` styles the `renaming` row with the accent (FR-009).

#### `crates/ship-client/src/ui/panes.rs` (slice 4)

The title row gives `scene.renaming`'s span the accent, whether or not it's the stack's front pane. A lone pane has no title row, so only the field shows it.

#### `crates/ship-client/src/keymap.rs` (slice 3)

```rust
pub enum ClientTab {
    // existing variants unchanged
    /// Open the rename field for the selected tab, or the selected pane's tab,
    /// resolved at its turn.
    Rename {},
}

pub enum ClientPane {
    // existing variants unchanged
    /// Open the rename field for the selected pane, resolved at its turn.
    Rename {},
}
```

`Action::label` gives both `rename` through strum, so no hint label is needed.

#### `crates/ship-core/src/defaults.toml` (slices 2, 3 and 4)

Slice 2, panes mode and normal mode (FR-016, FR-017):

```toml
# [client.modes.normal.keys]: the four alt-shift-h/j/k/l lines go, and:
"alt-p"       = { action = { client.mode = "panes" } }

[client.modes.panes]
kind = "sticky"

[client.modes.panes.keys]
"h"       = { action = { client.pane.focus.direction = "left" } }
"j"       = { action = { client.pane.focus.direction = "down" } }
"k"       = { action = { client.pane.focus.direction = "up" } }
"l"       = { action = { client.pane.focus.direction = "right" } }
"shift-h" = { action = { client.pane.swap.direction = "left" }, label = "swap left" }
"shift-j" = { action = { client.pane.swap.direction = "down" }, label = "swap down" }
"shift-k" = { action = { client.pane.swap.direction = "up" }, label = "swap up" }
"shift-l" = { action = { client.pane.swap.direction = "right" }, label = "swap right" }
"."       = { action = { server.pane.cycle = { direction = "next" } } }
","       = { action = { server.pane.cycle = { direction = "prev" } } }
"esc"     = { action = { client.mode = "normal" } }
```

Slice 3 adds `"r" = { action = { client.pane.rename = {} } }` before panes mode's `esc`, and `"r" = { action = { client.tab.rename = {} } }` before tabs mode's `esc` (FR-018). Slice 4 adds `{ label = "rename", action = { client.pane.rename = {} } }` to the pane menu and `{ label = "rename", action = { client.tab.rename = {} } }` to the tab menu, each just before `close` (FR-019).

#### Docs (slice 5)

`docs/site/keys.md`, `docs/site/tabs.md` and `docs/site/panes.md`: panes mode, `alt-p`, renaming from tabs mode, panes mode and the menus, and the removal of `alt-shift-hjkl` from normal mode (FR-020).

#### OpenSpec change artifacts

- `specs/keymap/spec.md`:
  - Slice 1: MODIFIED "Key actions act on the selection" and "Sidebar resize availability" to drop the press-time exceptions.
  - Slice 2: MODIFIED "Default pane keys" for the normal-mode change, and ADDED "Default panes keys".
  - Slice 3: MODIFIED "Client actions" to list `client.tab.rename` and `client.pane.rename`, and "Default tabs keys" for `r`.
- `specs/terminal-client/spec.md`:
  - Slice 1: MODIFIED "Ordered input": a key's meaning is decided at its turn, with the delayed `alt-g j` scenario.
  - Slice 3: ADDED requirements for the rename field (FR-004 to FR-015), with the P1, P2 and P4 scenarios.
- `specs/context-menus/spec.md`:
  - Slice 1: MODIFIED "Opening a menu", "Selection when a menu opens" and "Menu items run as key actions" for opening and picking at the turn. MODIFIED "Using an open menu", where "At most one menu" becomes at most one overlay.
  - Slice 4: MODIFIED "Default pane menu" and "Default tab menu".
- `specs/mouse-input/spec.md` (slice 1): MODIFIED "Clicks keep their target" for clicks that find an overlay open at their turn.
- `proposal.md`, `design.md` (pointing at the three papers) and `tasks.md`, one task group per slice. These are written once this paper locks.

### Moved

None.

### Untouched

- The server, protocol and CLI (product non-goal). Renames use `server.tab.set` and `server.pane.set` as they are.
- `Mouse::route` and its busy-lane rule (`ui/mouse.rs:307`), apart from `Clicked::Menu` now becoming a lane item in the loop (KD-4).
- `ui/menu.rs`: `Menu`, `Picked`, `Target` and the hardcoded menu keys are unchanged. Only who opens it and when changes.
- Sidebar width, split and scroll drags, which apply at press to client-local state, as today.
- The `Item::Key`, `Item::Paste`, `Item::Select`, `Item::Fold`, `Item::Move` and `Item::Resize` turn bodies, apart from closing an open overlay first.
- `#61` (`ship skill`). It joins this pull request through its own paper, not this one.

### Blocking unknowns

None. The one found while drafting, whether routed mouse input that isn't a click closes an overlay, is settled by the narrowed KD-5 in `architecture.md`: clicks close it and run, and program mouse input and border-drag positions are dropped.

## Build order

### 1. Input is read at its turn

- **Files:**
  - `ui/keys.rs`, `ui/lane.rs`, and `ui/mod.rs` (`Overlay` with `Menu` only, `read`, `offline`, `land`, `Turn::Local`, the event loop changes, `Item::Menu`).
  - The slice-1 deltas in the keymap, terminal-client, context-menus and mouse-input specs.
- **Delivers:** KD-1 and KD-3 to KD-6. Every key, paste and right-click is read against the state left by the input before it. Mode, sidebar and reload actions run at their turn. Nothing rename-related exists yet.
- **Done when:**
  - `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo build` pass.
  - With a temporary delay proxy of 500 ms each way, as earlier changes used:
    - `alt-g j` selects the next tab, ten runs of ten.
    - `alt-g` with keys still waiting shows the tabs-mode bar only after those keys have landed.
    - A right-click, then `j` and Enter, runs the menu's second item.
    - Keys typed before a right-click reach the pane, not the menu.
    - A wheel turn over a mouse-tracking program, queued just after `alt-g` and a right-click, is dropped and leaves the menu open.
    - The existing "Type right after a new tab" and "Zoom what was just split" scenarios still pass.
  - On an idle lane, mode switches, sidebar toggle and resize, reload, menus, clicks, border drags and tab drops behave as before, by feel and by `ship tab list`.
  - While the server is stopped: `alt-g` still switches mode, `alt-q` detaches, and other keys do nothing.
  - `openspec validate ui-rename --type change --strict` passes.

### 2. Panes mode

- **Files:** `defaults.toml` (slice-2 keys), the slice-2 keymap delta.
- **Delivers:** P3 and FR-016 and FR-017.
- **Done when:**
  - Every P3 case gives the stated result, checked with `ship tab get`.
  - The mode bar shows the panes-mode hints with `swap left` and the other three swap labels.
  - In normal mode, `alt-shift-l` reaches a program that prints its input.
  - SC-005 is walked once.
  - Checks as in slice 1.

### 3. The rename field

- **Files:**
  - `Cargo.toml` and `crates/ship-client/Cargo.toml` (`tui-input`).
  - `ui/rename.rs`.
  - `ui/mod.rs`: `Overlay::Rename`, the two `act` arms, `reconcile_rename`, and keeping the field on resize and cut.
  - `keymap.rs` (`Rename` on both enums).
  - `draw.rs` (split labels, `placeholder`, the field row and cursor).
  - `defaults.toml` (both `r` keys).
  - The slice-3 keymap and terminal-client deltas.
- **Delivers:** P1, P2's keyboard cases and P4. FR-001 to FR-008 and FR-010 to FR-015.
- **Done when:**
  - `cargo tree -d` shows one crossterm and one ratatui.
  - Every P1 and P4 case and P2's first two cases give the stated result, read back with `ship tab get` and `ship pane get`.
  - Esc, a click and a closed target leave `ship tab list` output byte-for-byte unchanged (SC-003).
  - A second client sees no change until Enter (SC-004).
  - Through the 500 ms delay proxy, `alt-p r foo` Enter names the pane `foo`, and nothing reaches its program.
  - A title change during an edit shows in the placeholder at once.
  - Pasting two lines inserts one line with a space.
  - Enter after the server stops closes the field and sends nothing.
  - A resize keeps the field.
  - Checks as in slice 1.

### 4. Accent and menu items

- **Files:** `sidebar/mod.rs`, `sidebar/tabs.rs`, `panes.rs`, `Scene::renaming`, `defaults.toml` (menu items), the slice-4 context-menus delta.
- **Delivers:** FR-009 and FR-019, and P2's menu cases.
- **Done when:**
  - While the field is open, the target's sidebar row or title row takes the accent, including a pane behind its stack's front.
  - `rename` in each menu opens the field for what was right-clicked.
  - SC-001 is walked once with keys only.
  - Checks as in slice 1.

### 5. Docs

- **Files:** `docs/site/keys.md`, `docs/site/tabs.md`, `docs/site/panes.md`.
- **Delivers:** FR-020.
- **Done when:**
  - The three pages match `defaults.toml`, following `docs/agents/docs.md`, and stay within their scopes.
  - The site builds.

All checks run on macOS, the machine this is built on. Linux is unverified unless Cyan runs it there. Expected size, estimated and not measured: about 90 lines of implementation Rust for `rename.rs`, 40 to 60 net in `mod.rs`, `keys.rs` and `lane.rs` together, and 30 across drawing, labels and `keymap.rs`. That's roughly 160 to 180 lines, with no test code, as `docs/agents/code.md` asks.

## Deviation log

- **Docs completion and archive.** Cyan approved finishing the last docs slice before syncing, archiving, committing and opening the PR. Task 5.1 updates the three existing guide pages; their key and pane scope comments now include panes mode and renaming. Correct the remaining disconnected-mode example to `alt-t`. Cyan also explicitly approved shortening `Client actions` and `Ordered input` to fit strict validation without changing behavior or scenarios. The synced main specs retain every prior scenario and untouched requirement. Track the two approved sidebar argument-count lint exemptions in [#98](https://github.com/shipmux/ship/issues/98); no drawing-interface redesign is selected by this change.

- **Pre-commit tabs shortcut.** Cyan approved replacing `alt-g` with `alt-t` in the default normal-mode bindings, without an alias, and updating `docs/site/keys.md`, `docs/site/tabs.md` and the current change records. The keymap delta now modifies `Default view keys`; current delta scenarios use `alt-t`, while earlier evidence is retained as history. Verify normal-mode `alt-g` passthrough, `alt-t` navigation and rename, and delayed input ordering. This does not complete the remaining docs slice.

- **Slice 4, approved selection cancellation.** Cyan approved cancelling a rename when this attachment moves away from its target, superseding FR-002's selection-change clause. The product and architecture amendments above carry the same approval. Extend the existing `reconcile_rename` only; retain fixed IDs for submission and existing deletion feedback. Add corresponding terminal-client delta scenarios and runtime probes. The live stack check established that selecting a different stack member repairs every viewer of that slot to its new front; Cyan explicitly chose cancellation on that repair. Replace the old persistent rear-pane check with this cancellation check and reuse the existing selected-pane accent instead of adding `Scene::renaming`. Cyan also approved ignoring the argument-count lint for now: local documented `#[expect(clippy::too_many_arguments)]` attributes on the two sidebar draw functions preserve the approved signatures without suppressing other warnings.

- **2026-10-30, delta specs written ahead of the slices.** Cyan asked for the OpenSpec artifacts before implementation, so all four deltas were written at once instead of slice by slice. They carry FR-001 to FR-019 and the architecture's KD-1 to KD-6 and KD-12 unchanged, so no paper re-opens. Differences from the skeleton map's list:
  - `context-menus`: "Opening a menu" didn't need a change; opening at the turn is stated in "Selection when a menu opens".
  - `terminal-client`: the rename field is split into six requirements to stay within strict validation's 500-character limit, and "Input while disconnected" is added for KD-6.
  - `keymap`: "Modes" is modified to list `panes` among the default modes, and "Rename actions" is added for FR-001 to FR-003.
  - `mouse-input`: "Clicks keep their target" stays as it is, and the narrowed KD-5 is a new requirement, "Mouse input queued before an overlay".
