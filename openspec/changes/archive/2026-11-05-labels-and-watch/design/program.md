# `ship watch` and tags: program

Status: Locked on 2026-11-04, approved in chat ("LOCK IT"). A first review asked for `Id`'s Display/FromStr derives on `TagName`, whether an existing color type would do, and a `--tag` closer to the other `set` flags; this revision answers all three. Built on the locked [product.md](product.md) and the locked [architecture.md](architecture.md).

## Summary

Six slices:

1. Tags in core and the server, reachable over HTTP.
2. The `--tag` and `--clear-tags` flags.
3. `ship watch`.
4. Tag tokens in agent rows.
5. Multi-line tab entries with `[client.tabs] rows`.
6. The git example and the docs.

Slices 1 to 3 need no UI work, and each one can be checked with `curl`, `ship` and `jq`.

The three product unknowns are still open, and each one blocks part of a slice:

- **OU-1, tag colors on the CLI**, blocks the color flags in slice 2.
- **OU-2, control characters in values**, blocks the value check in slice 1.
- **OU-3, how `watch` ends**, blocks the end of slice 3.

The skeleton marks each spot. If you decide them in this review, the product paper takes an amendment for them, and this paper locks with it.

Six choices in this paper aren't in the papers above and need a look:

- **C1. `ship watch` never starts a server.** It connects like `ship server status`, unlike tab and pane commands, which start a missing server. A plugin that restarts `ship watch` in a loop would otherwise bring back a server you just stopped with `ship server stop`. This also gives `watch` its health answer for free: `checked_target` in `crates/ship/src/main.rs` already returns it.
- **C2. The first line is the whole health answer.** `watch` prints `HealthResponse` as it is (`service`, `protocol_version`, `version`, `log`, `socket`) instead of a new one-field type. FR-001 only needs `protocol_version` in it, and the code rules say to reuse a named shape. KD-3 wrote the narrower `{"protocol_version": N}`, so this needs your OK.
- **C3. Each later line is a bare `Replica`, not the SSE envelope.** `{"incarnation":…,"revision":…,"tabs":…,"viewers":…}` rather than `{"type":"state","data":{…}}`. Then `jq .tabs` works on every line. The envelope would also give the seed line (`attached`, with `data.replica`) a different path from the rest (`state`, with `data`).
- **C4. Color names are the ANSI names.** They are `black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan` and `white`, plus `bright_black` to `bright_white`, as tmux and the SGR codes 30–37 and 90–97 name them. Ratatui's names (`gray`, `dark_gray`, `light_red`) shift the meaning of `white`, so they stay behind a `From` into ratatui's `Color`.
- **C5. Tab rows join items with ` · `,** as the locked product mockup draws `main · 2`. Agent rows keep their single space, where `● claude working` already reads well.
- **C6. `--clear-tags` conflicts with `--tag`.** A merge patch can't say "remove all, then set these" in one object: `tags: null` and a `tags` map exclude each other. Clap refuses the pair. A script that wants a clean slate runs two `set` calls.

This is a plan written in the document only. None of the code below exists in the working tree, and none of it has been compiled.

## Rationale

Shape A makes tags plain attributes, so the skeleton mostly extends what names already use:

- **One new core module, `ship-core/src/tag.rs`,** owns every tag type and the merge-patch step. `TagName` is a validated newtype with `FromStr` and `Display`, serialized through `serde_as` with `DisplayFromStr`, so an invalid name can't be built from the CLI, HTTP or config. `Tag` is one shape for output and input, so a script learns it once. `TagPatch` is the PATCH form, and `patch()` applies it.
- **Server:**
  - Each existing `Set` handler gains one `if let` that calls `Tags::patch` on its copy before comparing. The commit-on-change rule then covers tags too.
  - The saver clears tags beside `program`.
  - Nothing in the attach stream changes.
- **Client library:** `observe()` in `automation.rs` becomes public and takes a feed. It is already the observer attach `ship pane wait` uses, so `watch` adds no new attach code.
- **Binary:** `crates/ship` gains `ship watch`. It prints lines through a small writer that treats a closed pipe as a normal end, so `ship watch | head -1` exits cleanly.
- **Sidebar tags:** a new `ui/sidebar/tags.rs` holds the two things both sidebar sections share: the tab fallback lookup and turning a tag into a span. The color mapping lives on `NamedColor` in core, which already depends on `ratatui-core`; the architecture's "core stays free of ratatui" was wrong about that, and nothing else in it depends on the sentence.
- **Agent rows:** agents' `Token` gains `Tag(TagName)`.
- **Tab rows:**
  - A new `TabToken { Name, Tag(TagName) }` reads `"name"` and `{ tag = "…" }` with serde's default enum format, as `Sound` already reads `"off"` and `{ builtin = "done" }`.
  - `tabs.rs` keeps `walk()` per tab, renamed `Entry`, so `visible()` and navigation don't change. `draw` expands each entry into one `Row` per line (KD-5), and an explicit `Part` says whether a row is the entry's head or a later line.
- **Where the tab rows config lives:** the `Sidebar` keeps `TabSettings` through `configure`, as it already keeps the width, so `draw` takes no new argument. `AgentSettings` stays in `Local` because sounds and delays read it too.

## Skeleton map

### Added

#### `crates/ship-core/src/tag.rs` (slice 1; `parse_edit` in slice 2)

```rust
//! Tags: short display text that scripts put on tabs and panes.

use std::{
    fmt,
    ops::{Deref, DerefMut},
    str::FromStr,
};

use indexmap::IndexMap;
use ratatui_core::style::Color;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};
use utoipa::ToSchema;

use crate::prelude::*;

/// Any non-empty string without `=` or control characters (FR-006).
/// `.` means nothing to Ship; `plugin.key` is a documented convention.
/// No serde derives: fields holding one use `serde_as` with
/// `DisplayFromStr`. The schema derive reads a one-field tuple struct as
/// its field, a string.
#[derive(Clone, Debug, PartialEq, Eq, Hash, ToSchema)]
pub struct TagName(String);

impl TagName {
    pub fn as_str(&self) -> &str;
}
impl FromStr for TagName { type Err = AppError; } // err!(Validation, …)
impl fmt::Display for TagName {}

/// A tag's value and optional colors. One shape in `get` output, PATCH
/// bodies and `watch`. Writing a tag replaces all three (FR-009).
/// An entity shape, so its optional fields skip per field like `Pane.dir`;
/// `skip_serializing_none` stays on `protocol.rs` bodies. It is also PATCH
/// input, so `present` refuses `null` colors as the code rules ask.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct Tag {
    /// OU-2: what a value may contain is open. See "Blocking unknowns".
    pub value: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::protocol::present"
    )]
    #[schema(nullable = false)]
    pub fg: Option<NamedColor>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::protocol::present"
    )]
    #[schema(nullable = false)]
    pub bg: Option<NamedColor>,
}

/// The 16 terminal colors, by their ANSI names (C4). Its own enum because
/// both colors core already has accept more than FR-007 allows: `screen::Color`
/// takes any palette index or RGB, and ratatui's `Color` takes hex, indexes
/// and `reset`, under names where `white` means bright white.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum NamedColor {
    Black, Red, Green, Yellow, Blue, Magenta, Cyan, White,
    BrightBlack, BrightRed, BrightGreen, BrightYellow,
    BrightBlue, BrightMagenta, BrightCyan, BrightWhite,
}

/// Onto ratatui's names: `white` is `Gray`, `bright_black` is `DarkGray`,
/// `bright_white` is `White`. Here because core already depends on
/// `ratatui-core`, and the client then needs no mapping of its own.
impl From<NamedColor> for Color {}

/// A tab's or pane's tags. `IndexMap` like `Pane.program`; its equality
/// ignores order, so re-setting a tag in place is still a no-op. A named
/// type so the `serde_as` key adapter is written once, here, rather than on
/// `Pane` and `Tab`; it derefs to the map.
#[serde_as]
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
pub struct Tags(#[serde_as(as = "IndexMap<DisplayFromStr, _>")] IndexMap<TagName, Tag>);

impl Deref for Tags { type Target = IndexMap<TagName, Tag>; }
impl DerefMut for Tags {}

impl Tags {
    /// Apply a PATCH body's `tags`: `None` (`tags: null`) removes all.
    pub fn patch(&mut self, patch: Option<TagPatch>);
}

/// PATCH's `tags`: a tag sets its name, `null` removes it, and an
/// absent name is unchanged (RFC 7396). A named type for the same reason as
/// `Tags`, and because it must be: `serde_as` on `SetPane`'s
/// `Option<Option<…>>` reads `tags: null` as absent, losing "remove all",
/// so the field keeps `double_option` and the adapter lives in here.
#[serde_as]
#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
pub struct TagPatch(#[serde_as(as = "IndexMap<DisplayFromStr, _>")] IndexMap<TagName, Option<Tag>>);

/// `--tag` entries collect into a patch; the last occurrence of a name wins.
impl FromIterator<(TagName, Option<Tag>)> for TagPatch {}

/// One `--tag` flag as the patch entry it stands for: `NAME=VALUE` sets a
/// plain tag, a bare `NAME` removes it. Split at the first `=`, which a
/// name can't contain. Clap's own `KEY=VALUE` idiom: a `value_parser`
/// function filling a `Vec` of pairs. Colorless until OU-1 is decided.
pub fn parse_edit(arg: &str) -> Result<(TagName, Option<Tag>)>;
```

`TagName` has `FromStr` and `Display` only, and is serialized through `serde_as` with `DisplayFromStr` at its four field sites: inside `Tags` and `TagPatch` here, and the two sidebar token variants. A throwaway probe (serde_with 3, outside the repo) confirmed that `null` stays distinct from absent with this layout, and that a bad name fails deserialization with `FromStr`'s error. (`ServerName` uses a `try_from` pair and `Id` the `SerializeDisplay` derives; aligning them is out of scope.)

`Tags::patch` is about eight lines of `insert` and `shift_remove`. The `json-patch` crate's `merge` works on `serde_json::Value`, and using it would mean converting a typed `Tags` there and back. The typed form is shorter, so nothing is rented here.

#### `crates/ship-client/src/ui/sidebar/tags.rs` (slice 4)

```rust
//! Tags as the sidebar draws them, shared by the tab and agent sections.

use ratatui::text::Span;
use ship_core::{
    tag::{Tag, TagName},
    model::Tab,
};

/// The tab's own tag, else its first pane's (FR-015), the way
/// `tab_auto_label` falls back for names. Display only.
pub(super) fn on_tab<'a>(tab: &'a Tab, name: &TagName) -> Option<&'a Tag>;

/// The value in its colors; `None` for an empty value, so it disappears
/// with its separator like an empty agent token (FR-017). Colors go on
/// this span only (FR-018), through `NamedColor`'s `From` in core.
pub(super) fn span(tag: &Tag) -> Option<Span<'static>>;
```

An agent's own pane tag is `agent.pane.tags.get(name)`, read inline. A one-line wrapper around it would only forward the call.

#### `examples/plugins/git-branch.sh` (slice 6)

```sh
#!/bin/sh
# Tags each pane with the git branch of its directory as git.branch, and
# removes the tag outside a checkout. Needs ship, jq and git.
#
# Reads `ship watch`. The first line is the health answer: warn if its
# protocol_version isn't the one this script was written for. For every
# later line (a replica), for each pane with a `dir`:
#   branch = git -C "$dir" branch --show-current (short commit when detached)
#   compare with the pane's tags["git.branch"].value in that replica
#   if they differ: ship pane set --pane "$id" --tag git.branch="$branch"
#   outside a checkout, with a tag present: ship pane set --pane "$id" --tag git.branch
# The comparison against the replica means the script keeps no state of its
# own, and an unchanged branch costs one git call and no write.
```

#### `examples/plugins/README.md` (slice 6)

It says how to run the example, `sh examples/plugins/git-branch.sh &`, and what it needs (`jq`, `git`). It gives the rows config that shows the branch, `[client.tabs] rows = [["name"], [{ tag = "git.branch" }]]`. It also says how a plugin is just a client: `ship watch` in, `ship pane set` out, ignore fields it doesn't know.

#### `docs/site/scripting/writing-a-plugin.md` (slice 6, a new page)

Proposed scope, for your approval as `docs/agents/docs.md` asks:

```markdown
<!--
Scope: Following Ship from a script with `ship watch`, and showing text on tabs and panes with tags: the conventions (ignore unknown fields, `plugin.key` names, `ship.*` reserved, no compatibility promise before 1.0) and the git example.
Not here: the sidebar rows settings (configuration.md), JSON shapes (openspec/specs).
-->
```

### Replaced

- **`crates/ship-core/src/lib.rs`** (slice 1): gains `pub mod tag;`.
- **`crates/ship-core/src/model.rs`** (slice 1):
  - `Pane` gains `tags: Tags` after `program`, and `Tab` gains it after `zoomed`.
  - Both use `#[serde(default, skip_serializing_if = "Tags::is_empty")]` (through `Deref`), so saved files from before this change still load (FR-005). No `serde_as` on `Pane` or `Tab`; `Tags` carries it.
- **`crates/ship-core/src/protocol.rs`** (slices 1 and 3):
  - `SetPane` and `SetTab` gain `tags: Option<Option<TagPatch>>` with `#[serde(default, with = "double_option")]`, the `name` pattern: absent, `null` and a map are the three cases (FR-010). No `serde_as` here; `TagPatch` carries it.
  - `present` becomes `pub(crate)` so `Tag` can refuse `null` colors.
  - `PROTOCOL_VERSION` goes from 21 to 22 (KD-8).
  - `FeedMode` gains `#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]` for `--feed` (slice 3).
- **`crates/ship-core/src/command.rs`** (slice 2): `tab::Set` and `pane::Set` each gain these fields. The other `set` flags carry the PATCH field's own type, so `execute` only moves them across. A map can't come from repeated flags that way, so `tags` carries the patch's entries, and `execute` collects them into the map:

  ```rust
  /// Set a tag with NAME=VALUE, or remove one with a bare NAME; repeatable
  #[cfg_attr(
      feature = "clap",
      arg(long = "tag", value_name = "NAME[=VALUE]", value_parser = tag::parse_edit)
  )]
  #[serde(skip)] // KD-6: TOML has no null, so config can't remove one
  pub tags: Vec<(TagName, Option<Tag>)>,
  /// Remove every tag
  #[cfg_attr(feature = "clap", arg(long, conflicts_with = "tags"))] // C6
  #[serde(skip)]
  pub clear_tags: bool,
  ```

  OU-1 adds the color flags here once it is decided.
- **`crates/ship-server/src/state.rs`** (slice 1):
  - `Set<Pane, SetPane>` gains `if let Some(patch) = set.tags { pane.tags.patch(patch) }` before its `&pane != before` check.
  - `Set<Tab, SetTab>` builds `tags` beside `name` and `zoomed`, adds `tags != tab.tags` to its change test, and assigns the new tags in the commit. That is FR-011's one state per request, with nothing published for a no-op.
  - `restore_tab` sets `tab.tags` to empty beside `tab.geometry = None`. Restored panes are rebuilt from `NewPane` and lose theirs already, but a tab is reused as loaded, so tags in a hand-written saved file would otherwise survive the restart (FR-012).
  - The `Pane { … }` literals at about lines 354 and 957 and the `Tab { … }` literal at about line 497 gain `tags: Tags::default()`.
- **`crates/ship-server/src/state/geometry.rs`** (slice 1): `placeholder`'s `Pane` literal gains `tags`.
- **`crates/ship-server/src/saver.rs`** (slice 1): `blank()` adds `tab.tags.clear()` and `pane.tags.clear()` beside `program` (FR-012). The saver compares projections, so tag churn writes nothing to disk.
- **`crates/ship-client/src/execute.rs`** (slice 2): the `tab::Set` and `pane::Set` arms destructure the new fields and pass `tags: tag_patch(tags, clear_tags)`, a private helper:

  ```rust
  /// `--clear-tags` is `Some(None)`; entries collect into one map, the
  /// last occurrence of a name winning; neither leaves tags alone.
  fn tag_patch(entries: Vec<(TagName, Option<Tag>)>, clear: bool) -> Option<Option<TagPatch>>;
  ```

- **`crates/ship-client/src/ui/nav.rs`** (slice 2): `set_zoom`'s `tab::Set` literal swaps `name: None` for `..Default::default()`, as `rename.rs` already does.
- **`crates/ship-client/src/automation.rs`** (slice 3): `observe` becomes public and takes the feed. Both waits pass `FeedMode::Every`.

  ```rust
  /// An observer: no selection or area, so it moves and sizes nothing and
  /// gets no screens or clipboard (FR-003).
  pub async fn observe(&self, feed: FeedMode) -> Result<impl Stream<Item = Result<SseEvent>> + use<>>;
  ```

- **`crates/ship/src/cli.rs`** (slice 3): `Command` gains `Watch(WatchArgs)`:

  ```rust
  /// Print the server's state as JSON lines, one per change, for plugins
  Watch(WatchArgs),

  #[derive(Args)]
  pub struct WatchArgs {
      /// latest: the newest state, skipping ones a slow reader missed.
      /// every: each state in order, ending if the reader falls behind
      #[arg(long, value_enum, default_value_t)]
      pub feed: FeedMode,
  }
  ```

- **`crates/ship/src/main.rs`** (slice 3): `dispatch` routes `Command::Watch` through `checked_target` (C1) and passes the client and health to `commands::watch`.
- **`crates/ship/src/commands.rs`** (slice 3):

  ```rust
  /// `ship watch`: the health answer (C2), then each replica the observer
  /// receives (C3), one JSON line each. A closed stdout is a normal end;
  /// the stream ending is an error (FR-004; its form waits on OU-3).
  pub async fn watch(client: &Client, health: &HealthResponse, feed: FeedMode) -> Result<()>;

  /// One JSON line, flushed by stdout's line buffering. `println!` would
  /// panic on a closed pipe.
  fn line(out: &mut impl Write, value: &impl Serialize) -> io::Result<()>;
  ```

- **`crates/ship-client/src/keymap.rs`** (slices 4 and 5):
  - `Token` drops `Copy`, gains `#[serde_as]` and `Tag(#[serde_as(as = "DisplayFromStr")] TagName)`.
  - `ClientSettings` gains `tabs: TabSettings`, parsed from a new `TabsFile` under `Client` with `deny_unknown_fields`.

  ```rust
  /// `[client.tabs]` with defaults applied. Replaced whole on reload.
  pub struct TabSettings {
      /// The lines of a tab's entry in the sidebar.
      pub rows: Vec<Vec<TabToken>>,
  }

  /// One value in a tab's entry: `"name"` or `{ tag = "git.branch" }`.
  #[serde_as]
  #[derive(Clone, Debug, Deserialize)]
  #[serde(rename_all = "snake_case")]
  pub enum TabToken {
      /// The tab's name, else its automatic tag, with the zoom marker.
      Name,
      Tag(#[serde_as(as = "DisplayFromStr")] TagName),
  }
  ```

  An unknown bare string fails as an unknown variant, which covers FR-017's config error.
- **`crates/ship-client/src/ui/sidebar/agents.rs`** (slice 4): `row` and `value` take `&Token`. `value` gains `Token::Tag(name) => tags::span(agent.pane.tags.get(name)?)`. A tag's span keeps its colors under the band, because `Paragraph`'s style is the base and span styles patch over it.
- **`crates/ship-client/src/ui/sidebar/mod.rs`** (slices 4 and 5):
  - Gains `mod tags;`.
  - `new` and `configure` take `TabSettings` beside the width and hand it to the tabs section.
- **`crates/ship-client/src/ui/mod.rs`** (slice 5): `Sidebar::new` and `reload`'s `configure` call pass `settings.tabs`.
- **`crates/ship-client/src/ui/sidebar/tabs.rs`** (slice 5):

  ```rust
  pub(in crate::ui) struct Tabs {
      collapsed: HashSet<IdOf<Tab>>,
      offset: usize,
      area: Rect,
      /// Every visible line as last drawn, one or more per entry (KD-5).
      rows: Vec<Row>,
      selected: Option<Placed>,
      /// `[client.tabs]`, from `Sidebar::configure`.
      settings: TabSettings,
  }

  /// One visible tab, from one walk of the tree under unfolded ancestors.
  /// Was `Row`; `visible()` and navigation read these.
  struct Entry {
      tab: IdOf<Tab>,
      depth: u16,
      fold: Fold,
      /// Among its siblings, for the automatic tag.
      position: usize,
  }

  /// One drawn line of an entry.
  struct Row {
      tab: IdOf<Tab>,
      depth: u16,
      part: Part,
      text: Line<'static>,
  }

  /// Which line of its entry a row is.
  #[derive(Clone, Copy, PartialEq, Eq)]
  enum Part {
      /// The first: carries the fold marker and the rollup icon.
      Head(Fold),
      /// A later line, indented to the head's text.
      Tail,
  }

  impl Tabs {
      pub fn configure(&mut self, settings: TabSettings);
      fn walk(&self, tabs: &TabTree) -> Vec<Entry>;
  }

  /// The entry's lines: one per `rows` line with any value, items joined
  /// by ` · ` (C5). At least one line, blank if nothing has a value, so
  /// every visible tab can still be clicked and stepped to.
  fn lines(tab: &Tab, position: usize, rows: &[Vec<TabToken>]) -> Vec<Line<'static>>;

  /// One item's spans: the name with ` Z` when zoomed, or a tag through
  /// `tags::on_tab`.
  fn value(tab: &Tab, position: usize, token: &TabToken) -> Option<Vec<Span<'static>>>;
  ```

  How the line-based code changes:

  - `row_at`: only a `Head` row has a marker, and a click on a `Tail` row selects its tab.
  - `gap`: the pointer's index snaps up to its entry's head, so drops land only between entries.
  - `draw`: keeps the whole selected entry in view, not just its head. It sets the band before writing the text, so a tag's `bg` survives on the selected row, which settles the architecture's band risk. It draws the gutter icon on `Head` rows only.
- **`crates/ship-core/src/defaults.toml`** (slices 4 and 5): see "Proposed settings".
- **`docs/site/scripting/README.md`** (slice 6): links the new page.
- **`docs/site/configuration.md`** (slice 6): `[client.tabs] rows`, plus the tag item in `[client.agents] rows`.
- **`GLOSSARY.md`** (slice 6): gains **Tag**.

### Moved

None.

### Proposed settings

`crates/ship-core/src/defaults.toml`:

```toml
[client.tabs]
rows = [["name"]]                    # lines of: name, or { tag = "..." }

[client.agents]
rows = [["icon", "app", "state"], ["tab"]]  # lines of: icon app state tab pane title msg, or { tag = "..." }
```

One manifest change: `serde_as` adapts `IndexMap` only with serde_with's `indexmap_2` feature, which the workspace doesn't enable today (slice 1):

```toml
# Cargo.toml
serde_with = { version = "3", features = ["indexmap_2"] }
```

`indexmap` is already in `ship-core`, and clap is behind its existing `clap` feature.

### Untouched

- **The attach stream:** the server's attach route, `Progress::view`, the SSE encoding and `FeedMode`'s semantics. `watch` reads the stream as it is.
- **Program status:** `records.rs`, attention, notifications and sounds (FR-013).
- **Create bodies:** `NewPane`, `NewTab` and `[server.defaults.pane]`. Nothing asks for tags at creation, and a tag can follow the create call.
- **Menus and keys** (KD-6).
- **`draw.rs`:** `tab_label` and `pane_label`, where "label" means a display name. Tags no longer share the word, so nothing there changes.

### Blocking unknowns

- **OU-1** (slice 2): the color flags. If you take the recommendation, `--tag-fg NAME=COLOR` and `--tag-bg NAME=COLOR` become two more `Vec<(TagName, NamedColor)>` fields with their own `value_parser`, and `NamedColor` gains `strum::EnumString`. `tag_patch` then attaches them to the `--tag` of the same name, and a color for a name with no `--tag` is a usage error.
- **OU-2** (slice 1): the value rule. If you take the recommendation, `value` becomes a `TagValue` newtype that refuses control characters, in the same `try_from` style as `TagName`, and the `Set` handlers strip bidirectional characters with `records.rs`'s `disarm`, made `pub(crate)`. Until you decide, values are stored as sent, and slice 1's check leaves the value rule out.
- **OU-3** (slice 3): the end of `watch`'s output. If you take the recommendation, `Ended` prints as a last line, `{"reason":"server_shutdown"}` or `{"reason":"lost"}`, and then `watch` returns an `Unavailable` error, so it exits non-zero with the reason on stderr. A stream cut without `Ended` is a `Network` error with no last line, either way.

## Build order

Checks run on the machine doing the work. A check named for a platform that machine isn't is reported unverified there, per `AGENTS.md`. Each slice passes `cargo fmt --check`, `cargo clippy --workspace` and `cargo build`. The change ships as one pull request.

### 1. Tags in core and the server

Files: `ship-core/src/tag.rs` (all but `parse_edit`), `ship-core/src/lib.rs`, `ship-core/src/model.rs`, `ship-core/src/protocol.rs` (`SetPane`, `SetTab`, `present`, version 22), `ship-server/src/state.rs`, `ship-server/src/state/geometry.rs`, `ship-server/src/saver.rs`.

Delivers:

- A `PATCH /api/v0/panes/{id}` with `{"tags":{"git.branch":{"value":"main","fg":"green"}}}` makes `ship pane get` show the tag.
- `{"tags":{"git.branch":null}}` removes it, and `{"tags":null}` removes all.
- The same works on tabs, with `ship tab list`.
- A name with `=` or a control character, or `"fg":"pink"`, gets a 400 with a pointer, and nothing changes.
- Sending the same body twice leaves the revision header unchanged the second time.
- `name` and `tags` in one body move the revision by one.
- After `ship server stop && ship`, the tags are gone.

Checks: those requests with `curl --unix-socket`, reading `x-ship-revision` on each response. Then restart a server whose saved file predates the change, and confirm it restores.

### 2. `--tag` and `--clear-tags`

Files: `ship-core/src/tag.rs` (`parse_edit`), `ship-core/src/command.rs`, `ship-client/src/execute.rs`, `ship-client/src/ui/nav.rs`.

Delivers P2 from the CLI:

- `--tag git.branch=main --tag git.ahead=2` sets both.
- A bare `--tag git.ahead` removes one.
- `--tag git.ahead=` keeps an empty value.
- `--clear-tags` removes all, and with `--tag` in the same call, clap refuses it (C6).
- A repeated name takes its last value.
- A `server.pane.set = { tags = … }` in config fails `ship config check` as an unknown field.

Checks: each P2 scenario with `ship pane set` and `ship tab set`, reading `get` after each. Also `ship pane set --help` and `ship config check` on a config holding `tags`.

### 3. `ship watch`

Files: `ship-core/src/protocol.rs` (`FeedMode` clap derive), `ship-client/src/automation.rs`, `ship/src/cli.rs`, `ship/src/main.rs`, `ship/src/commands.rs`.

Delivers P1:

- `ship watch | jq -c .` prints the health line and then a replica per change. Creating and renaming a tab from another terminal shows each change arrive.
- No line holds screens or clipboard text.
- `ship watch | head -1` exits 0.
- `ship watch` with no server running fails without starting one (C1).
- `--feed every` prints each revision in order, so a burst of ten renames shows ten consecutive revisions.
- `ship server stop` ends `watch` per OU-3.

Checks: those runs on Linux and macOS. For SC-003, run `benchmarks/bench.py --mux ship --workload type_idle type_agent` with five `ship watch >/dev/null` processes attached to its server and with none, five reps each, and compare echo latency with `--summarize`. If the harness can't reach its server's name from outside, a flag for it goes into `bench.py`, which is outside the line budget.

### 4. Tags in agent rows

Files: `ship-client/src/ui/sidebar/tags.rs`, `ship-client/src/ui/sidebar/mod.rs` (`mod tags`), `ship-client/src/ui/sidebar/agents.rs`, `ship-client/src/keymap.rs` (`Token::Tag`), `ship-core/src/defaults.toml` (agents comment).

Delivers:

- With `[client.agents] rows = [["icon", "app", "state", { tag = "git.branch" }], ["tab"]]`, an agent whose pane has `git.branch` shows it after its state.
- A pane without it shows the line as before.
- A tag with `fg = "white"` and `bg = "red"` keeps those colors on the selected agent's band.
- `rows = [["nope"]]` fails `ship config check`.

Checks: set tags by hand with slice 2's flags and look, selected and not. Also confirm the `tab` token still shortens on a narrow sidebar.

### 5. Multi-line tab entries

Files: `ship-client/src/keymap.rs` (`TabSettings`, `TabToken`, `TabsFile`), `ship-core/src/defaults.toml` (`[client.tabs]`), `ship-client/src/ui/sidebar/mod.rs` (`new`, `configure`), `ship-client/src/ui/sidebar/tabs.rs`, `ship-client/src/ui/mod.rs`.

Delivers P3:

- With `rows = [["name"], [{ tag = "git.branch" }]]`, a tab shows its branch under its name, else its first pane's (FR-015), while `ship tab get` shows only the tab's own.
- A tab with neither keeps one line.
- With no `[client.tabs]` in the file, the sidebar is unchanged (SC-004).

Checks, by hand on a tree three levels deep with some entries one line and some two, by clicking and dragging:

- Clicking either line of an entry selects its tab, and only a head's marker folds.
- Dropping onto the second line of an entry lands above that entry, never inside it.
- Wheel scrolling stops at the last line.
- Stepping through tabs with the keys keeps the whole selected entry in view.
- A `bg` tag stays visible on the selected entry.
- With the default config, compare a screenshot of the sidebar before and after the change.

### 6. The git example and the docs

Files: `examples/plugins/git-branch.sh`, `examples/plugins/README.md`, `docs/site/scripting/writing-a-plugin.md`, `docs/site/scripting/README.md`, `docs/site/configuration.md`, `GLOSSARY.md`.

Delivers P4 and FR-019 and FR-020:

- With the example running, a pane in a checkout carries `git.branch`.
- `cd` into another repo or `git switch` updates it when the prompt returns.
- `cd /tmp` removes it.
- The README and the new page say how to run it and show it.

Checks:

- Run the example through a day of normal use on Linux, and on macOS, where `dir` comes from OSC 7 or the process's directory, to settle SC-002.
- Run `shellcheck` on the script.
- The site build strips the new page's scope comment without complaint.

## Deviation log

Slice 1:

- **`Tags` and `TagPatch` write their schemas by hand, and `TagName` has no `ToSchema`.** utoipa's derive can't put `TagPatch`'s nullable `Tag` (a `oneOf`) under `additionalProperties`, and on `Tags` it emits a qualified map path that trips `unused_qualifications` outside any item an `expect` can reach. Each impl is a few lines building an object of `Tag` refs, nullable for the patch. With the map schemas written out, nothing reads `TagName`'s.
- **`Tags` has an inherent `is_empty`.** `skip_serializing_if = "Tags::is_empty"` names a path, which doesn't go through `Deref`.
- **`execute.rs` passes `tags: None` in slice 1.** The client builds `SetTab` and `SetPane` literals, so the workspace doesn't compile without the field; slice 2 replaces both with `tag_patch`.
- **OU-2's value rule lives in `TagValue`'s `FromStr`, read through `serde_as` like `TagName`, not in the `Set` handlers,** and `disarm` moves from `records.rs` to `ship_core::model` so both callers share it. Parsing is where a value is refused anyway, so stripping there too means the handlers need no access to `TagPatch`'s entries and the change test compares cleaned text.
- **The `Pane` literal near line 957 of `state.rs` is unchanged.** It ends in `..before.clone()`, which already carries the tags.

Slice 2:

- **OU-1 took the recommendation (amendment P2), with the color check in the client rather than clap.** `--tag-fg` and `--tag-bg` are `Vec<(TagName, NamedColor)>` fields parsed by `tag::parse_color`, so a bad color name is a clap usage error. Clap can't compare one flag's values with another's, so a color whose name has no `--tag NAME=VALUE` in the call fails in `tag_patch`, now fallible, as a `validation` error like a missing `--pane`. `TagPatch` gains `tag_mut` for that lookup, and `NamedColor` derives strum's `VariantNames` beside `EnumString` so the usage error lists the 16 names.
- **The `session-structure` delta's Set attribute fields list the color flags.** It named only `--tag` and `--clear-tags`, from before OU-1 was decided.
- **`writing-a-plugin.md` is also listed in `docs/site/README.md`.** The site's sidebar comes from the overview's lists, and its build fails on a page that isn't in one.

Slice 3:

- **Amendment P3 replaces C3 and decides OU-3: `watch` forwards every attach event as it arrives, not bare replicas.** Cyan: "the entire api that the client gets should be the same one watch gives back." `commands::watch` writes each `SseEvent` with its `type`/`data` envelope after the health line (C2 stands), and fails after writing `ended`, so the reason is on stdout and the exit is non-zero. Screen and clipboard events would be forwarded too; an observer gets none (FR-003).
- **`http_error` in `ship-client/src/lib.rs` no longer maps reqwest's decode kind to `serialization`.** reqwest 0.13 labels every failed body read as decode, including an attach stream cut by a killed server, and every caller decodes JSON itself, so the kind only ever meant a lost connection. It is now `network`, which also fixes `ship pane wait`'s report for a cut stream.
- **`crates/ship` depends on `futures-util`** (already a workspace dependency) for `StreamExt::next`.
- **The `--feed` help is one line.** `FeedMode`'s variant docs already describe each value, and clap prints them.
- **`bench.py` gains `--watchers N`,** which starts N `ship watch --server-url` processes beside Ship's server for each run and kills them at its end. Each run's server lives in a private XDG tree, so watchers can't reach it from outside.
- **SC-003 holds on macOS: five watchers are within noise of none.** `--workload type_idle type_agent --reps 5`, release build, results in `benchmarks/results/results-20261105-015013.jsonl` (none) and `results-20261105-015210.jsonl` (five). Median p50: `type_idle` 1.51 ms with none and 1.58 ms with five, `type_agent` 1.65 ms and 1.57 ms; the per-run p50s overlap (1.32 to 1.58 against 1.39 to 1.67 idle, 1.57 to 1.78 against 1.48 to 1.69 agent). Linux unverified.
- **Falling behind on `every` was checked with the reader frozen by SIGSTOP and 400 renames, each writing a fresh 40 KB random tag.** Repeating states compress to almost nothing across the stream, so smaller or repeating states never pass the 256-revision ring through the buffers.


Slice 4:

- **`tags::on_tab` waits for slice 5.** Nothing calls it until tab rows exist, and an unused function fails clippy's `-D warnings`. `tags.rs` holds only `span` for now.
- **`ship-client` depends on `serde_with`** (already a workspace dependency) for `Token::Tag`'s `serde_as`.
- **`Token` is no longer `Copy`,** since `TagName` holds a `String`; `row` and `value` take `&Token` as planned.
- **An unknown token's error lists `tag` among the accepted values.** That's serde's message for the enum; a bare `"tag"` fails too, as a variant that needs a name.
- **Amendment P4: agent lines share their width.** Cyan saw a branch tag run off the sidebar unmarked. `row` in `agents.rs` gives each value a width from `shares` (the icon keeps its own, the rest grow a cell at a time in turns, as Herdr's `src/ui/sidebar.rs` at `ee5bfb5` does) and cuts it with `fit`: `shorten` for `tab`, a new `truncate_end` for the rest, keeping the span's style. A zero share drops the value and its separator, where Herdr drops values from the left. The architecture's open "Agent rows' `tab` shortening" note, where a tag took room first, is replaced by this. Slice 5 moves `shares`, `fit` and the two cut helpers into the shared sidebar module for tab rows. Checked in tmux at sidebar widths 26, 22 and 10, including a cut red tag keeping its background on the `…`.

### Slice 5

- **Fitting lives in a new `sidebar/fit.rs`,** not `tags.rs`, since it fits every value and not only tags. `shares` and `fit` became one `fit::line(values, separator, width)`, where each value is a list of `Piece`s, a span and a `Cut` (`Keep`, `Start` or `End`). A value is a list so `name` can carry the zoom marker as a kept piece beside its cut name. Agent rows call it with `" "`, tab rows with `" · "`.
- **`lines` takes a `width`,** which the planned signature lacked: P4 came after it. The width is the text column's minus the entry's indent and marker, so a nested entry gets less room. Tail lines are indented to the head's text, as planned.
- **`Tabs` loses `Default`** for `Tabs::new(settings)`, since `TabSettings` has no default outside `defaults.toml`. The navigation-only `Sidebar` built at startup takes a clone of the settings.
- **Long tab names are now cut with `…`** where they used to run off the edge, as P4 says for every tab line. The default sidebar with short names is unchanged: a capture of the slice 4 build and this one on the same tree matched byte for byte, escapes included (SC-004).
- **The terminal-client delta gains "Fitting tab lines"** for the ` · ` join (C5) and P4 on tab lines, kept apart from "Tab entry lines" because strict validation flags that requirement as too long otherwise.
- Checked in tmux on a three-level tree with one- and two-line entries: tag on the tab, fallback to the first pane with `ship tab get` showing no tags, reload without a restart, clicks on second lines, a tail line's marker column not folding, a drop on `api`'s second line landing above `api`, the wheel stopping at the last line, stepping onto `deep` scrolling both its lines into view, and `fix/auth` keeping white on red inside the selected band. Narrowed to 18 and 13 columns, tab lines read `fea… · 2` and `fix/au…`, and the agent path still shortens from the left.

### Slice 6

- **The example is `examples/plugins/git.sh`, which also tags pull requests** (amendment P5). It replaces the planned `git-branch.sh`. The branch half is as planned, plus the cyan color Cyan asked for after seeing it live; the pull request half is new.
- **A `git switch` reaches the script only when the pane's state changes.** The server publishes a pane when something in it differs, and a prompt returning in the same folder changes nothing by itself. Shells that put the running command in the window title (Cyan's zsh does) change the title twice per command, so the tag follows when the prompt returns, as P4 says. With a shell that doesn't, a branch switch waits until the pane next changes. The example's README says so. Publishing a state at every prompt would make P4 hold for any shell, but it is a server change, left out here.
- **The branch comes from `git symbolic-ref --short -q HEAD`, else `git rev-parse --short HEAD`,** not `git branch --show-current`. `branch --show-current` succeeds with empty output when detached, which would take an extra call to tell apart; this pair is one call on a branch and fails cleanly outside a checkout. Detached heads get no pull request lookup.
- **Panes are found as the `panes` of every `"kind": "slot"` object,** since slots nest in splits and tabs nest in tabs. Each pane becomes one line of id, dir and both tags joined by the unit separator, since tab-separated fields collapse when a tag is empty. A tag is compared as `"VALUE FG"`, so a tag with the right value but the wrong color is rewritten. The script uses `$SHIP_BIN` when set and sends `pane set`'s JSON to `/dev/null`.
- **Pull requests are the script's one piece of state.** `gh pr view BRANCH --json number,state,isDraft,statusCheckRollup` runs at most once per 30 seconds per repository and branch, keyed by the absolute `--git-common-dir`, and the answer is kept in a `mktemp -d` folder removed on exit. A failed lookup other than "no pull requests found" keeps the last answer, so a network blip doesn't clear the tag. Checks roll up as failing if any failed (cancelled and timed out count), else running if any are pending, else passing.
- **A timer re-reads the panes every 30 seconds** with `ship tab list --server-url unix://SOCKET`, the socket taken from the health line, so it can't start a server after the one it follows stops. It runs only when `gh` is signed in. It is a background loop beside the `watch` pipeline, not a second writer into it, because a tick written into the pipe could land inside a state line longer than the pipe's atomic write size.
- **A write can repeat once.** A state can arrive before the script's own `pane set` lands, so the next pass sees the old tag and writes the same value again. That converges on the following state.
- Checked on throwaway servers with zsh in the pane. The branch: `main`, `git switch other` changing at the prompt, `cd` into another repo, a detached HEAD giving the short commit, `cd /tmp` removing the tag, and `cd` back restoring it. The pull request: a `/tmp` clone of this repo on `self-update` got a magenta `◆ #111`, a wrong `git.pr` set by hand was corrected, a branch without a pull request got none, and with the interval cut to 3 seconds a 10-second trace showed the timer's `tab list` and about one `gh` call per interval. Stopping the server ended the script, removed its folder and started no server. The summary filter gave the expected tag for real open (passing, failing, running), draft and closed pull requests on herdrdev/herdr, in both icon styles. Ran live on Cyan's server. `shellcheck` and `sh -n` are clean, and the site build strips the plugin page's scope comment and points the example links at GitHub. Linux unverified.
