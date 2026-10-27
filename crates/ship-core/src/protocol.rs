use std::sync::Arc;

use crossterm::event::{KeyEvent, KeyModifiers, MouseEventKind};
use indexmap::IndexMap;
use serde::{Deserialize, Deserializer, Serialize};
use serde_with::rust::double_option;
use utoipa::{
    PartialSchema, ToSchema,
    openapi::{Object, ObjectBuilder, Ref, RefOr, schema::Schema},
};
use uuid::Uuid;

use crate::{
    command::Direction,
    id::{Attachment, IdOf},
    model::{ExitBehavior, NodeId, Pane, Switch, Tab, tabs_schema},
    screen::{Screen, Size},
    tree::Tabs,
};

pub const DEFAULT_PORT: u16 = 43179;
pub const DEFAULT_SERVER_URL: &str = "http://127.0.0.1:43179";
pub const HEALTH_PATH: &str = "/health";
/// What a Ship server reports as `service` from `/health`.
pub const SERVICE: &str = "ship";
pub const PROTOCOL_VERSION: u32 = 18;
/// Names the attachment a view or input request controls.
pub const ATTACHMENT_HEADER: &str = "x-ship-attachment-id";
/// On every response: the server's revision once the request was handled,
/// at least that of any commit it made. A client that waits for a replica of
/// this revision sees its own changes.
pub const REVISION_HEADER: &str = "x-ship-revision";
/// `{attachment}/{n}`: run this request after the attachment's request
/// `n - 1` has finished. Only the full-screen client sends it.
pub const ORDER_HEADER: &str = "x-ship-order";

/// Server identity and protocol compatibility, not an authentication boundary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct HealthResponse {
    pub service: String,
    pub protocol_version: u32,
    pub version: String,
    /// The log part the server is writing.
    pub log: String,
}

/// One problem with a request: each entry of a 400 `validation` error's
/// `data.errors`.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct Problem {
    /// JSON Pointer into the body, e.g. `/pane/exit_behavior`; absent when
    /// the error has no field path, such as malformed JSON.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pointer: Option<String>,
    pub detail: String,
}

/// PUT /api/v0/server/log: {"set": "<filter>"} or "reset".
#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum LogChange {
    Set(String),
    Reset,
}

/// The running server's filters and current log part.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct LogFilter {
    pub active: String,
    pub startup: String,
    pub file: String,
}

/// POST /tabs body, e.g. `{"at": {"after": "tab:…"}, "starter": "shell"}`.
/// `at` defaults to the end of the top level.
#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct CreateTab {
    #[serde(default = "MoveTab::top")]
    pub at: MoveTab,
    #[serde(default)]
    pub tab: NewTab,
    #[serde(default, deserialize_with = "present")]
    #[schema(nullable = false)]
    pub starter: Option<Starter>,
    /// Select the new tab, or its starter pane, in the same commit.
    #[serde(default, deserialize_with = "present")]
    #[schema(schema_with = attachment_schema, nullable = false)]
    pub select: Option<IdOf<Attachment>>,
}

/// What a new tab starts with.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[serde(rename_all = "snake_case")]
pub enum Starter {
    /// One pane running the configured shell, else the login shell.
    Shell,
}

/// POST /panes body.
#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct CreatePane {
    pub at: PaneAt,
    #[serde(default)]
    pub direction: SplitDirection,
    #[serde(default)]
    pub pane: NewPane,
    /// Select the new pane in the same commit.
    #[serde(default, deserialize_with = "present")]
    #[schema(schema_with = attachment_schema, nullable = false)]
    pub select: Option<IdOf<Attachment>>,
}

/// Split an explicit pane, or the largest pane of a tab (first pane if empty).
#[serde_with::skip_serializing_none]
#[derive(Clone, Copy, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PaneAt {
    Tab(IdOf<Tab>),
    Pane(IdOf<Pane>),
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[serde(rename_all = "snake_case")]
pub enum SplitDirection {
    #[default]
    Right,
    Down,
}

/// The new tab, as a subset of Tab.
#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct NewTab {
    #[serde(default)]
    pub name: Option<String>,
}

/// The new pane, as a subset of Pane. Absent settings take
/// `[server.defaults.pane]`. Also what a restored pane starts from.
#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct NewPane {
    #[serde(default)]
    pub name: Option<String>,
    /// argv; absent means the configured shell, else the login shell.
    #[serde(default, deserialize_with = "present")]
    #[schema(nullable = false)]
    pub command: Option<Vec<String>>,
    /// Absolute directory; absent means the split pane's current directory,
    /// else its starting one, else the home directory.
    #[serde(default, deserialize_with = "present")]
    #[schema(nullable = false)]
    pub cwd: Option<String>,
    #[serde(default, deserialize_with = "present")]
    #[schema(nullable = false)]
    pub right_click_passthrough: Option<bool>,
    #[serde(default, deserialize_with = "present")]
    #[schema(nullable = false)]
    pub exit_behavior: Option<ExitBehavior>,
    #[serde(default, deserialize_with = "present")]
    #[schema(nullable = false)]
    pub scrollback_bytes: Option<usize>,
}

/// POST /tabs/{id}/move body.
#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct MoveTabBody {
    pub at: MoveTab,
}

/// PATCH /tabs/{id}: absent leaves a field, null clears it.
#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct SetTab {
    #[serde(default, with = "double_option")]
    pub name: Option<Option<String>>,
    /// A pane in this tab fills it; null unzooms.
    #[serde(default, with = "double_option")]
    pub zoomed: Option<Option<IdOf<Pane>>>,
}

/// PATCH /panes/{id}: absent leaves an attribute, only name accepts null.
#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct SetPane {
    #[serde(default, with = "double_option")]
    pub name: Option<Option<String>>,
    #[serde(default, deserialize_with = "present")]
    #[schema(nullable = false)]
    pub right_click_passthrough: Option<Switch>,
    #[serde(default, deserialize_with = "present")]
    #[schema(nullable = false)]
    pub exit_behavior: Option<ExitBehavior>,
    #[serde(default, deserialize_with = "present")]
    #[schema(nullable = false)]
    pub scrollback_bytes: Option<usize>,
}

/// Read a present field as a plain value, refusing null.
fn present<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(deserializer).map(Some)
}

/// POST /panes/{id}/swap body: the pane to exchange places with, in the same
/// tab. Adjacency is not required.
#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct SwapPane {
    pub other: IdOf<Pane>,
}

/// POST /panes/{id}/resize body. At most one of `amount` and `to`.
#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct ResizePane {
    /// Which way the edge moves: the pane's edge on that side, or the
    /// opposite edge when that side is the tab's.
    pub direction: Direction,
    /// Cells; absent is one.
    #[serde(default, deserialize_with = "present")]
    #[schema(nullable = false)]
    pub amount: Option<u16>,
    /// Where the edge's border is drawn, from the tab's left or top edge.
    #[serde(default, deserialize_with = "present")]
    #[schema(nullable = false)]
    pub to: Option<u16>,
}

/// POST /panes/{id}/scroll body.
#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct ScrollPane {
    pub direction: ScrollDirection,
    #[serde(default, deserialize_with = "present")]
    #[schema(nullable = false)]
    pub amount: Option<u32>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[serde(rename_all = "snake_case")]
pub enum ScrollDirection {
    Up,
    Down,
    Top,
    Bottom,
}

/// GET /panes/{id}/text query.
#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
#[serde(default, rename_all = "snake_case")]
pub struct ReadPane {
    pub lines: Option<u32>,
    pub unwrap: bool,
    pub ansi: bool,
}

/// Tab placement: exactly one destination. `{"parent": "tab:…"}`,
/// `{"parent": null}` for the top level, `{"before": "tab:…"}` or
/// `{"after": "tab:…"}`.
#[serde_with::skip_serializing_none]
#[derive(Clone, Copy, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MoveTab {
    /// Append under the parent, or to the top level when `null`.
    Parent(Option<IdOf<Tab>>),
    /// Insert before the sibling, under its parent.
    Before(IdOf<Tab>),
    /// Insert after the sibling, under its parent.
    After(IdOf<Tab>),
}

impl MoveTab {
    /// The end of the top level.
    pub fn top() -> Self {
        Self::Parent(None)
    }
}

/// POST /attach body. The selection is what a reconnecting client last had;
/// kept if it still exists, else nothing is selected.
#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct AttachRequest {
    #[serde(default)]
    pub selection: Option<NodeId>,
    /// Where the client draws the viewed tab, after its own chrome. None for
    /// an observer, which has no screen and sizes no tab.
    #[serde(default)]
    pub area: Option<Size>,
    #[serde(default)]
    pub feed: FeedMode,
}

/// Which replicas an attach stream sends. Screens and clipboard are unchanged.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum FeedMode {
    /// The newest replica only; intermediate revisions may be skipped.
    #[default]
    Latest,
    /// Every revision after the seed, in order; ends with `lost` on overflow.
    Every,
}

/// PUT /attach/view body: the client's area only.
#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct ViewInput {
    /// Where the client draws the viewed tab, after its own chrome.
    pub area: Size,
}

/// PUT /views/{attachment}/selection body; `null` selects nothing.
#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct SelectView {
    pub selection: Option<NodeId>,
}

/// Server-owned view of one attachment. Deleted only when the attachment
/// ends; a removed selection falls back to an ancestor or to nothing.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct ViewingRecord {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection: Option<NodeId>,
    /// Where the client draws the viewed tab. Tab sizes derive from it; an
    /// observer has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub area: Option<Size>,
}

/// Complete published state, including each viewed tab's derived geometry.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct Replica {
    /// UUID v7 chosen at server start; a later start sorts higher.
    #[schema(value_type = String, format = "uuid")]
    pub incarnation: Uuid,
    pub revision: u64,
    #[schema(schema_with = tabs_schema)]
    pub tabs: Tabs,
    #[schema(schema_with = viewers_schema)]
    pub viewers: IndexMap<IdOf<Attachment>, ViewingRecord>,
}

/// `Attachment` is a marker with no schema of its own for the derive to pass.
fn attachment_schema() -> RefOr<Schema> {
    IdOf::<Attachment>::schema()
}

fn viewers_schema() -> Object {
    ObjectBuilder::new()
        .property_names(Some(IdOf::<Attachment>::schema()))
        .additional_properties(Some(Ref::from_schema_name("ViewingRecord")))
        .build()
}

/// One event of the attach stream, sent as one SSE `data:` line, e.g.
/// `{"type": "screen", "data": {"pane": "pane:...", "screen": {...}}}`.
// Adjacently tagged: serde reads `type` first and deserializes `data` directly.
// Internally tagged, it buffered every screen cell to find the tag.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum SseEvent {
    /// Always first: the new attachment's ID and the state it starts from.
    Attached(Attached),
    /// A newer complete state, replacing the previous one.
    State(Arc<Replica>),
    /// The latest screen of a pane in the attachment's viewed tab. Sent when
    /// it changes, and for every such pane when the viewed tab changes.
    Screen(PaneScreen),
    /// Text for this client's clipboard: its own mouse selection, or a
    /// program's clipboard write in a pane of its viewed tab.
    Clipboard(ClipboardText),
    /// Always last: why the server is closing the stream. A stream that ends
    /// without it was cut.
    Ended(Ended),
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct PaneScreen {
    pub pane: IdOf<Pane>,
    pub screen: Arc<Screen>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct ClipboardText {
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct Ended {
    pub reason: EndReason,
}

/// Why an attach stream ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EndReason {
    /// The server is shutting down. Reattaching may succeed later.
    ServerShutdown,
    /// An every-revision stream fell behind the server's ring.
    Lost,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct Attached {
    #[schema(schema_with = attachment_schema)]
    pub attachment: IdOf<Attachment>,
    pub replica: Arc<Replica>,
}

/// POST /attach/input: frames in the order they should reach their panes.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct InputBody {
    pub frames: Vec<InputFrame>,
}

/// One input frame, e.g. `{"type": "paste", "pane": "pane:...", "text":
/// "ls"}`. View changes use PUT /attach/view instead.
#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InputFrame {
    Key(KeyInput),
    Paste(PasteInput),
    Mouse(MouseInput),
}

/// A key press, encoded on the server against the pane's live terminal modes.
#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct KeyInput {
    pub pane: IdOf<Pane>,
    /// crossterm's own serialization, e.g. `{"code": "Enter", "modifiers": "",
    /// "kind": "Press", "state": ""}`; modifiers read `"SHIFT | CONTROL"`.
    #[schema(value_type = Object)]
    pub key: KeyEvent,
}

/// A pointer relative to the pane's content; drags may be outside it.
#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct MouseInput {
    pub pane: IdOf<Pane>,
    #[schema(value_type = Object)]
    pub kind: MouseEventKind,
    #[schema(value_type = String)]
    pub modifiers: KeyModifiers,
    pub x: i32,
    pub y: i32,
}

/// Text sent as one paste; bracketed when the program asked for it.
#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct PasteInput {
    pub pane: IdOf<Pane>,
    pub text: String,
}
