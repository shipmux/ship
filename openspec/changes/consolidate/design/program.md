# Consolidate the server and client: program

Status: Locked on 2026-10-31. Cyan first approved the ten-slice draft in chat when asking to start slice 1 ("Lock it, then build"), then, in a second session, settled O-1, asked for the four slices and the `docs/agents/code.md` fixes, and approved this revision ("also lock them before committing"). Cyan chose to skip the product and architecture papers for this change, because it keeps product behavior the same. This paper is the only design paper. O-2 stays open and keeps its default until Cyan decides. O-3 was settled at task 1.6: convert the vendor.

## Rationale

This change makes Ship's implementation smaller and easier to follow without changing what users or scripts see. It comes from a read of all five crates (13,732 lines of implementation Rust by `cloc`, code lines only). Five decisions shape it:

- **KD-1: Reads come from the published replica.** GET routes, the ordering middleware and the input route read the latest replica instead of asking the state actor. The actor handles writes only. `ServerState` owns the `ReplicaSink` and publishes each replica synchronously inside `commit`, so the replica is current before a commit replies. That makes the revision `AtomicU64` a second copy of a value the watch already holds, so it goes. Pane events and clipboard text stay on the `RelayBus`; Cyan chose to keep the bus.
- **KD-2: Panes report their state, not a sequence of changes.** A pane task sends one `PaneEvent` carrying its current title, directory and program records, plus its exit status once it has exited. The state actor commits only when something differs. Today a shell prompt can send title, program and directory as three events, three commits and three full replicas.
- **KD-3: New panes are sized by the geometry that publishes them.** `CreatePane` places a placeholder pane in a copy of the tab and reads its content size from `geometry::tab`, as `resize` already tries edits on a copy. The separate split-halves and title-row arithmetic goes.
- **KD-4: The client loop is a dispatcher.** Keys and the mouse produce lane items directly, navigation reads one per-turn `View`, and the attach stream's lifecycle (attach, reconnect, backoff, area PUTs) lives in one `Connection`. `ui/mod.rs` keeps the loop and the turn.
- **KD-5: Shared shapes are stated once.** Lookups the code repeats get names (`tree::pane_tab`, `tree::parent`), conversions written twice move into `ship-core` (screen styles and colors, `Size` from `Rect`, `Axis` and `Direction` helpers), and suppressed lints are fixed rather than suppressed where a clean fix exists. New restriction lints teach future agents the conventions in `docs/agents/code.md` through the compiler.

Behavior stays the same, with three visible differences, all on the wire rather than on screen:

- Every-revision feeds (`ship pane wait`, `send --wait`) see fewer, merged revisions after KD-2. Each revision's contents are what the separate ones would have converged to.
- A commit can no longer fail with `Unavailable` after its edit, since publishing the replica can't fail (KD-1).
- Reads stop waiting behind queued writes in the actor's mailbox (KD-1). A read issued after a write's response still sees that write.

New shapes follow `docs/agents/code.md`: a flag, default or special case gets a named type or variant (`SavedTabs`, `Reattach`, `Fold`, `Hop::Collapse` and `Hop::Expand`), new tuples become named structs (`Checked`, `Prepared`, `Startup`'s fields), and an existing shape is reused where it fits (`Starter` in `ServeOptions`, `PaneStatus` in `PaneEvent`, `CycleDirection` in `Step`). Two cleanups were dropped for the same reason: deriving `Ord` on `ui::agents::Display` would make its ranking implicit in variant order, so the explicit `rank()` stays, and dropping `Identified` would leave `IdOf<T>` as an alias that only restates `Id<T>`.

Kept on purpose, by Cyan's choice: `#[serde(rename_all = "snake_case")]` on structs, explicit `err!(…, @external: error)` at call sites, wildcard imports, the `RelayBus`, and the two documented `too_many_arguments` expectations on the sidebar draw functions unless open choice O-2 says otherwise.

This is a document-only plan. None of the code below exists in the working tree, and none of it has been compiled. The container this was written in can't build Ship (no Zig for `libghostty-vt-sys`).

## Skeleton map

Paths are relative to `crates/` unless they start with the repository root.

### Added

#### `ship-client/src/ui/nav.rs` (slice 3)

```rust
//! Where navigation keys and tab-tree actions go, read from one view of
//! the replica per turn. No requests; callers turn results into lane items.

use ship_core::{
    command::Command,
    id::IdOf,
    layout::CycleDirection,
    model::{NodeId, Pane, Switch, Tab},
    protocol::MoveTab,
    tree::Tabs,
};

use super::{lane::TabMove, memory::Memory, sidebar::Sidebar};
use crate::keymap::TabDirection;

/// Moved from `ui/mod.rs`. The bare `forward: bool` becomes the layout's
/// own `CycleDirection`.
#[derive(Clone, Copy)]
pub(super) enum Step {
    Tab(CycleDirection),
    Pane(CycleDirection),
    Focus(ship_core::command::Direction),
}

/// What `client.tab.collapse` or `client.tab.expand` does on the viewed
/// tab's row. Was `Fold { collapsed: bool }`; each fold now has its own
/// variant. `Select` carries the landing node, not the tab.
pub(super) enum Hop {
    Collapse(IdOf<Tab>),
    Expand(IdOf<Tab>),
    Select(NodeId),
}

/// The replica's tabs, this client's selection and the client-local state
/// navigation reads. Built once per turn by `Local::view`.
pub(super) struct View<'a> {
    pub tabs: &'a Tabs,
    pub selection: Option<NodeId>,
    memory: &'a Memory,
    sidebar: &'a Sidebar,
}

impl<'a> View<'a> {
    pub fn new(tabs: &'a Tabs, selection: Option<NodeId>, memory: &'a Memory, sidebar: &'a Sidebar) -> Self;

    /// The selected tab, or the selected pane's tab.
    pub fn viewed_tab(&self) -> Option<IdOf<Tab>>;

    /// Was `navigate`.
    pub fn navigate(&self, step: Step) -> Option<NodeId>;

    /// Was `collapse`, on the viewed tab.
    pub fn collapse(&self) -> Option<Hop>;

    /// Was `expand`, on the viewed tab.
    pub fn expand(&self) -> Option<Hop>;

    /// Was `shift`, on the viewed tab.
    pub fn shift(&self, direction: TabDirection) -> Option<TabMove>;

    /// Was `Local::land`'s lookup: a clicked node, or the tab's landing
    /// pane. `None` when the node is gone.
    pub fn land(&self, node: NodeId) -> Option<NodeId>;

    /// Was `typing_pane`.
    pub fn typing_pane(&self) -> Option<IdOf<Pane>>;

    /// Was `zoomed_pane`.
    pub fn zoomed_pane(&self) -> Option<IdOf<Pane>>;

    /// Was `zoom`.
    pub fn zoom(&self, zoomed: Switch) -> Option<Command>;
}

/// Moved from `ui/mod.rs`, unchanged.
pub(super) fn set_zoom(pane: IdOf<Pane>, zoomed: Option<IdOf<Pane>>) -> Command;
pub(super) fn leaves_zoom(action: &crate::keymap::Action) -> bool;
fn step(index: usize, len: usize, direction: CycleDirection) -> usize;
```

#### `ship-client/src/ui/connection.rs` (slice 3)

```rust
//! The attach stream and everything that keeps it open: the first attach,
//! reconnecting with backoff, cutting a broken connection and the
//! throttled area PUT.

use std::time::Duration;

use ship_core::{
    id::{Attachment, IdOf},
    prelude::*,
    protocol::{AttachRequest, SseEvent, ViewInput},
    screen::Size,
};

use super::{Events, Request};
use crate::Client;

pub(super) struct Connection<'a> {
    client: &'a Client,
    /// What the next attach sends; its area is refreshed before each try.
    request: AttachRequest,
    events: Option<Events>,
    reconnect: Option<Request<'a, Events>>,
    backoff: Duration,
    putting: Option<Request<'a>>,
}

/// What the connection has to say to the loop.
pub(super) enum Link {
    /// Any event but `ended`. On `attached`, numbering has started and the
    /// backoff has reset.
    Event(SseEvent),
    /// `ended` with `server_shutdown`.
    Shutdown,
    /// The stream broke, fell behind, or ended without notice. Logged here;
    /// the loop cuts.
    Lost,
    /// The area PUT failed.
    PutFailed(AppError),
}

/// What a cut connection reattaches with.
pub(super) enum Reattach {
    /// The last record's selection at the current area, from
    /// `Observer::remembered`.
    Remembered(AttachRequest),
    /// The request the connection last sent, as before the first
    /// `attached` event.
    Previous,
}

impl<'a> Connection<'a> {
    pub fn new(client: &'a Client, request: AttachRequest, events: Events) -> Self;

    /// Waits on the stream, a reconnect attempt and the area PUT together.
    /// A failed reconnect retries after the doubled backoff with `area`.
    pub async fn next(&mut self, area: Size) -> Link;

    /// Drop the stream, the PUT and numbering, and start reattaching.
    pub fn cut(&mut self, reattach: Reattach);

    /// No PUT in flight.
    pub fn idle(&self) -> bool;

    pub fn put(&mut self, attachment: IdOf<Attachment>, view: ViewInput);
}
```

Moves `attach_after`, `refused`, `FIRST_BACKOFF` and `MAX_BACKOFF` here from `ui/mod.rs`. `next`'s three waits are one future that borrows `self`, so `drive`'s `select!` has one connection arm instead of three.

#### `openspec/changes/consolidate/` (this change)

`proposal.md`, `design.md` and `tasks.md`, assembled from this paper. No spec deltas: no requirement changes.

### Moved

- `ui/mod.rs` → `ui/nav.rs`: `Step`, `Hop`, `navigate`, `collapse` and `expand` (as `View` methods), `shift`, `typing_pane`, `zoomed_pane`, `zoom`, `set_zoom`, `leaves_zoom`, `step` (slice 3).
- `ui/mod.rs` → `ui/connection.rs`: `attach_after`, `refused`, the backoff constants, and the reconnect, cut and PUT handling now inline in `drive` (slice 3).

### Replaced

#### `ship-core/src/tree.rs` (slice 1)

```rust
/// The tab holding pane `id`. Replaces `tab(tabs, pane_owner(tabs, id)?)`,
/// written out about 15 times.
pub fn pane_tab(tabs: &Tabs, id: IdOf<Pane>) -> Result<&Tab>;

/// The tab's parent, `None` at the top level or when absent. Replaces
/// `path(..)?.iter().rev().nth(1)`.
pub fn parent(tabs: &Tabs, id: IdOf<Tab>) -> Option<IdOf<Tab>>;

/// Every pane in the tree, in no particular order. `pane_ids` becomes
/// `panes(tabs).map(|pane| pane.id)` at its one server call site;
/// `attention::roots` stops looking each ID back up.
pub fn panes(tabs: &Tabs) -> impl Iterator<Item = &Pane> + '_;
```

#### `ship-server/src/state/tree.rs` (slice 1)

```rust
/// `tab_mut` on the tab holding pane `id`.
pub(crate) fn pane_tab_mut(tabs: &mut Tabs, id: IdOf<Pane>) -> Result<&mut Tab>;

/// The pane itself, through `pane_tab_mut`; `not_found` if absent.
pub(crate) fn pane_mut(tabs: &mut Tabs, id: IdOf<Pane>) -> Result<&mut Pane>;
```

#### `ship-core/src/screen.rs` (slice 1)

```rust
use ratatui_core::{layout::Rect, style::{Color as TuiColor, Modifier}};

impl From<Rect> for Size { /* cols = width, rows = height */ }

/// Named colors are palette entries 0 to 15. Was `pane::color` (server).
impl From<TuiColor> for Color;
/// Was `panes::color` (client).
impl From<Color> for TuiColor;

impl Attr {
    /// One table for both directions. Was `ATTRS` (server `pane.rs`) and
    /// `MODIFIERS` (client `panes.rs`).
    const MODIFIERS: [(Attr, Modifier); 9];
    pub fn from_modifier(modifier: Modifier) -> Vec<Attr>;
    pub fn to_modifier(attrs: &[Attr]) -> Modifier;
}

/// `[offset/history]`, the scroll badge. Was formatted twice in `panes.rs`.
impl fmt::Display for Scrolled;
```

About seven hand-written `Size { cols: rect.width, rows: rect.height }` conversions use `Size::from`.

#### `ship-core/src/layout.rs` and `ship-core/src/command.rs` (slice 1)

```rust
// layout.rs
impl From<SplitDirection> for Axis;                    // was state::layout::axis
impl From<Axis> for ratatui_core::layout::Direction;   // was inline in geometry::halves
impl Axis {
    /// Width or height.
    pub fn length(self, rect: Rect) -> u16;
    /// x or y of the rect's start.
    pub fn start(self, rect: Rect) -> u16;
    /// x or y of a position.
    pub fn along(self, position: Position) -> u16;
}

// command.rs
impl Direction {
    /// Left and right are horizontal; up and down vertical.
    pub fn axis(self) -> Axis;
    /// Right and down.
    pub fn forward(self) -> bool;
}
```

Callers: `state/layout.rs` (`resize`), `state/geometry.rs` (`halves`, `ratio_for`), `ui/mouse.rs` (grabs), `ui/mod.rs` (`Item::Resize`), `sidebar/mod.rs` (`nudge`).

#### `ship-core/src/id.rs` and `model.rs` (slice 1)

```rust
pub trait Prefixed {
    const PREFIX: &'static str;
}

impl<T: Prefixed> Id<T> {
    /// Was `new`, which needed `#[allow(clippy::new_without_default)]`; a
    /// default ID would hide that it's random. Four call sites.
    pub fn random() -> Self;
}

impl Prefixed for Attachment { const PREFIX: &'static str = "attachment"; }
// model.rs: the same one-liners for Tab ("tab") and Pane ("pane").

/// A JSON object keyed by an ID, with `$ref` values. Replaces
/// `tabs_schema`, `viewers_schema` and `panes_schema`.
pub(crate) fn id_map_schema<K: Prefixed>(value: &str) -> utoipa::openapi::Object;
```

`Identified` and `IdOf<T>` stay. Dropping the trait would leave `IdOf<T>` as an alias that only restates `Id<T>`, and replacing it everywhere is churn for no new meaning.

#### `ship-core/src/geometry.rs` (slice 1)

```rust
/// Mirror of ratatui's `Rect` for the schema only; replaces the hand-built
/// `rect_schema`. Utoipa states `u16` as an integer of minimum 0 without
/// the old maximum; AGENTS.md accepts that imprecision.
#[derive(utoipa::ToSchema)]
#[schema(as = Rect)]
struct RectSchema { x: u16, y: u16, width: u16, height: u16 }
```

#### `ship-core/src/error.rs` (slice 1)

```rust
impl AppError {
    /// By value: `ErrorCode` is `Copy`. Drops `*error.code()` at call sites.
    pub fn code(&self) -> ErrorCode;
    fn data_mut(&mut self) -> &mut Option<Value>;   // shared by `with_data` and `with_serialized_data`
}

// `err!` turns a bare variant into a path once, at the entry,
// instead of six `#[allow(unused_imports)] use ErrorCode::*`.
macro_rules! err {
    ($code:ident, $($rest:tt)+) => { $crate::err!(@code $crate::error::ErrorCode::$code, $($rest)+) };
    // `err!(ErrorCode::UpstreamHttpStatus(..), ..)` and other expressions:
    (@code $code:expr, ...) => { /* the existing arms, without the import */ };
    ($code:expr, $($rest:tt)+) => { $crate::err!(@code $code, $($rest)+) };
}
```

#### Workspace lints, `Cargo.toml` (slice 1)

Proposed settings:

```toml
[workspace.lints.clippy]
# existing entries unchanged, plus:
allow_attributes = "warn"                 # `#[expect]`, so stale suppressions surface
allow_attributes_without_reason = "warn"
let_underscore_untyped = "warn"           # code.md: discard with `.ok()`, not `let _ =`
unwrap_used = "warn"                      # `expect("why")`, as the rest of the code does
```

`vendor/ratatui-ghostty` inherits the workspace lints. It has 32 `let _ =` and one `.unwrap()`; see open choice O-3.

#### Lint fixes (slice 1)

```rust
// ship-server/src/lib.rs: replaces the `too_many_arguments` expectation.
pub struct ServeOptions {
    pub endpoint: Endpoint,
    /// What a new tab starts with when nothing was restored; `None` creates
    /// no tab. The protocol's own `Starter`, as `CreateTab` takes it.
    pub starter: Option<Starter>,
    pub config: PathBuf,
    /// Where saved tabs are read and written.
    pub save: PathBuf,
    pub saved: SavedTabs,
}

/// Whether startup restores the saved tabs. Was the positional `restore: bool`.
pub enum SavedTabs {
    Restore,
    /// `--no-restore`: start empty; the first save replaces the file.
    Ignore,
}
pub async fn serve(
    options: ServeOptions,
    shutdown: impl Future<Output = Result<()>> + Send + 'static,
    log: impl Fn(Option<LogChange>) -> Result<LogFilter> + Send + Sync + 'static,
    run_output: impl Fn() -> Result<File> + Send + Sync + 'static,
) -> Result<()>;

// ship-client/src/ui/mod.rs: replaces the other expectation.
struct Startup<'a> {
    client: &'a Client,
    config: &'a Path,
    /// The file's settings, or the defaults when it was unusable.
    settings: ClientSettings,
    /// Why the file was unusable, for the status line. Was the second half
    /// of the `loaded` tuple.
    message: Option<String>,
    connection: &'a RefCell<tracing::Span>,
}
async fn drive(
    startup: Startup<'_>,
    request: AttachRequest,
    events: Events,
    signaled: impl Future<Output = ()>,
    run_output: impl Fn() -> Result<File> + Send + Sync + 'static,
) -> Result<Exit>; // slice 3 folds `request` and `events` into `Connection`

// ship-server/src/attach.rs: `let _ = &guard;` goes. The guard moves into
// `Progress`, so the unfold state owns it and drops it with the stream.
struct Progress {
    guard: AttachmentGuard,
    // existing fields
}
```

`ship/src/runlog.rs`: six `.lock().unwrap()` and one `file_name().unwrap()` become `expect` with a reason.

#### `ship-core/src/relay.rs` (slice 1)

Removed, unused: the `Sink` impls for `mpsc::Sender`, `Recipient` and closures, `SinkExt`, `FilterSink`, `FilterMapSink`, and `SendResult::Full`. Kept: `RelayBus`, `Subscribe`, `Publish`, `Sink`, and the impls for `mpsc::UnboundedSender` and `watch::Sender`. Slice 2 then removes the `watch::Sender` impl, whose only user was `ReplicaSink`.

#### Server local cleanups (slice 1)

```rust
// state.rs
impl ServerState {
    /// The tab's viewed size, else the 80x24 fallback. Two call sites.
    fn tab_size(&self, tab: IdOf<Tab>) -> Size;
}
/// Six copies of `err!(NotFound, "attachment {} not found", ..)`.
pub(crate) fn attachment_not_found(attachment: IdOf<Attachment>) -> AppError;
fn viewer_mut(viewers: &mut Viewers, attachment: IdOf<Attachment>) -> Result<&mut ViewingRecord>;

// state/layout.rs
/// Split or stack by `place`. Was the same match in `CreatePane` and `PaneMove`.
pub(crate) fn place(tab: &mut Tab, anchor: Option<IdOf<Pane>>, place: Place, pane: Pane) -> Result<()>;
/// Clear the tab's zoom unless it's on `pane`. Three call sites.
pub(crate) fn unzoom_unless(tab: &mut Tab, pane: IdOf<Pane>);
/// Find the pane's slot, edit it, then let zoom follow the slot's front.
/// Shared by `cycle` and `bring_to_front`; `stack` reuses the refocus.
fn edit_slot(tab: &mut Tab, pane: IdOf<Pane>, edit: impl FnOnce(&mut Slot)) -> Result<()>;

// routes.rs, views.rs, attach.rs: handlers return `Result<Json<T>>`, or
// `Result<(StatusCode, Json<T>)>` for 201, instead of `Json(v).into_response()`.
/// The live pane or 404, for `scroll_pane` and `read_pane`.
fn live_pane(app: &AppState, id: IdOf<Pane>) -> Result<PaneHandle>;

// saver.rs
impl Saver {
    /// Project the latest replica and save it if it differs from what was
    /// written. The three copies in `run` call this; `run`'s timer arm uses
    /// a `select!` precondition instead of the `pending()` wrapper.
    async fn flush(&mut self, replicas: &mut watch::Receiver<Arc<Replica>>) -> Result<()>;
}

// pane.rs
fn pty_size(size: Size) -> PtySize;   // two copies of the four-field literal
```

#### Client local cleanups (slice 1)

```rust
// ui/rename.rs: `Target` was the same shape as `NodeId`.
pub fn open(target: NodeId, stored: Option<&str>) -> Self;
pub fn target(&self) -> NodeId;

// ui/attention.rs
impl Watched { fn new(state: ReportedState) -> Self; }

// ui/sidebar/mod.rs: one entry point, dispatching on `direction.axis()`.
pub fn nudge(&mut self, direction: Direction, amount: u16, cols: u16); // `nudge_split` folds in

// ui/observer.rs
impl Observer {
    /// `record()?.selection`, written out six times.
    pub fn selection(&self) -> Option<NodeId>;
    pub fn tabs(&self) -> Option<&Tabs>;
}
```

`ui/mouse.rs`: the `Up` and `Drag` arms merge; only `Up` takes `held`.

#### `ship/src` (slice 1)

```rust
// main.rs: three `server` subcommand arms, `connect` and bare `ship`
// repeat target -> endpoint -> checked.
async fn checked_target(cli: &Cli) -> Result<Checked>;

/// A client whose health check passed, and that check's answer. Was the
/// `(Client, HealthResponse)` tuple `checked` returns.
struct Checked {
    client: Client,
    health: HealthResponse,
}

// cli.rs: the two environment-variable branches of `target`.
fn env_target<T: FromStr<Err = AppError>>(name: &str) -> Result<Option<T>>;

// diagnostics.rs: the Unix and Windows `shutdown` share their wait.
async fn first_signal(a: impl Future<Output = Option<()>>, b: impl Future<Output = Option<()>>) -> Result<()>;
```

`diagnostics::init` builds its two fmt layers from one closure, and `startup_filter` states the `SHIP_LOG` error once.

#### `ship-server/src/replicas.rs`, `app.rs`, `lib.rs` (slice 2)

```rust
// replicas.rs
pub(crate) struct ReplicaSink {
    latest: watch::Sender<Arc<Replica>>,
    ring: broadcast::Sender<Arc<Replica>>,
}
impl ReplicaSink {
    pub(crate) fn new(first: Arc<Replica>) -> Self;
    /// Never fails: `send_replace`, and no ring readers is normal.
    pub(crate) fn publish(&self, replica: Arc<Replica>);
    pub(crate) fn latest(&self) -> Arc<Replica>;
    /// A read handle for routes, streams and the saver.
    pub(crate) fn subscribe(&self) -> Replicas;
}
impl Replicas {
    pub fn latest(&self) -> watch::Receiver<Arc<Replica>>;   // unchanged
    pub(crate) fn every(&self) -> Option<broadcast::Receiver<Arc<Replica>>>; // unchanged
    /// The replica published last.
    pub(crate) fn current(&self) -> Arc<Replica>;
}
// `impl Sink<Arc<Replica>> for ReplicaSink` goes.

// app.rs: `AppState::revision: Arc<AtomicU64>` goes; `stamp` reads
// `app.replicas.current().revision`.
```

`serve` stops subscribing the replica sink on the bus. `ServerState::new` creates the `ReplicaSink` with an empty first replica, restores, publishes the restored replica, and `serve` takes `state.replicas()` before spawning the actor. When the state actor stops, it drops the sink, which ends every attach stream with `server_shutdown`, as dropping the bus does today.

#### `ship-server/src/state.rs` (slice 2)

```rust
pub struct ServerState {
    // `written: Arc<AtomicU64>` goes; the sink holds the revision.
    replicas: ReplicaSink,
    // other fields unchanged
}

impl ServerState {
    pub(crate) fn replicas(&self) -> Replicas;
    /// Reads the replica `commit` last published instead of recomputing
    /// every viewed tab's geometry.
    fn published_tab(&self, id: IdOf<Tab>) -> Result<Tab>;
}
```

Removed messages and their handlers: `ListTabs`, `Get<Tab>`, `Get<Pane>`, `ListViews`, `GetView` and `CheckAttachment`. Their routes (`list_tabs`, `get_tab`, `get_pane`, `list_views`, `get_view`), `input`'s attachment check and `order::lane` read `app.replicas.current()`. `commit` publishes with `self.replicas.publish(..)` instead of `bus.tell(Publish(..)).await`, so its `Unavailable` path goes. `Attach` returns the replica it just published.

#### `ship-server/src/pane.rs` and `state.rs` (slice 2)

```rust
/// The pane's reported state as it stands, sent after a publish in which
/// any of it changed, and once more when the program exits. The fields are
/// `Pane`'s own names and types.
#[derive(Clone, PartialEq)]
pub(crate) struct PaneEvent {
    pub pane: IdOf<Pane>,
    pub title: Option<String>,
    /// From OSC 7, else the program's working directory; the last one known.
    pub dir: Option<String>,
    /// Empty once exited.
    pub program: IndexMap<String, StatusRecord>,
    /// `Pane::status`'s own type, not a second way to say "exited".
    pub status: PaneStatus,
}
```

`PaneChange`, `PaneTask::set_dir` and `PaneTask::event` go. `PaneTask` keeps the last event it sent and sends a new one when it differs. The `Message<PaneEvent>` handler writes the four fields, commits only if the pane differs, and on an exit that closes the pane commits the removal second, as now, so every-revision feeds still see the exit before the pane goes.

#### `ship-server/src/state.rs` and `state/geometry.rs` (slice 2)

```rust
// state/geometry.rs
/// The content size a new pane would get: place a placeholder in a copy of
/// `tab` at `anchor` by `place`, then read its rectangle from `tab()`. When
/// another slot's zoom hides it, read the unzoomed copy instead.
/// `InvalidStructure` when it or the split anchor's front would fall below
/// `MIN_CONTENT`. An empty tab gets `size`.
pub(crate) fn start_size(tab: &Tab, size: Size, anchor: Option<IdOf<Pane>>, place: Place) -> Result<Size>;

/// A pane with only an ID, for trial placements.
fn placeholder(id: IdOf<Pane>) -> Pane;
```

`CreatePane` calls `start_size` in place of the `halves`, `Block::bordered().inner`, `MIN_CONTENT` and title-row arithmetic.

#### `ship-client/src/ui/keys.rs`, `mouse.rs`, `mod.rs`, `panes.rs` (slice 3)

```rust
// keys.rs: `Handled` goes; its variants were `Item::Action`, `Item::Key`,
// `Item::Paste` and nothing.
impl Keys {
    pub fn handle(&mut self, event: Event) -> Option<Item>;
}

// mouse.rs: `Clicked` goes. Most variants were lane items under another name.
pub(super) enum Routed {
    Lane(Item),
    /// Border drags: one `Item::Resize` per edge, merged by the lane.
    Resize(Vec<EdgeAt>),
    Sidebar(SidebarEdit),
}
pub(super) enum SidebarEdit {
    Width(u16),
    Split(u16),
    Scroll { at: Position, scroll: Scroll },
}
impl Mouse {
    pub fn route(&mut self, event: MouseEvent, scene: &Scene, busy: bool) -> Option<Routed>;
}

// sidebar/mod.rs
impl Sidebar { pub fn edit(&mut self, edit: SidebarEdit, cols: u16); }

// mod.rs
impl Local<'_> {
    /// Was `read -> Handled`: the overlay's pick, else the mode's item.
    fn read(&mut self, event: Event) -> Option<Item>;
}
impl Scene<'_> {
    /// `rect`, relative to the pane area, on the terminal. Was closures in
    /// `panes::draw`, `mouse::route` and `panes::title_row`.
    fn on_screen(&self, rect: Rect) -> Rect;
    /// `at` relative to the pane area, saturating at its edges.
    fn relative(&self, at: Position) -> Position;
}
```

`Picked` stays: overlays need its open, close and run cases.

#### `ship-client/src/ui/mod.rs` (slice 2)

```rust
impl<'a> Local<'a> {
    /// `None` before the first replica.
    fn view(&self) -> Option<View<'_>>;
    /// Was the event arm's follow-ups: the observer, attention, memory and
    /// the sidebar's reveal. Returns whether anything visible changed.
    fn apply(&mut self, event: SseEvent) -> bool;
    /// Was the loop's tail: rename, sidebar and mouse reconciliation.
    fn reconcile(&mut self) -> bool;
    /// Fold the row or select the landing node, for `client.tab.collapse`
    /// and `client.tab.expand`.
    fn hop(&mut self, client: &'a Client, hop: Hop) -> Turn<'a>;
}

async fn drive(
    startup: Startup<'_>,
    connection: Connection<'_>,
    signaled: impl Future<Output = ()>,
    run_output: impl Fn() -> Result<File> + Send + Sync + 'static,
) -> Result<Exit>;
```

`act`'s Collapse and Expand arms each become one line handing their `Hop` to one shared `Local::hop`. The three `Turn::Wait` blocks that treat `NotFound` (and `InvalidStructure` for moves) as done share one helper:

```rust
/// Run `request` as a turn, counting the given codes as done: the target
/// vanished before the turn.
fn tolerant<'a, T>(request: impl Future<Output = Result<T>> + 'a, gone: &'static [ErrorCode]) -> Turn<'a>;
```

#### `ship-client/src/lib.rs` and `api.rs` (slice 4)

```rust
impl Client {
    /// JSON in, JSON out, numbered while attached. Most endpoints.
    async fn call<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<&(impl Serialize + ?Sized)>,
        expect: StatusCode,
    ) -> Result<T>;

    /// The request with `x-ship-order` and, when given, the attachment header.
    fn build(&self, method: Method, path: &str, attachment: Option<IdOf<Attachment>>, order: Option<Order>) -> Prepared;

    /// Send within `TIMEOUT`, check the status, record `x-ship-revision`,
    /// then `read` the response. The one copy of the timeout and context.
    async fn exchange<T>(
        &self,
        prepared: Prepared,
        expect: StatusCode,
        read: impl AsyncFnOnce(Response) -> Result<T>,
    ) -> Result<T>;
}

/// A request ready to send. Named rather than a `(RequestBuilder, Url)` tuple.
struct Prepared {
    request: RequestBuilder,
    /// The URL without credentials, for errors.
    safe: Url,
}
```

`request`, `numbered`, `stream` and `NO_BODY` go. `read` (text), `attach` (stream, head only), `input` and `set_view` (attachment header) use `build` and `exchange` directly. `pane_edit` stays as a thin `call`.

#### `ship-client/src/execute.rs` (slice 4)

```rust
impl Scope {
    fn keys(&self) -> Option<&KeyScope>;
    fn selected_pane(&self) -> Option<IdOf<Pane>>;
    /// The selected tab, or the selected pane's tab.
    fn selected_tab(&self) -> Option<IdOf<Tab>>;
}

impl Client {
    /// The keys' tabs, or `GET /tabs` on the CLI.
    async fn scope_tabs<'s>(&self, scope: &'s Scope) -> Result<Cow<'s, Tabs>>;
}
```

`tab_target`, `current_tab`, `pane_at`, `pane_target` and `select_attachment` keep their names and errors, written over these.

#### `ship-client/src/ui/sidebar/tabs.rs` (slice 3)

```rust
/// One visible row, from one walk of the tree under unfolded ancestors.
struct Row {
    tab: IdOf<Tab>,
    depth: u16,
    fold: Fold,
    label: Line<'static>,
}

/// A row's fold marker. Was two flags, one meaningless without the other.
enum Fold {
    /// No children, no marker.
    Leaf,
    Open,
    Folded,
}

pub(in crate::ui) struct Tabs {
    collapsed: HashSet<IdOf<Tab>>,
    /// First drawn row; moved to keep a newly selected row in view.
    offset: usize,
    area: Rect,
    /// As last drawn, for clicks and drop gaps.
    rows: Vec<Row>,
}
```

`Tabs::fold(tab, collapsed: bool)` becomes `collapse(tab)` and `expand(tab)`, matching `Hop`. Replaces the `TreeState` mirror, `items`, the `rendered_at` row rebuild and its dedup, and the marker-column arithmetic copied from the widget. `visible` becomes the same walk. Proposed settings: remove `tui-tree-widget` from the workspace and `ship-client` manifests.

### Untouched

- `vendor/ratatui-ghostty`, except whatever O-3 decides.
- `ship-macros`: `Actor`, `traced` and `named` stay as they are.
- The `RelayBus` itself, pane-event and clipboard delivery, and `LivePanes`.
- `order.rs`'s numbering, the `x-ship-order` protocol and the outbox.
- `protocol.rs`, `model.rs`'s entity fields, `defaults.toml`, `keymap.rs`, `automation.rs`, `sound.rs`, `watch.rs`, `claim.rs`, `body.rs`, `records.rs`, `endpoint.rs`, `config.rs`.
- `ship/src/runlog.rs`, apart from the `unwrap` fixes.
- Docs: no user-visible change, so `README.md` and `docs/site/` stay. `docs/agents/code.md` gains one line naming the new lints (slice 1).

### Open choices and blocking unknowns

- **O-1, settled: hand-drawn sidebar tree (slice 3).** Cyan chose to draw the tree from Ship's own rows and drop `tui-tree-widget`. The widget's model differs from Ship's at every point the sidebar uses: it keys folds by path and opens nothing by default, where Ship keys them by tab ID and opens by default, so a server-side move would reset a fold and a tab gaining its first child would show folded. Its `click_at` selects locally and toggles on a second click, where Ship folds from the marker and leaves selection to the server. Its `scroll_down` lets the last row reach the top. Using it as designed would change those behaviors and save about 25 lines; dropping it keeps them and saves about 85. Ship owns the offset clamp and the scroll-into-view on a new selection in exchange.
- **O-2: the sidebar draw expectations.** Cyan approved the two `too_many_arguments` expectations on `Sidebar::draw` and `Tabs::draw` "for now" in ui-rename. A small view struct would remove them. Default: keep.
- **O-3, settled: the new lints in the vendored crate.** Cargo can't combine `workspace = true` with overrides. The options were to convert the vendor's 32 `let _ =` and one `unwrap`, give the vendor its own lint table, or set the new lints as crate attributes in the five Ship crates instead. On 2026-10-31 Cyan agreed with the recommended default, converting the vendor, when asked to build slice 1.
- **U-1: `Connection::next` borrows.** One future over three `Option` fields should borrow cleanly; confirm at slice 3 that `drive`'s `select!` preconditions on `connection.idle()` compile alongside it.
- **U-2: the attach response's revision.** `exchange` records `x-ship-revision` on every response, including `attach`, which `stream` didn't. It can only equal or trail a `state` event, so it shouldn't stall the lane; confirm at slice 4, or skip recording for streams.
- **U-3: one `ratatui-core`.** `Cargo.lock` has one `ratatui-core` (0.1.2) today, so `ratatui::style::Color` and core's are one type. Slice 1 confirms with `cargo tree -d`.
- **U-4, settled: openspec without spec deltas.** `openspec validate --strict` refuses a change with no deltas unless its `.openspec.yaml` sets `skip_specs: true`. This change sets it, and `openspec validate consolidate --type change --strict` passes.

## Build order

Every slice keeps behavior as it was, so its completion evidence is the regression walk below plus the slice's own checks. Each runs on an owned temporary server, with `XDG_STATE_HOME`, `XDG_CONFIG_HOME` and `XDG_RUNTIME_DIR` pointed at temporary directories, never on a server already running.

**Regression walk:** bare `ship` on a fresh server; `ship tab create --starter`, nested and moved tabs; split right and down, stack, cycle, swap, zoom and unzoom; resize by key and by border drag; drag a tab in the sidebar; rename a tab and a pane; open each menu; `ship pane send --wait` and `ship pane wait`; `ship pane read`; edit the config and see it reload; stop the server under a client and watch it reconnect after a restart; detach; restart with saved tabs and see them restored; `ship server stop`.

**Every slice also runs:** `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo build`, `cargo test --workspace`, and `cloc --include-lang=Rust crates` against the previous slice's count. Checks run on the implementing machine; the other priority platform is reported unverified.

### 1. Shared shapes and lints

- **Files:** workspace `Cargo.toml` (lints). `ship-core`: `tree.rs`, `screen.rs`, `layout.rs`, `command.rs`, `id.rs`, `model.rs`, `geometry.rs`, `error.rs`, `relay.rs`. `ship-server`: `lib.rs`, `attach.rs`, `state.rs`, `state/tree.rs`, `state/layout.rs`, `routes.rs`, `views.rs`, `saver.rs`, `pane.rs`. `ship-client`: `api.rs` (`Resource`), `ui/{mod.rs, agents.rs, attention.rs, draw.rs, mouse.rs, observer.rs, panes.rs, rename.rs, sidebar/mod.rs}`. `ship/src/{main.rs, cli.rs, diagnostics.rs, runlog.rs}`. The vendor per O-3, `docs/agents/code.md`, and the call sites of every replaced helper.
- **Delivers:** KD-5. Screens, styles, schemas and lookups are produced by one definition each, the local cleanups land, no `#[allow]` remains in Ship's crates, and the only expectations left are O-2's.
- **Done when:**
  - The regression walk passes.
  - A styled program (colors, bold, underline, italics) looks the same in a client.
  - `cargo tree -d` shows one `ratatui-core` (U-3).
  - The OpenAPI document diffs only in the `Rect` maximum and is otherwise byte-identical, checked by saving `ship_server::openapi()` before and after.
  - `grep -rn '#\[allow' crates` is empty, and Clippy passes with the new lints.
  - Closing an attach stream (client quits) still detaches its attachment, seen in `ship view list`.
  - The saver still writes after a change and on stop.
  - About 335 code lines fewer.

### 2. Server: replica reads, pane reports and new-pane sizing

- **Files:** `replicas.rs`, `app.rs`, `lib.rs`, `state.rs`, `state/geometry.rs`, `routes.rs`, `views.rs`, `input.rs`, `order.rs`, `pane.rs`, `relay.rs` (the `watch::Sender` impl).
- **Delivers:** KD-1, KD-2 and KD-3. Reads stop going through the actor, the revision has one home, publishing can't fail a commit, one frame is at most one commit, and new panes are sized by `geometry::tab`.
- **Done when:**
  - The regression walk passes.
  - Every response's `x-ship-revision` equals the revision of the replica a following GET returns, over 100 create, set and remove calls from a script.
  - A GET issued right after a write's response sees the write, 100 of 100.
  - Profiled before and after with `SHIP_TRACE`: GET latency while 20 panes print output, and commit latency. GETs no slower; commits no slower.
  - `ship server stop` with a client attached ends its stream with `server_shutdown` and exits within the five-second bound.
  - Counting `publishing replica` debug lines for one `cd` at a zsh and a bash prompt, before and after: after is no more than one per frame.
  - `ship pane wait` and `send --wait` against a program reporting OSC 7501 states give the same results as before.
  - A pane whose program exits with `exit_behavior = keep_failed` and a failing code stays, showing its exit; with `close`, an every-revision feed sees the exit status before the pane goes.
  - For split right, split down, stack onto a single pane, stack onto a stack, stack into a zoomed slot, and stack into a slot hidden by another zoom: the program's first `stty size` equals the published content size, on 80x24 and 200x60 clients.
  - Splitting a pane already at the minimum still refuses with "no space for new pane".
  - About 190 code lines fewer.

### 3. Client loop and sidebar

- **Files:** `ui/nav.rs` and `ui/connection.rs` (new), `ui/mod.rs`, `ui/keys.rs`, `ui/mouse.rs`, `ui/panes.rs`, `ui/sidebar/mod.rs`, `ui/sidebar/tabs.rs`, workspace and `ship-client` manifests.
- **Delivers:** KD-4 and the hand-drawn tab tree (O-1). `Handled` and `Clicked` are gone, navigation reads one `View` per turn, `drive` reads as a dispatcher, and `tui-tree-widget` is gone. The sidebar's mouse routing and the tree's hit-testing change together, so the click path is reworked once.
- **Done when:**
  - The regression walk passes.
  - Every mouse gesture behaves as before: click a tab, fold, drag a tab to each gap kind, drag the sidebar edge and split, wheel over both sidebar sections, click a pane title behind the front, border drags at a junction, right-click with and without passthrough, drags and wheel to a mouse-tracking program.
  - With the 500 ms delay proxy earlier changes used, typed keys and clicks still land in order.
  - Tab next and prev over folded rows, pane next and prev, focus in all four directions with ties going to the most recent pane, collapse and expand on leaves, folded rows and open rows, tab move up, down, in and out at both edges, zoom toggling, and `client.agents.next` all match their current behavior.
  - Stopping the server under a client shows "disconnected, reconnecting", and a restart within 5 s reattaches with the previous selection. A client that falls behind an every-revision feed reattaches. A refused number reattaches without losing the screen.
  - Resizing the terminal sends one area PUT per frame at most, seen in the server's debug log.
  - Side by side with the previous build, the tab tree's indentation, fold markers, selected band, rename accent, zoom flag and rollup icons look the same. Clicks on markers and labels, drop gaps at every depth, wheel scrolling to the end, folding or closing a tab while scrolled to the end, and a newly selected row scrolling into view behave the same.
  - `cargo tree` no longer lists `tui-tree-widget`.
  - About 255 code lines fewer.

### 4. Client API

- **Files:** `ship-client/src/lib.rs`, `api.rs`, `execute.rs`.
- **Delivers:** one request path and one target resolver.
- **Done when:**
  - The regression walk passes.
  - Every `ship tab` and `ship pane` subcommand prints the same JSON as before for the same inputs, including its errors (missing `--pane` outside a pane, unknown IDs, a stopped server's "no server running", a timeout from a server paused with `SIGSTOP`).
  - U-2 is settled.
  - About 75 code lines fewer.

Expected size, estimated and not measured: about 855 code lines fewer by `cloc` (about 6.2% of 13,732). No test code, as `docs/agents/code.md` asks.

The change ships as one pull request. The slices are its build order, not review units.

## Deviation log

### Slice 1

- **`id_map_schema` takes the value type, not a name.** Utoipa's `schema_with` calls a bare path with no arguments, so `id_map_schema::<K, V: ToSchema>()` names the `$ref` from `V::name()`. Call sites read `schema_with = id_map_schema::<Tab, Tab>`.
- **`RectSchema` is public and `#[doc(hidden)]`.** A private struct that is never built needs a `dead_code` expectation, and "Done when" allows only O-2's. The OpenAPI diff is still confined to `Rect`: its four integers lose `maximum: 65535` and gain Utoipa's `format: int32` for `u16`.
- **O-3's conversion stops at the vendor's tests and demo.** The vendor's `src` and examples drop every `let _ =` for `.ok()` and its one `unwrap` for `expect`. Its three integration test files and the demo example also hold about 60 `unwrap`s that the paper's count left out, and Clippy's `allow-unwrap-in-tests` doesn't cover their helper functions. Each of those four crate roots carries one `#![expect(clippy::unwrap_used, reason = ...)]` instead. They are in `vendor/`, so `grep -rn '#\[allow' crates` stays empty.
- **`err!` lost its unused `@external` entry arms.** No caller used `err!(@external code, source)`. The `@external:` modifier stays.
- **`layout::split` and `layout::stack` became private**, because every caller now goes through `layout::place`.
- **`attach` keeps returning `Response`.** Its body is an SSE stream, not JSON. Every JSON handler returns `Json<T>`, or `(StatusCode, Json<T>)` for 201, and the 204 handlers return `StatusCode`.
- **`checked_target` serves the three `server` arms only.** `connect` and bare `ship` need the target's name for the named-server branch, so they keep calling `checked` and now read `Checked`'s fields.
- **Four `Size` conversions, not about seven.** That's all the code had: `state.rs` three times and `draw.rs` once.
- **`order.rs` uses `attachment_not_found`.** It had a seventh copy of the message.
- **Size: 100 code lines fewer, not about 335.** `cloc --include-lang=Rust crates` went from 13,732 to 13,632. No test code changed in `crates`.

### Slice 2

- **`commit` is synchronous.** Publishing no longer awaits the bus, so nothing in `commit` awaits, and its 17 call sites drop `.await`.
- **`Replicas::new` is gone.** `ReplicaSink::new` builds both channels, and `ReplicaSink::subscribe` hands out the read side, as the skeleton's `subscribe` says. `published_tabs` folded into `replica()`, its last caller, and `geometry::halves` became private, since only geometry uses it now.
- **`PaneTask::publish` takes the exit status.** The exit arm passes it in, so the exit and the frame's last title, directory and program changes go out as one `PaneEvent`. The handler closes a pane only on its first change from running to exited.
- **The read routes keep their documented 503.** The OpenAPI document is byte-identical to slice 1's.
- **Stacking into a slot under `MIN_CONTENT` now refuses.** `start_size` checks the new pane in a stack as well as in a split, as its doc asks. Read from the code rather than exercised: before, a stack onto a lone borderless pane only one row tall started its program with zero rows, and now it answers "no space for new pane". Every placement the matrix covers sizes as before.
- **Size: 93 code lines fewer, not about 190.** `cloc --include-lang=Rust crates` went from 13,632 to 13,539.

### Slice 3

- **Size: 83 code lines more, not about 255 fewer.** `cloc --include-lang=Rust crates` went from 13,539 to 13,622. `ui/mod.rs` lost 221 and `keys.rs`, `mouse.rs` and `panes.rs` lost 37 between them, but `ui/nav.rs` (+193) and `ui/connection.rs` (+126) cost more than the code they took over, almost all of it named types, their docs and imports rather than logic. Dropping `tui-tree-widget` saved nothing: keeping the tree's behavior meant drawing, clamping and scrolling rows by hand, so `sidebar/tabs.rs` came out about even (+10). Cyan chose to keep the structure over the line count.
- **The tab tree states two rules where the widget had quirks.** Both are visible. The offset never leaves blank rows below the last row, the stop the wheel already had, so folding or closing a tab while scrolled to the end now scrolls up to fill the section instead of leaving the last row near the top. The selected row comes back into view when it changes or moves, tracked as `Placed { tab, index }`, instead of when its path changes, so moving the selected tab among its siblings also brings it back. `Tabs` keeps that in a `selected` field the skeleton didn't list.
- **The pointer's row is `Pointed`.** `Row` names the drawn row, so `Hit::Row` carries `Pointed { tab, marker }`.
- **No wrapper constructors.** `View` is built as a struct literal with public fields, so `View::new` is gone. `Connection::new(client, request, events)` became `Connection::open(client, request, span)`, which does the first attach and owns the tracing span: on `attached` it swaps the span, starts numbering and resets the backoff together, and `Startup` loses its span field. `attach_after` is private to `connection.rs`.
- **One selection resolution.** `Selected::resolve(tabs, selection)` serves `Observer::selected` and `View::selected`; typing, zoom, rename and the navigation lookups all read it.
- **`reconcile` runs inside `apply`.** Folds, drop feedback and the rename field only go stale when a replica arrives, so `Local::apply` reconciles them there, and the loop has no separate step. Before, rename reconciliation ran before the lane and the other two after it.
- **Three small additions to the skeleton.** `Item::waits_for_input` and `Item::redraws` name the lane loop's two item lists and why they differ. `Step::of(&action)` maps a navigation action to its step. `Observer::resized(area)` returns a named `Resize { attachment, view }`, computed once per loop before the `select!` (a precondition can't hand its value to the branch), which replaces `unsent` and its unnamed tuple.
- **`act` reads the view once.** Turns run only while attached, which brings a replica, so `act` takes one `View` and one match; the replica checks that could never fail are gone. `hop` takes a `Hop`, and `Hop::Select` carries the landing node.
- **U-1 settled.** `connection.idle()` in the tick's precondition compiles beside `connection.next(area)`; `next` documents that it's safe to drop at any await, since the stream, the reconnect and the PUT live in `self`.
- **"Falls behind an every-revision feed" isn't reachable from the client.** The full-screen client attaches with the `latest` feed, which skips revisions instead. `Link::Lost` was exercised by killing the server under a client.
- **A refused area PUT doesn't reattach, as before.** Lane requests and input reattach on `OutOfOrder`; a refused area PUT shows the error and retries with the next number on the next frame. Seen when numbers were claimed from outside the client; unchanged from slice 2 and left for Cyan.
- **Found and fixed outside the slice: the config watcher reloaded in a loop on Linux.** `notify` 8.2 watches inotify `OPEN` and `notify-debouncer-mini` drops the event kind, so every read of the config counted as a save, and each reload read it again: about one reload every 100 ms once the server first read pane defaults, closing menus and rename fields and dropping drags. `watch.rs` now yields only when the file's modified time or size changes. Its own commit; 20 code lines.
- **Verification, Linux only; macOS unverified.** fmt, Clippy, build and tests pass, `cargo tree` no longer lists `tui-tree-widget`, and `grep -rn '#\[allow' crates` is empty. Against the slice 2 build in tmux: the keyboard walk (split, stack, cycle, focus and swap in four directions, zoom, resize by key, tab navigation, collapse and expand, moves at both edges) matched screen for screen, and the mouse walk (tab clicks, folds, drags to each gap kind, sidebar edge and split, wheel over both sections, menus, a title behind the front, border drags at a junction, right-click with and without passthrough, drags and wheel to a mouse-tracking program) matched in 37 of 40 captures, the other three being the offset rule above. Through a 500 ms each-way delay proxy, typing after `alt-n`, `alt-f` after `alt-|`, and typing after a tab click landed in order 9/9. A killed server shows "disconnected, reconnecting" and a restart within 5 s reattaches with the previous selection; a refused number reattaches and keys keep working; 30 terminal widths in 0.1 s sent 2 area PUTs and settled on the right area.
