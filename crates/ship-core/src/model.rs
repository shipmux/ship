use std::{fmt, str::FromStr};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use utoipa::openapi::{Object, ObjectBuilder, Ref};

use crate::{
    AppError, config, err,
    geometry::TabGeometry,
    id::{Id, IdOf, Identified, Prefixed},
    layout::Layout,
    tree::Tabs,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct Tab {
    pub id: IdOf<Tab>,
    #[serde(default)]
    pub name: Option<String>,
    #[schema(schema_with = tabs_schema)]
    pub tabs: Tabs,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<Layout>,
    /// Derived from layout and viewers when published; absent when unviewed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry: Option<Box<TabGeometry>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zoomed: Option<IdOf<Pane>>,
}

/// A leaf running one program in its own terminal. Owns no layout or children.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct Pane {
    pub id: IdOf<Pane>,
    #[serde(default)]
    pub name: Option<String>,
    /// The argv the pane started; the shell's path for the default login shell.
    pub command: Vec<String>,
    /// Starting directory.
    pub cwd: String,
    /// The directory the shell last reported through OSC 7, else, on Linux
    /// and macOS, its program's working directory; absent until known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dir: Option<String>,
    /// The title the program last set; absent when none or cleared.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Give right-clicks on contents to a program that tracks the mouse.
    #[serde(default)]
    pub right_click_passthrough: bool,
    /// Set at creation from `[server.defaults.pane]` unless given. Saved files
    /// from before it existed get the built-in default.
    #[serde(default)]
    pub exit_behavior: ExitBehavior,
    /// Bytes of history the pane keeps; set like `exit_behavior`.
    #[serde(default = "config::builtin_scrollback_bytes")]
    pub scrollback_bytes: usize,
    pub status: PaneStatus,
    /// OSC 7501 records keyed by id, oldest update first; ROOT is the root.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub program: IndexMap<String, StatusRecord>,
}

/// What happens to a pane when its program exits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[cfg_attr(feature = "clap", value(rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
pub enum ExitBehavior {
    /// Close the pane.
    #[default]
    Close,
    /// Keep the pane, showing its last screen.
    Keep,
    /// Close only after code 0 with no signal; keep it otherwise.
    KeepFailed,
}

impl ExitBehavior {
    /// Whether this exit closes the pane; a signal is never a successful exit.
    pub fn closes(self, status: &ExitStatus) -> bool {
        match self {
            Self::Close => true,
            Self::Keep => false,
            Self::KeepFailed => status.code == 0 && status.signal.is_none(),
        }
    }
}

/// The root record's id, which an explicit protocol id cannot claim.
pub const ROOT: &str = "";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct StatusRecord {
    pub state: ReportedState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<BlockedKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msg: Option<String>,
}

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    utoipa::ToSchema,
    strum::IntoStaticStr,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ReportedState {
    Idle,
    Working,
    Done,
    Blocked,
    Error,
}

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    strum::IntoStaticStr,
    utoipa::ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum BlockedKind {
    Permission,
    Question,
    Auth,
}

impl Pane {
    pub fn root_status(&self) -> Option<&StatusRecord> {
        self.program.get(ROOT)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum PaneStatus {
    Running,
    Exited(ExitStatus),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct ExitStatus {
    pub code: u32,
    /// The signal's name when a signal ended the program.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signal: Option<String>,
}

/// Ordered child tabs keyed by ID. Hand-written because the derive inlines
/// `Tab`'s schema for the `IdOf<Tab>` key, recursing without end. Also used by
/// `Replica::tabs`.
pub(crate) fn tabs_schema() -> Object {
    use utoipa::PartialSchema;

    ObjectBuilder::new()
        .property_names(Some(IdOf::<Tab>::schema()))
        .additional_properties(Some(Ref::from_schema_name("Tab")))
        .build()
}

impl Tab {
    pub fn panes(&self) -> impl Iterator<Item = &Pane> {
        self.layout.iter().flat_map(Layout::panes)
    }

    pub fn panes_mut(&mut self) -> impl Iterator<Item = &mut Pane> {
        self.layout.iter_mut().flat_map(Layout::panes_mut)
    }

    pub fn pane(&self, id: IdOf<Pane>) -> Option<&Pane> {
        self.layout.as_ref()?.pane(id)
    }

    pub fn pane_mut(&mut self, id: IdOf<Pane>) -> Option<&mut Pane> {
        self.layout.as_mut()?.pane_mut(id)
    }
}

impl Prefixed for Tab {
    fn prefix() -> &'static str {
        "tab"
    }
}

impl Identified for Tab {
    type Id = Id<Tab>;
}

impl Prefixed for Pane {
    fn prefix() -> &'static str {
        "pane"
    }
}

impl Identified for Pane {
    type Id = Id<Pane>;
}

/// A two-valued attribute's new value, resolved where the edit takes effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(untagged)]
pub enum Switch {
    Set(bool),
    Toggle(Toggle),
}

/// The word `"toggle"`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Toggle {
    Toggle,
}

impl Switch {
    pub fn apply(self, current: bool) -> bool {
        match self {
            Self::Set(value) => value,
            Self::Toggle(_) => !current,
        }
    }
}

impl FromStr for Switch {
    type Err = AppError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "true" => Ok(Self::Set(true)),
            "false" => Ok(Self::Set(false)),
            "toggle" => Ok(Self::Toggle(Toggle::Toggle)),
            _ => Err(err!(
                Validation,
                "expected true, false or toggle, got '{}'",
                value
            )),
        }
    }
}

/// Any selectable entity. Untagged; the prefix decides the variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(untagged)]
pub enum NodeId {
    Tab(IdOf<Tab>),
    Pane(IdOf<Pane>),
}

impl FromStr for NodeId {
    type Err = AppError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.split_once(':').map(|(prefix, _)| prefix) {
            Some("tab") => value.parse().map(Self::Tab),
            Some("pane") => value.parse().map(Self::Pane),
            _ => Err(err!(Validation, "expected tab or pane ID, got '{}'", value)),
        }
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tab(id) => id.fmt(f),
            Self::Pane(id) => id.fmt(f),
        }
    }
}
