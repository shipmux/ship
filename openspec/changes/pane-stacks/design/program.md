# Stacked panes: program

Status: Locked on 2026-10-29. Drafted 2026-10-28, revised 2026-10-29 against `docs/agents/code.md`: cycle takes `direction`, `Slot::take` returns a named `Taken`, `layout::close` folds into `take`, and the move body reuses create's `at` and `place`. Built on `product.md` and `architecture.md`, both re-locked with it. Approved in Plannotator review ("LGTM!") and locked in chat ("lock it!").

## Rationale

Shape A puts a stack of panes in each layout leaf. In code that means one new type and a set of edits that keep it consistent:

- **`ship-core/src/layout.rs` owns the stack.** `Slot` wraps the `VecDeque1<Pane>` privately and exposes the few operations the server needs: stack, cycle, bring to front, take out. Every mitsein call sits in this one file, so the `remove_if_many` trap (architecture, risks) has exactly one place to get it right. The serde compatibility for old `"pane"` leaves lives here too.
- **`ship-server/src/state/layout.rs` owns the tree and zoom.** It finds the slot, calls the `Slot` method, collapses a split when a slot empties, and points `tab.zoomed` at the zoomed slot's new front. It's still the only code that changes a tab's layout.
- **`ship-server/src/state.rs` owns selection.** `repair` gains the "never behind the front" rule, the select handler brings a behind pane forward, and two new handlers (`Cycle`, `PaneMove`) commit through the existing `commit`.
- **`ship-server/src/state/geometry.rs`** emits one entry per slot, for its front, and takes a title row out of a borderless stacked slot.
- **`ship-client`** maps the new commands (`execute.rs`, `api.rs`), draws titles from the layout and turns title clicks into selects (`ui/panes.rs`, `ui/mouse.rs`), and runs edit-scrollback (`automation.rs`).

One small move from the architecture paper: decision 1 and the structure block put stack-order edits in `state/layout.rs`. Here the order edits are methods on `Slot` in `ship-core`, called only from `state/layout.rs`. The reason is that the stack field can then stay private, and only `layout.rs` touches mitsein. Nothing about who decides what changes. This is flagged for review.

This is a plan in the document only. None of the code below exists in the working tree, and none of it has been compiled.

## Skeleton map

### Added

None. Every change lands in an existing file.

### Changed

#### `Cargo.toml` and crate manifests (slices 1 and 5)

Proposed settings:

```toml
# Cargo.toml [workspace.dependencies] (slice 1)
mitsein = { version = "0.9", features = ["serde"] }

# crates/ship-core/Cargo.toml (slice 1)
mitsein.workspace = true

# crates/ship-client/Cargo.toml (slice 5)
etcetera.workspace = true
tempfile.workspace = true
tokio = { workspace = true, features = ["process"] }
```

`ship-server` doesn't name mitsein: it only calls `Slot` methods.

#### `crates/ship-core/src/layout.rs` (slice 1; stack edits in slice 2)

```diff
 #[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
-#[serde(tag = "kind", rename_all = "snake_case")]
+#[serde(tag = "kind", rename_all = "snake_case", from = "Repr")]
 #[expect(clippy::large_enum_variant, reason = "...")]
 pub enum Layout {
-    Pane(Pane),
+    Slot(Slot),
     Split(Split),
 }
```

```rust
use std::num::NonZeroUsize;

use mitsein::vec_deque1::VecDeque1;

/// A place in the layout holding one or more panes. Only the front is drawn.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct Slot {
    /// Front first. Never empty.
    #[schema(value_type = Vec<Pane>)]
    panes: VecDeque1<Pane>,
}

/// Which way `Slot::cycle` turns: `Next` makes [X, A, B] into [A, B, X].
#[derive(Clone, Copy, Debug, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[serde(rename_all = "snake_case")]
pub enum CycleDirection {
    Next,
    Prev,
}

impl Slot {
    pub fn new(pane: Pane) -> Self;
    pub fn front(&self) -> &Pane;
    /// Front first.
    pub fn panes(&self) -> impl Iterator<Item = &Pane>;
    pub fn panes_mut(&mut self) -> impl Iterator<Item = &mut Pane>;
    pub fn len(&self) -> NonZeroUsize;
    pub fn contains(&self, pane: IdOf<Pane>) -> bool;
    /// The pane directly behind `pane`, the edit-scrollback target.
    pub fn behind(&self, pane: IdOf<Pane>) -> Option<&Pane>;

    // Edits. Called only from ship-server's state/layout.rs.
    pub fn push_front(&mut self, pane: Pane);
    pub fn cycle(&mut self, direction: CycleDirection);
    /// Rotates until `pane` is in front, keeping the others' order. False
    /// when `pane` isn't here.
    pub fn bring_to_front(&mut self, pane: IdOf<Pane>) -> bool;
    /// Takes `pane` out, or gives the slot back unchanged as `Err` when
    /// `pane` isn't here. The only removal path: every `remove_if_many`
    /// result is consumed in here.
    pub fn take(self, pane: IdOf<Pane>) -> Result<Taken, Slot>;
}

/// What `Slot::take` leaves.
pub enum Taken {
    /// Others remain; `rest` is the slot without the pane.
    Pane { pane: Pane, rest: Slot },
    /// It was the only pane, so the slot is gone.
    Last(Pane),
}

/// Reads the pre-stack `{"kind": "pane", ...}` leaf as a slot of one.
/// Nothing writes it.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[expect(clippy::large_enum_variant, reason = "mirrors Layout")]
enum Repr {
    Pane(Pane),
    Slot(Slot),
    Split(Split),
}

impl From<Repr> for Layout;

impl Layout {
    /// Every pane, slot by slot in layout order, each slot front first.
    pub fn panes(&self) -> Box<dyn Iterator<Item = &Pane> + '_>;
    pub fn panes_mut(&mut self) -> Box<dyn Iterator<Item = &mut Pane> + '_>;
    pub fn pane(&self, id: IdOf<Pane>) -> Option<&Pane>;
    pub fn pane_mut(&mut self, id: IdOf<Pane>) -> Option<&mut Pane>;
    /// Slots in layout order: first child before second.
    pub fn slots(&self) -> Box<dyn Iterator<Item = &Slot> + '_>;
    /// The slot holding `pane`.
    pub fn slot(&self, pane: IdOf<Pane>) -> Option<&Slot>;
    pub fn slot_mut(&mut self, pane: IdOf<Pane>) -> Option<&mut Slot>;
}
```

`CycleDirection` lives here rather than in `protocol.rs` because `Slot::cycle` takes it. `protocol.rs` reuses it, the same way `protocol.rs` already reuses `command::Direction`.

#### `crates/ship-core/src/model.rs` (slice 1)

```rust
impl Tab {
    // panes, panes_mut, pane, pane_mut unchanged: they forward to Layout.
    pub fn slot(&self, pane: IdOf<Pane>) -> Option<&Slot>;
}
```

#### `crates/ship-core/src/protocol.rs` (slice 1 for the version; 2 and 4 for the bodies)

```diff
-pub const PROTOCOL_VERSION: u32 = 19;
+pub const PROTOCOL_VERSION: u32 = 20;

 pub struct CreatePane {
     pub at: PaneAt,
-    #[serde(default)]
-    pub direction: SplitDirection,
+    #[serde(default)]
+    pub place: Place,
     #[serde(default)]
     pub pane: NewPane,
     pub select: Option<IdOf<Attachment>>,
 }
```

```rust
/// Where a created pane goes relative to its anchor. `{"split": "right"}`,
/// `{"split": "down"}` or `"stack"`.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Place {
    /// Split the anchor's slot, the new pane alone on that side.
    Split(SplitDirection),
    /// In front of the anchor's slot.
    Stack,
}

impl Default for Place; // Split(Right)

/// POST /panes/{id}/cycle body. (slice 2)
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct CyclePane {
    pub direction: CycleDirection,
}

/// POST /panes/{id}/move body. Places the pane relative to `at` the way
/// `CreatePane` places a new one: `"stack"` is `--onto`, `{"split": "right"}`
/// is `--beside --direction right`. (slice 4)
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct MovePane {
    pub at: IdOf<Pane>,
    pub place: Place,
}
```

`at` is a pane, not `PaneAt`, because moving into an empty tab is a non-goal. `place` has no default here: a move names where it goes.

`docs/agents/code.md` lists `direction` among create's request fields. Slice 2 changes it to `place`.

#### `crates/ship-core/src/command.rs` (slices 2, 4 and 5)

```diff
 pub enum PaneCommand {
     Create(Create),
     ...
     Swap(Swap),
+    /// Rotate a pane's stack one step and print its tab as JSON
+    Cycle(Cycle),
+    /// Move a pane onto or beside another pane, in any tab, and print the
+    /// destination tab as JSON
+    #[serde(skip)]
+    Move(Move),
+    /// Open the scrollback of the pane behind this one in your editor
+    #[serde(skip)]
+    EditScrollback(EditScrollback),
 }

 pub struct Create {
     pub tab: TabTarget,
-    /// Which side of the anchor to split; right by default
-    #[cfg_attr(feature = "clap", arg(long))]
-    pub direction: Option<SplitDirection>,
+    /// Split the anchor to the right or down, or stack in front of it;
+    /// right by default
+    #[cfg_attr(feature = "clap", arg(long, visible_alias = "direction"))]
+    #[serde(alias = "direction")]
+    pub place: Option<Placement>,
     ...
 }
```

```rust
/// The flat form of `protocol::Place` for flags and bindings.
#[derive(Clone, Copy, Debug, Deserialize)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[serde(rename_all = "snake_case")]
pub enum Placement {
    Right,
    Down,
    Stack,
}

impl From<Placement> for Place;

#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "clap", derive(Args))]
#[serde(deny_unknown_fields)]
pub struct Cycle {
    #[cfg_attr(feature = "clap", command(flatten))]
    #[serde(default)]
    pub pane: PaneTarget,
    /// next brings the pane behind the front forward; prev brings the back
    /// pane forward
    #[cfg_attr(feature = "clap", arg(long))]
    pub direction: CycleDirection,
}

/// CLI only: a key can't name the target pane.
#[derive(Clone, Debug, Default, Deserialize)]
#[cfg_attr(feature = "clap", derive(Args))]
#[cfg_attr(feature = "clap", command(group(
    clap::ArgGroup::new("to").required(true).args(["onto", "beside"])
)))]
pub struct Move {
    #[cfg_attr(feature = "clap", command(flatten))]
    pub pane: PaneTarget,
    /// Put the pane in front of this pane's stack
    #[cfg_attr(feature = "clap", arg(long, value_name = "ID"))]
    pub onto: Option<IdOf<Pane>>,
    /// Split this pane's slot and put the pane alone on the --direction side
    #[cfg_attr(feature = "clap", arg(long, value_name = "ID", requires = "direction"))]
    pub beside: Option<IdOf<Pane>>,
    /// With --beside: which side
    #[cfg_attr(feature = "clap", arg(long, requires = "beside"))]
    pub direction: Option<SplitDirection>,
}
// execute.rs maps --onto Y to MovePane { at: Y, place: Stack } and
// --beside Y --direction D to MovePane { at: Y, place: Split(D) }.

/// CLI only; keys run it through `SHIP_BIN` in a stacked pane.
#[derive(Clone, Debug, Default, Deserialize)]
#[cfg_attr(feature = "clap", derive(Args))]
pub struct EditScrollback {
    /// Pane to edit; defaults to the pane directly behind this one
    #[cfg_attr(feature = "clap", arg(long, value_name = "ID"))]
    pub pane: Option<IdOf<Pane>>,
    /// The pane this runs in
    #[cfg_attr(feature = "clap", arg(long = "from", env = "SHIP_PANE_ID", hide = true))]
    pub from: Option<IdOf<Pane>>,
}
```

`EditScrollback` doesn't flatten `PaneTarget`, because `PaneTarget` fills `--pane` from `SHIP_PANE_ID`, and edit-scrollback has to tell "named a pane" apart from "ran in a pane".

#### `crates/ship-server/src/state/layout.rs` (slice 1; stack, cycle and zoom rules in slice 2; move in slice 4)

```rust
use ship_core::layout::{CycleDirection, Slot};

/// Splits `anchor`'s slot, putting `pane` alone in the new slot on the
/// `direction` side. Clears zoom. Unchanged signature.
pub(crate) fn split(tab: &mut Tab, anchor: Option<IdOf<Pane>>, direction: SplitDirection, pane: Pane) -> Result<()>;

/// Puts `pane` in front of `anchor`'s slot, or as the first slot of an
/// empty tab. If that slot is zoomed, zoom moves to `pane`. (slice 2)
pub(crate) fn stack(tab: &mut Tab, anchor: Option<IdOf<Pane>>, pane: Pane) -> Result<()>;

/// Rotates `pane`'s slot one step. Zoom follows the slot's front. (slice 2)
pub(crate) fn cycle(tab: &mut Tab, pane: IdOf<Pane>, direction: CycleDirection) -> Result<()>;

/// Brings `pane` to its slot's front. Zoom follows. No change when it's
/// already in front. (slice 2)
pub(crate) fn bring_to_front(tab: &mut Tab, pane: IdOf<Pane>) -> Result<()>;

/// Takes `id` out of its slot. A slot left empty is removed and its split
/// collapses to the sibling. Zoom stays, retargets to the new front when
/// the zoomed front leaves, and clears only when the zoomed slot is
/// removed. The one removal path: closing drops the returned pane, and move
/// and restore use it too. Replaces `close`.
pub(crate) fn take(tab: &mut Tab, id: IdOf<Pane>) -> Result<Pane>;

/// Exchanges two panes' places, slot and stack position. Clears zoom.
/// Body changes; signature doesn't.
pub(crate) fn swap(tab: &mut Tab, a: IdOf<Pane>, b: IdOf<Pane>) -> Result<()>;

// resize, edge, split_at: unchanged. They never matched on the leaf, and
// `Layout::pane` still finds a behind pane, so resizing it moves its
// slot's edge.

/// The pane a closed pane's selection lands on. When the closed pane's slot
/// has others, that slot's next front; otherwise as today.
pub(crate) fn successor(layout: &Layout, closed: IdOf<Pane>) -> Option<IdOf<Pane>>;

/// Points `tab.zoomed` at the front of the slot it was in, if any.
fn refocus_zoom(tab: &mut Tab, was: IdOf<Pane>);
```

The zoom rules from the architecture's decision 3 and FR-014 to FR-016, by function:

| Function | Zoom |
| --- | --- |
| `split`, `swap`, `resize` | cleared, as today |
| `stack`, `cycle`, `bring_to_front` | kept; follows the slot's front |
| `take` | kept; follows the front; cleared only if the zoomed slot is removed |

#### `crates/ship-server/src/state/geometry.rs` (slice 1; title row in slice 2)

```rust
pub(crate) fn tab(tab: &Tab, size: Size) -> TabGeometry {
    fn walk(layout: &Layout, frame: Rect, bordered: bool, panes: &mut IndexMap<IdOf<Pane>, PaneGeometry>);
    //   Layout::Slot(slot) -> panes.insert(slot.front().id, PaneGeometry { frame, content: content(frame, bordered, slot) })
    // The zoomed branch looks up tab.slot(zoomed) and uses content(frame, false, slot).
}

/// Bordered: inside the border, where the titles go. Borderless: the whole
/// frame, less one top row for titles when the slot holds more than one pane.
fn content(frame: Rect, bordered: bool, slot: &Slot) -> Rect;
```

`CreatePane`'s start size uses the same rule. For `Place::Stack` there's no split to fit, so the new pane starts at the anchor's `content` size, one row shorter when the anchor's slot is borderless and holds one pane.

#### `crates/ship-server/src/state.rs` (slice 1; selection in slice 2; move in slice 4)

```rust
/// Rotate a pane's slot. Answers with the tab. (slice 2)
pub struct Cycle {
    pub id: IdOf<Pane>,
    pub cycle: CyclePane,
}

/// Move a pane onto or beside another, in one commit. Answers with the
/// destination tab. (slice 4)
pub struct PaneMove {
    pub id: IdOf<Pane>,
    pub to: MovePane,
}

impl Message<Cycle> for ServerState { type Reply = Result<Tab>; }
impl Message<PaneMove> for ServerState { type Reply = Result<Tab>; }
```

The existing `Move` message moves tabs and keeps its name, so the pane one is `PaneMove`. Every `layout::close(tab, id)?` call (remove, exit, restore) becomes `layout::take(tab, id)?;`. What changes in the existing code:

```diff
 // CreateTab
-Some(Layout::Pane(self.start_pane(...)?))
+Some(Layout::Slot(Slot::new(self.start_pane(...)?)))

 // CreatePane: commit
-layout::split(tab, anchor, create.direction, pane.clone())?;
+match create.place {
+    Place::Split(direction) => layout::split(tab, anchor, direction, pane.clone())?,
+    Place::Stack => layout::stack(tab, anchor, pane.clone())?,
+}

 // restore_tab: take keeps zoom correct on its own
-let zoomed = tab.zoomed;
 ...
-tab.zoomed = zoomed.filter(|id| tab.pane(*id).is_some());

 // repair: one rule before the zoom rule (FR-009)
+if selection is Pane(p) and p isn't its slot's front -> Pane(front)

 // Select handler (FR-008, FR-011)
-if tab.zoomed.is_some_and(|zoomed| zoomed != pane) { unzoom }
+layout::bring_to_front(tab, pane)?;
+if tab.zoomed.is_some_and(|zoomed| zoomed != pane) { unzoom }   // after bringing forward, zoomed == pane exactly when pane is in the zoomed slot

 // Set<Tab, SetTab>: zooming a pane behind the front brings it forward first (slice 2)
+if let Some(Some(pane)) = set.zoomed { layout::bring_to_front(tab, pane)? }
```

`PaneMove`, in one commit (slice 4):

```text
check id != at, else InvalidStructure "cannot move pane X onto or beside itself"
pane = layout::take(source tab, id)        // source slot may collapse; zoom per take
Place::Stack            -> layout::stack(dest tab, Some(at), pane)
Place::Split(direction) -> layout::split(dest tab, Some(at), direction, pane)
answer: the destination tab
```

That's the same `match` on `Place` as `CreatePane`'s commit.

`retain_runtimes` keeps the program because the pane's ID is in the tree after the commit. Selections on the moved pane stay on it, since it's a front in both forms.

#### `crates/ship-server/src/routes.rs` and `lib.rs` (slices 2 and 4)

```rust
/// POST /api/v0/panes/{id}/cycle, body CyclePane, 200 with the tab. (slice 2)
pub(crate) async fn cycle_pane(State(app): App, Path(id): Path<IdOf<Pane>>, Body(cycle): Body<CyclePane>) -> Result<Response>;

/// POST /api/v0/panes/{id}/move, body MovePane, 200 with the destination
/// tab; 404 for either pane, 422 for moving onto or beside itself. (slice 4)
pub(crate) async fn move_pane(State(app): App, Path(id): Path<IdOf<Pane>>, Body(body): Body<MovePane>) -> Result<Response>;
```

Both are added to `api_routes!` in `lib.rs`.

#### `crates/ship-client/src/api.rs` and `execute.rs` (slices 2, 4 and 5)

```rust
impl Client {
    /// Rotate the pane's slot; returns its tab.
    pub async fn cycle(&self, pane: IdOf<Pane>, body: &CyclePane) -> Result<Tab>;   // pane_edit(pane, "cycle", body)
    /// Move the pane; returns the destination tab.
    pub async fn move_pane(&self, pane: IdOf<Pane>, body: &MovePane) -> Result<Tab>; // pane_edit(pane, "move", body)
}
```

```diff
 PaneCommand::Create(pane::Create { tab, direction, ... })
-    direction: direction.unwrap_or_default(),
+    place: place.map(Place::from).unwrap_or_default(),
+PaneCommand::Cycle(pane::Cycle { pane, direction })  -> Outcome::Tab(self.cycle(id, &CyclePane { direction }))
+PaneCommand::Move(pane::Move { pane, onto, beside, direction }) -> Outcome::Tab(self.move_pane(..))
+PaneCommand::EditScrollback(pane::EditScrollback { pane, from }) -> { self.edit_scrollback(pane, from).await?; Outcome::Closed }
```

#### `crates/ship-client/src/automation.rs` (slice 5)

```rust
impl Client {
    /// Opens `target`'s scrollback, or the scrollback of the pane directly
    /// behind `from` in its slot, in the user's editor. Discards edits.
    pub(crate) async fn edit_scrollback(&self, target: Option<IdOf<Pane>>, from: Option<IdOf<Pane>>) -> Result<()>;
}

/// `<cache>/ship/scrollback`, from etcetera's cache dir, created if missing.
fn scrollback_dir() -> Result<PathBuf>;

/// Removes files in `dir` no running edit-scrollback holds. Each run holds
/// an exclusive `File::try_lock` on its own file until it exits, and a
/// crashed run's lock is released by the OS.
fn sweep(dir: &Path);

/// `VISUAL`, else `EDITOR`, else `vi`.
fn editor() -> String;

/// Runs `sh -c '<editor> "$1"' sh <file>` and waits.
#[cfg(unix)]
async fn launch(editor: &str, file: &Path) -> Result<ExitStatus>;

/// Fails with "edit-scrollback needs sh".
#[cfg(not(unix))]
async fn launch(editor: &str, file: &Path) -> Result<ExitStatus>;
```

The flow, in order:

```text
target  = --pane, else tab.slot(from).behind(from), else error "no pane behind pane X"
text    = self.read(target, ReadPane { lines: Some(u32::MAX), unwrap: true, ansi: false })
sweep(dir)
file    = tempfile::Builder::new().prefix("scrollback-").suffix(".txt").tempfile_in(dir)   // 0600 on Unix
file.try_lock(); write text
status  = launch(editor(), file.path())
drop file (deletes it); non-zero exit when the editor couldn't start (FR-025)
```

`u32::MAX` is safe: the vendored session clamps the count to the history it has (`vendor/ratatui-ghostty/src/session.rs:872`, `read.lines.unwrap_or(rows).min(total)`). That closes the architecture's "all lines" risk without a new read form.

#### `crates/ship-client/src/ui/panes.rs` (slice 3)

```rust
/// Each pane's title on `row`, front first, separated by " · ". Titles
/// that don't fit are left off. Used by drawing and by clicks, so both
/// agree on the cells.
pub(super) fn titles(slot: &Slot, row: Rect) -> Vec<(IdOf<Pane>, Rect)>;
```

`draw` changes in two places:

```diff
 // Labels after every border, since a border merges over text.
 for (id, pane) in &geometry.panes {
-    let label = tab.pane(*id).map(pane_label).unwrap_or_default();
-    ... render " {label} " on the top row
+    let slot = tab.slot(*id);
+    ... render titles(slot, top row): the front accent when selected,
+    ... bold when not; panes behind dim
 }
+// A borderless slot with more than one pane: titles on its frame's top
+// row, which geometry left out of the content.
```

The architecture's decision 10 says the client "remembers each label's cells for clicks". Here it recomputes them with the same `titles` function instead of storing them, so there's no state to keep in step with the drawing. Flagged for review.

#### `crates/ship-client/src/ui/mouse.rs` (slice 3)

```diff
 if kind == MouseEventKind::Down(MouseButton::Left) {
+    // A behind pane's title wins over a border grab on the same cell.
+    if let Some(pane) = title_at(scene, at) { return Some(Clicked::Select(NodeId::Pane(pane))); }
     let edges = edges_at(geometry, relative);
```

```rust
/// The pane behind a front whose title covers `at`.
fn title_at(scene: &Scene, at: Position) -> Option<IdOf<Pane>>;
```

The front pane's title keeps today's behavior: a press there selects it or grabs the border.

#### `crates/ship-client/src/ui/mod.rs` and `ui/memory.rs` (slice 3)

```diff
 // ui/mod.rs, Step::Pane (client.pane.next / prev, alt-tab)
-let panes: Vec<_> = tab.panes().map(|pane| pane.id).collect();
+let panes: Vec<_> = tab.layout.iter().flat_map(Layout::slots).map(|slot| slot.front().id).collect();

 // ui/memory.rs, Memory::landing
 tab.zoomed
-    .or_else(|| self.last.get(&tab.id).copied())
+    .or_else(|| self.last.get(&tab.id).and_then(|pane| tab.slot(*pane)).map(|slot| slot.front().id))
     .or_else(|| tab.panes().next().map(|pane| pane.id))
```

Neither is in the product paper. Both stop a client-local walk from reaching a pane behind a front, where selecting it would rotate the stack (FR-008). Without the `landing` change, switching back to a tab would undo another client's cycle. Both are marked under Open choices.

#### `crates/ship-core/src/defaults.toml` (slices 2 and 5)

```diff
 [client.menus]
 pane = [
-  { label = "split right", action = { server.pane.create = { direction = "right", select = true } } },
-  { label = "split down", action = { server.pane.create = { direction = "down", select = true } } },
+  { label = "split right", action = { server.pane.create = { place = "right", select = true } } },
+  { label = "split down", action = { server.pane.create = { place = "down", select = true } } },
+  { label = "stack", action = { server.pane.create = { place = "stack", select = true } } },
+  { label = "edit scrollback", action = { server.pane.create = { place = "stack", select = true, exit_behavior = "keep_failed", command = ["sh", "-c", "exec \"$SHIP_BIN\" pane edit-scrollback"] } } },
   { label = "zoom", ... },

-"alt--" = { action = { server.pane.create = { direction = "down", select = true } } }
-"alt-|" = { action = { server.pane.create = { direction = "right", select = true } } }
+"alt--" = { action = { server.pane.create = { place = "down", select = true } } }
+"alt-|" = { action = { server.pane.create = { place = "right", select = true } } }
+"alt-s" = { action = { server.pane.create = { place = "stack", select = true } } }
+"alt-]" = { action = { server.pane.cycle = { direction = "next" } } }
+"alt-[" = { action = { server.pane.cycle = { direction = "prev" } } }
+"alt-e" = { action = { server.pane.create = { place = "stack", select = true, exit_behavior = "keep_failed", command = ["sh", "-c", "exec \"$SHIP_BIN\" pane edit-scrollback"] } } }
```

The "stack" menu item lands in slice 2, "edit scrollback" and `alt-e` in slice 5.

#### Docs (in the slice that ships each behavior)

- `docs/site/panes.md`: stacking, cycling and the border titles (slice 3), moving (slice 4), edit-scrollback (slice 5). The `--direction right` example becomes `--place right`.
- `docs/site/keys.md`: `alt-s`, `alt-]`, `alt-[` (slice 2), `alt-e` (slice 5).
- `GLOSSARY.md`: slot and stack (slice 2).
- `docs/agents/code.md`: `direction` becomes `place` in the create-body rule (slice 2).

### Moved

None.

### Replaced

- `Layout::Pane(Pane)` is replaced by `Layout::Slot(Slot)` (slice 1).
- `CreatePane.direction` is replaced by `CreatePane.place` (slice 2). `direction` stays readable only through the command layer's alias, not on the wire.
- `pane::Create.direction` is replaced by `pane::Create.place`, with `direction` as an alias (slice 2).

### Untouched

- `ship-server/src/saver.rs`. `Layout`'s serde reads both leaf forms.
- `ship-core/src/tree.rs`. Lookups go through `Tab::pane`, which still finds every pane.
- `ship-core/src/geometry.rs` (`TabGeometry`, `largest`, `neighbor`). Directional focus and swap only see fronts because geometry only has fronts.
- `ship-client/src/ui/agents.rs` and the sidebar. The agents list still shows panes behind a front, and selecting one brings it forward through FR-008.
- `docs/research/edit-scrollback.md`. Its rewrite around stacks is separate from this build.

### Blocking unknowns

1. **Does `alt-[` reach Ship?** The client doesn't turn on the kitty keyboard protocol, so a terminal sends alt-`[` as `ESC [`, the start of an escape sequence. crossterm may wait for the rest or drop it. Settle it with a probe before slice 2. If it fails, FR-030 reopens: another key, or turning the protocol on.
2. **How does `Slot::take` get the only pane out by value?** The spike checked removal only while others remain, and `pop_front_if_many().or_get_only()` lends the last pane rather than returning it. Find the mitsein call that turns a one-item `VecDeque1` into its item before slice 1. If none exists, `Slot::take` converts to a `VecDeque` first.

### Open choices

None of these block slice 1. Each has a recommendation in the paper; review decides.

1. **`alt-tab` walks fronts only** (`ui/mod.rs`, slice 3). Alternative: walk every pane, rotating stacks as it goes.
2. **Tab landing goes to the remembered pane's slot front** (`ui/memory.rs`, slice 3). Alternative: bring the remembered pane forward, undoing other clients' cycles.
3. **`ship tab set --zoomed X` on a pane behind the front brings it forward, then zooms** (`state.rs`, slice 2). Alternative: reject it with an error.
4. **A behind pane's title wins over a border grab on the same cell** (`ui/mouse.rs`, slice 3). The front's title keeps today's behavior.
5. **Slot edits are `Slot` methods in `ship-core`** rather than free functions in `state/layout.rs` (Rationale).
6. **Title cells are recomputed, not remembered** (`ui/panes.rs`, slice 3).

## Build order

Each slice builds and passes `cargo fmt --check`, `cargo clippy --all-targets` and `cargo deny check`, run on macOS. Linux is unverified unless run there. The workflow checks use a fresh server on a scratch socket.

1. **Slot leaf, no behavior change.**
   Files: `Cargo.toml`, `crates/ship-core/Cargo.toml`, `ship-core/src/layout.rs`, `model.rs`, `protocol.rs` (version only), `ship-server/src/state/layout.rs`, `state/geometry.rs`, `state.rs` (`CreateTab`, `restore_tab`), and whatever client code matched `Layout::Pane`.
   Delivers: every leaf is a slot of one; `close` is replaced by `take`, which still clears zoom as today.
   Done when: `ship tab get | jq .layout` shows `"kind": "slot"` leaves with a `panes` list; a `tabs.json` saved by the current release (copied before building) loads with every pane present (SC-004, FR-027); splitting, closing, swapping, resizing and zooming behave as before.

2. **Stack, cycle, selection and zoom on the server and CLI.**
   Files: `protocol.rs` (`Place`, `CyclePane`), `command.rs` (`Placement`, `place`, `Cycle`), `state/layout.rs` (`stack`, `cycle`, `bring_to_front`, `take`'s zoom rules, `successor`), `state/geometry.rs` (title row), `state.rs` (create, `Cycle`, `repair`, select, `SetTab`), `routes.rs`, `lib.rs`, `ship-client/src/api.rs`, `execute.rs`, `defaults.toml` (`place` rename, `alt-s`, `alt-]`, `alt-[`, "stack" menu item), `docs/site/keys.md`, `GLOSSARY.md`, `docs/agents/code.md`.
   Delivers: P1 except title clicks, P3, and P5's stacked restore.
   Done when: from the CLI, `ship pane create --place stack` twice then `ship pane cycle --direction next` and `--direction prev` give the orders in P1, read back with `ship tab get`; `--direction right` still works; closing the front shows the next pane and closing the last removes the slot (SC-002); each P3 step leaves `zoomed` set and pointing at the front (SC-003); a selection on a pane pushed behind ends on the new front; `ship view select` of a behind pane brings it forward; a restart restores a stacked slot in order; in the client, `alt-s`, `alt-]` and `alt-[` work and only the front is drawn.

3. **Titles and clicks in the client.**
   Files: `ship-client/src/ui/panes.rs`, `ui/mouse.rs`, `ui/mod.rs`, `ui/memory.rs`, `docs/site/panes.md`.
   Delivers: FR-010, FR-028, FR-029, and Open choices 1, 2, 4.
   Done when: a stacked slot's border reads `X · A · B` with the front marked; a single borderless stacked slot shows the title row and loses it when unstacked; clicking `B`'s title selects `B` and the order becomes `[B, X, A]`; `alt-tab` skips panes behind; cycling feels as quick as moving between split panes (SC-005, judged in use).

4. **Move.**
   Files: `protocol.rs` (`MovePane`), `command.rs` (`Move`), `state/layout.rs`, `state.rs` (`PaneMove`), `routes.rs`, `lib.rs`, `api.rs`, `execute.rs`, `docs/site/panes.md`.
   Delivers: P4, FR-018 to FR-021.
   Done when: `ship pane move --onto` and `--beside --direction right` behave as in P4 within a tab and across tabs; a program running in the moved pane (say `top`) keeps running with the same pane ID; an emptied slot is gone; moving onto or beside itself fails and `ship tab get` is unchanged.

5. **Edit-scrollback.**
   Files: `crates/ship-client/Cargo.toml`, `command.rs` (`EditScrollback`), `execute.rs`, `automation.rs`, `defaults.toml` (`alt-e`, "edit scrollback" menu item), `docs/site/panes.md`, `docs/site/keys.md`.
   Delivers: P2, FR-022 to FR-025, SC-001.
   Done when: `alt-e` on a pane that printed `seq 1 5000` opens the editor with all 5000 lines; quitting returns to the pane with the layout unchanged; `EDITOR="code --wait"` style values work; the temp file is mode 0600 while open and gone after; a file left by a killed run is removed by the next run while a concurrent run's file is not; running it in a pane alone in its slot fails with the "no pane behind" message; with `EDITOR=/nonexistent` the pane stays open showing the error.

## Deviation log

Empty.
