# Entity and request shapes: program

Status: Locked on 2026-10-25. Built on the locked `product.md` and `architecture.md`, both revised the same day for the `client.pane.zoom` switch and KD-10. Cyan settled Q1 to Q3 and the `leaves_zoom` list in chat ("i agree with oyur deffaults on those questions"), then approved in Plannotator review ("LGTM").

## Superseding decision: unknown request fields

Cyan approved dropping `deny_unknown_fields` and continuing slice 2 ("i see. let's drop it then and keep going"), superseding the product, architecture and program requirements for unknown-field rejection. Remove it from request types in `protocol.rs` and from request-nested `Size`, keeping config strictness. No frame-specific schema dispatch is added. The earlier skeleton attributes, unknown-field checks and flattened-create refusal below are superseded history. Current checks are in `tasks.md`: known-field errors still get 400, unknown fields are ignored, and known valid inputs keep working. Commit remains on hold at Cyan's instruction.

## Superseding decision: schema library

Cyan approved boon for slice 2 ("Perfect! alright! keep going with boon for this slice then!"). This supersedes the original jsonschema dependency, library `ValidatorMap`, `build_map` call and error-filter implementation details below. The public decoder signatures, serde gate, error envelope and lazy initialization contract remain unchanged.

The current `body.rs` skeleton is:

```rust
use std::{collections::HashMap, sync::LazyLock};
use boon::{Compiler, Draft, ErrorKind, SchemaIndex, Schemas, ValidationError};

struct ValidatorMap {
    schemas: Schemas,
    indices: HashMap<String, SchemaIndex>,
}

// Compile generated components once in an in-memory resource, explicitly
// using Draft::V2020_12, at #/components/schemas/{component_name}.
static SCHEMAS: LazyLock<ValidatorMap>;

fn explain(schema: &str, body: &Value) -> Vec<Problem>;
fn shape_error_at(error: &ValidationError<'_, '_>, here: &str) -> bool;
fn flatten(error: &ValidationError<'_, '_>, out: &mut Vec<Problem>);
```

`explain` looks up the component index and calls `Schemas::validate`. `flatten` walks boon's causes, suppresses alternatives that fail on the enclosing shape when relevant alternatives remain, unwraps groups/references and splits required-property errors into field pointers. There is no request-specific dispatch.

Use `boon = "0.6.1"` in workspace dependencies and `boon.workspace = true` in ship-server; retain `serde_path_to_error.workspace = true`. Cargo regenerates the lockfile. Record the clean comparison and final checks in `tasks.md`. Commit remains on hold at Cyan's instruction.

## Rationale

This is a document-only plan. The skeleton below names real paths and shows types and signatures, but no file has been created and nothing here has been compiled. Snippets show declarations, not bodies, and need not compile on their own.

The skeleton follows shape A. `ship-core` owns every wire type, so the shape rules (one named type per body, nested `pane`/`tab`, `present` on fields that can't be empty, `double_option` on fields that can) live in `protocol.rs` and `model.rs`. `ship-server` gets one new module, `body.rs`, that every JSON body and query string goes through. It decodes with serde, and only when serde refuses does it ask the schema explainer, also in `body.rs`, for the full list. Zoom moves onto `SetTab`, the server's zoom message and route go away, and `ship-client` decides zoom in a new `client.pane.zoom` action.

Four slices, each a reviewable commit in one pull request: shapes, the error path, zoom, then docs and specs.

## Skeleton map

### Decisions made during program design

Cyan approved all three recommendations in chat ("i agree with oyur deffaults on those questions"). They are marked where they land below.

- **Q1. KD-10's premise is wrong.** KD-10 rejected "adding `serde_path_to_error` for this alone", but it is already a `ship-core` dependency (`config.rs:28` uses it for config errors). Using it in the decoder gives the fallback entry a pointer (for example `/pane/cwd`) whenever serde knows the path, at no new dependency. Decided: use it. `architecture.md` KD-10 is revised to match.
- **Q2. The input stream's lines.** `POST /attach/input` takes NDJSON lines (`InputFrame`), and a bad line stops the stream with 422 today. FR-015 says every JSON request body rejects unknown fields, and it doesn't say whether the lines count. Decided: they count. Lines reject unknown fields and decode through the same `decode` function, so a bad line gets the same `data.errors` list. A malformed line answers 400 like every other body, and the stream's other stops, such as mouse input without an attachment, keep 422.
- **Q3. Missing `Content-Type: application/json`.** axum answers 415 with plain text today. `ErrorCode` has no 415. Decided: make it 400 `validation` with one entry ("expected application/json") instead of adding an error code for it.

### Added

#### `crates/ship-server/src/body.rs` (slice 2)

The shared helper. Every body and query goes through it, and a decode failure always becomes a 400 `validation` `AppError` whose `data` is `{"errors": [{pointer, detail}]}`.

```rust
//! Request decoding: serde decides, the schema explains a refusal.

use std::sync::LazyLock;

use axum::{
    extract::{FromRequest, FromRequestParts, Request},
    http::request::Parts,
};
use jsonschema::{ValidatorMap, error::{ValidationError, ValidationErrorKind}};
use serde::de::DeserializeOwned;
use serde_json::Value;
use ship_core::{AppError, Result, error::Problem};
use utoipa::ToSchema;

/// A JSON body decoded by serde. On refusal, the problems the body's schema
/// finds, else serde's own message.
pub(crate) struct Body<T>(pub T);

impl<T, S> FromRequest<S> for Body<T>
where
    T: DeserializeOwned + ToSchema,
    S: Send + Sync;

/// A query string decoded by serde. Always one problem (FR-018).
pub(crate) struct Query<T>(pub T);

impl<T, S> FromRequestParts<S> for Query<T>
where
    T: DeserializeOwned,
    S: Send + Sync;

/// Decode one JSON document as `T`. Also used for input stream lines (Q2).
pub(crate) fn decode<T: DeserializeOwned + ToSchema>(bytes: &[u8]) -> Result<T>;

/// The 400 `validation` error carrying `problems`.
fn invalid(what: &'static str, problems: Vec<Problem>) -> AppError;

/// Every component schema in `crate::openapi()`, compiled once by
/// `jsonschema::options().build_map` from `{"components": <document's
/// components>}` on the first refused request (about 6 ms in the spike).
/// Looked up as `#/components/schemas/{T::name()}`, the document's own ref
/// form (KD-8).
static SCHEMAS: LazyLock<ValidatorMap>;

/// What `T`'s schema finds wrong with `body`, after the noise filters.
/// Empty when the schema and serde disagree.
fn explain(schema: &str, body: &Value) -> Vec<Problem>;

/// The spike's two filters: drop the `null` branch of an optional field,
/// and drop tagged-enum variants whose key wasn't sent, then merge
/// single-value enum and pattern branches.
fn flatten(error: &ValidationError, out: &mut Vec<Problem>);
```

Decode order in `Body`: check the content type (Q3), read the bytes, then call `serde_path_to_error::deserialize::<_, T>` over a `serde_json::Deserializer` on the bytes. On success, the schema is never touched. On failure, parse the bytes as a `Value`. If that also fails, the body isn't JSON, and the error carries serde's syntax message as its one entry. Otherwise, call `explain(T::name(), &value)` and fall back to serde's message when the list is empty, with `serde_path_to_error`'s path as its pointer when it has one (Q1).

### Moved

None.

### Replaced

#### `crates/ship-core/src/protocol.rs` (slice 1, except `SetTab::zoomed` in slice 3)

`PaneInput`, `PaneSpec`, `ZoomPane` and `set_name` are removed. `CreatePane` stops flattening. `MoveTab` keeps its name as the placement enum, and a new `MoveTabBody` wraps it.

```rust
use serde_with::rust::double_option;

pub const PROTOCOL_VERSION: u32 = 17;

/// POST /tabs body, e.g. `{"at": {"after": "tab:…"}, "starter": "shell", "tab": {"name": "x"}}`.
#[serde_with::skip_serializing_none]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CreateTab {
    #[serde(default = "MoveTab::top")]
    pub at: MoveTab,
    #[serde(default, deserialize_with = "present")]
    #[schema(nullable = false)]
    pub starter: Option<Starter>,
    #[serde(default, deserialize_with = "present")]
    #[schema(schema_with = attachment_schema, nullable = false)]
    pub select: Option<IdOf<Attachment>>,
    #[serde(default)]
    pub tab: NewTab,
}

/// The new tab, as a subset of `Tab`.
#[serde_with::skip_serializing_none]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct NewTab {
    #[serde(default)]
    pub name: Option<String>,
}

/// POST /panes body, e.g. `{"at": {"pane": "pane:…"}, "direction": "down", "pane": {"cwd": "/tmp"}}`.
#[serde_with::skip_serializing_none]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CreatePane {
    pub at: PaneAt,
    #[serde(default)]
    pub direction: SplitDirection,
    #[serde(default, deserialize_with = "present")]
    #[schema(schema_with = attachment_schema, nullable = false)]
    pub select: Option<IdOf<Attachment>>,
    #[serde(default)]
    pub pane: NewPane,
}

/// The new pane, as a subset of `Pane`. Absent settings take
/// `[server.defaults.pane]`. Also what a restored pane starts from.
#[serde_with::skip_serializing_none]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct NewPane {
    #[serde(default)]
    pub name: Option<String>,
    /// argv; absent means the configured shell, else the login shell.
    #[serde(default, deserialize_with = "present")]
    #[schema(nullable = false)]
    pub command: Option<Vec<String>>,
    /// Absolute directory; absent means the split pane's current directory,
    /// else its starting one, else the home directory (FR-017).
    #[serde(default, deserialize_with = "present")]
    #[schema(nullable = false)]
    pub cwd: Option<String>,
    // right_click_passthrough: Option<bool>, exit_behavior: Option<ExitBehavior>,
    // scrollback_bytes: Option<usize>, each with `present` and `nullable = false`.
}

/// POST /tabs/{id}/move body, e.g. `{"at": {"after": "tab:…"}}`.
#[serde_with::skip_serializing_none]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MoveTabBody {
    pub at: MoveTab,
}

/// PATCH /tabs/{id}, as JSON Merge Patch: absent leaves, `null` clears.
#[serde_with::skip_serializing_none]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SetTab {
    #[serde(default, with = "double_option")]
    pub name: Option<Option<String>>,
    /// A pane in this tab zooms it; `null` unzooms. (slice 3)
    #[serde(default, with = "double_option")]
    pub zoomed: Option<Option<IdOf<Pane>>>,
}

/// PATCH /panes/{id}, as JSON Merge Patch. Only `name` can be `null`.
#[serde_with::skip_serializing_none]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SetPane {
    #[serde(default, with = "double_option")]
    pub name: Option<Option<String>>,
    #[serde(default, deserialize_with = "present")]
    #[schema(nullable = false)]
    pub right_click_passthrough: Option<Switch>,
    // exit_behavior, scrollback_bytes: as above.
}

/// `double_option`'s sibling: serde calls it only when the key is there and
/// reads a plain `T`, so `null` is refused rather than read as absent.
/// Pair with `#[serde(default)]` and `#[schema(nullable = false)]`.
fn present<'de, D: Deserializer<'de>, T: Deserialize<'de>>(deserializer: D) -> Result<Option<T>, D::Error>;
```

Every other request type gains `deny_unknown_fields`, and the request's optional fields get `present` wherever an absent value means a default rather than "none".

| Type | `deny_unknown_fields` | `present` on |
| --- | --- | --- |
| `SwapPane`, `ViewInput` | yes | none |
| `ResizePane` | yes | `amount`, `to` |
| `ScrollPane` | yes | `amount` |
| `AttachRequest` | yes | none: `selection` and `area` mean "none" when absent, so `null` stays accepted, as in `SelectView` |
| `SelectView` | yes | none, since `null` selects nothing |
| `ReadPane` (query) | yes | not applicable |
| `KeyInput`, `PasteInput`, `MouseInput` | yes (Q2) | none |
| `Size` (`screen.rs`, inside `area`) | yes | none |

`IdOf<Pane>` takes its schema from a hand-written function, so `zoomed`'s nullable schema may not come out on its own (architecture risk 1). If the derived document shows `zoomed` as non-nullable, add `fn nullable_pane_schema() -> RefOr<Schema>` next to `attachment_schema` and point `schema_with` at it.

#### `crates/ship-core/src/model.rs` (slice 1)

`OptionalName` is removed (KD-7). The `name` fields on `Tab` and `Pane` become `#[serde(default)] pub name: Option<String>`. That still serializes as `null` when unset, so `get` output is unchanged, and saved files still read. `Switch` and `Toggle` stay.

#### `crates/ship-core/src/error.rs` (slice 2)

```rust
/// One thing wrong with a request, in a `validation` error's `data.errors`.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct Problem {
    /// JSON Pointer into the body, e.g. `/pane/exit_behavior`; absent when
    /// serde's error has no path, such as malformed JSON (KD-10, Q1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pointer: Option<String>,
    pub detail: String,
}
```

#### `crates/ship-core/src/command.rs` (slice 1, with `--zoomed` in slice 3)

```rust
// tab::Create, pane::Create: `name: Option<String>`; the doc says
// "omitted means none" (no blank rule).

pub struct tab::Set {
    pub tab: TabTarget,
    /// New name; a bare --name clears it
    #[arg(long, value_name = "NAME")]
    pub name: Option<Option<String>>,
    /// Zoom this pane of the tab to fill it; a bare --zoomed unzooms (slice 3)
    #[arg(long, value_name = "PANE")]
    pub zoomed: Option<Option<IdOf<Pane>>>,
}

pub struct pane::Set {
    pub pane: PaneTarget,
    /// New name; a bare --name clears it
    #[arg(long, value_name = "NAME")]
    pub name: Option<Option<String>>,
    // right_click_passthrough, exit_behavior, scrollback_bytes unchanged
}

// PaneCommand::Zoom and pane::Zoom are removed (slice 3).
```

In TOML, which has no `null`, `Option<Option<T>>` reads a present value as `Some(Some(v))`, so `server.tab.set` and `server.pane.set` can set but not clear, as the product paper's non-goal says.

#### `crates/ship-client/src/keymap.rs` (slice 3)

```rust
pub enum ClientPane {
    Next {},
    Prev {},
    Focus { direction: Direction },
    Swap { direction: Direction },
    /// Zoom the selected pane (`true`), unzoom its tab (`false`), or flip
    /// between the two (`"toggle"`), deciding from the rendered tree, through
    /// `PATCH /tabs/{id}`. Required, so a binding states which it does.
    Zoom { zoomed: Switch },  // ship_core::model::Switch
}
```

#### `crates/ship-client/src/ui/mod.rs` (slice 3)

```rust
// act(): the unzoom-before-moving item becomes
//   Command::Tab(TabCommand::Set(tab::Set { tab: TabTarget { pane: Some(zoomed), .. }, name: None, zoomed: Some(None) }))
// and the rule matches `leaves_zoom` instead of every `ClientAction::Pane`,
// so pressing alt-f on a zoomed pane doesn't unzoom and then zoom again,
// and a future `client.pane` action doesn't unzoom unless it's listed.

/// Actions that move off the selected pane, so a zoomed tab unzooms first
/// (FR-012): `client.pane.next`, `prev`, `focus` and `swap`.
fn leaves_zoom(action: &Action) -> bool;

/// The `tab::Set` for `client.pane.zoom`: `zoomed.apply(current)`, where
/// `current` is whether the selected pane's tab is zoomed on it; `true` sends
/// the pane's ID, `false` sends `null`. `None` when no pane is selected.
fn zoom(observer: &Observer, selection: Option<NodeId>, zoomed: Switch) -> Option<Command>;
```

#### Smaller replacements

| File | Change | Slice |
| --- | --- | --- |
| `crates/ship-core/src/protocol.rs` docs | `CreatePane`, `NewPane` and `ReadPane` doc comments state real defaults (FR-017), replacing the old `cwd` "server's home directory" text | 1 |
| `crates/ship-server/src/routes.rs` | Handlers take `Body<T>` and `Query<T>`; `create_*` pass the nested bodies; `move_tab` takes `Body<MoveTabBody>`, and its hand decode and doc comment go; `zoom_pane` goes (slice 3); each route lists `400 validation` with `body = AppError`, and the "422 malformed body" response lines go | 1, 2, 3 |
| `crates/ship-server/src/attach.rs`, `views.rs` | `Json<T>` inputs become `Body<T>` | 2 |
| `crates/ship-server/src/input.rs` | `frame()` calls `body::decode::<InputFrame>` (Q2) | 2 |
| `crates/ship-server/src/state.rs` | `start_pane(.., input: NewPane)`; `restore_tab` builds a `NewPane`; `CreateTab` reads `create.tab.name`; `Set<Tab>` applies `name` as `Option<Option<String>>` and `zoomed` in one commit, refusing an unknown pane with `not_found` and another tab's pane with `invalid_structure` (slice 3); `Set<Pane>` applies `name` the same way; the `Zoom` message and struct go (slice 3) | 1, 3 |
| `crates/ship-server/src/lib.rs` | Starter tab uses `NewTab::default()`; `OptionalName` and `PaneSpec` drop out of the imports and `ApiDoc`; `routes::zoom_pane` leaves `api_routes!` (slice 3); `mod body;` | 1, 2, 3 |
| `crates/ship-server/Cargo.toml` | Add `jsonschema = { version = "0.60", default-features = false }` through `[workspace.dependencies]` like the others, and `serde_path_to_error.workspace = true` (Q1) | 2 |
| `Cargo.toml` (workspace) | `jsonschema` entry in `[workspace.dependencies]` | 2 |
| `crates/ship-client/src/execute.rs` | Builds `CreateTab { tab: NewTab { name } }` and `CreatePane { pane: NewPane { .. } }`; `move_tab` sends `MoveTabBody`; `tab::Set` and `pane::Set` pass `name` (and `zoomed`) straight through; the `PaneCommand::Zoom` arm goes (slice 3) | 1, 3 |
| `crates/ship-client/src/api.rs` | `move_tab` takes `&MoveTabBody`; `zoom()` and the `ZoomPane` import go (slice 3) | 1, 3 |
| `crates/ship-client/src/ui/draw.rs` | `name.get()` becomes `name.as_deref()`; an empty name draws as an empty label | 1 |
| `crates/ship-core/src/defaults.toml` | `alt-f` and the pane menu's `zoom` item become `client.pane.zoom = { zoomed = "toggle" }` | 3 |

### Untouched

- `crates/ship-core/src/config.rs`: `PaneDefaults` already matches `Pane`'s keys apart from `shell` (KD-3).
- The saved tabs file format (`saver.rs`, `records.rs`). It serializes `Tab`, whose `name` still writes `null`.
- Swap, resize and scroll routes keep their verb shapes. Only their bodies gain `deny_unknown_fields`.
- Path parameters (`Path<IdOf<T>>`) and the attachment header keep axum's own rejections. FR-018 covers query strings, not paths.
- `set_server_log`'s 422 for a filter the runner refuses. Only its body decoding changes.
- Responses. `get` output keeps its shape.

### Proposed settings

- `jsonschema` 0.60 with `default-features = false`: no remote `$ref` resolution, which the document doesn't need. The spike built with this setting.
- Request structs carry `#[serde_with::skip_serializing_none]` above their derives, in place of `skip_serializing_if = "Option::is_none"` on each field. serde_with's `macros` feature, already on by default. Slice 1's document dump confirms utoipa still sees the rewritten attributes.
- No new feature flags. `jsonschema` is a `ship-server` dependency only. The one `ship` binary links it, because it runs the server.

## Build order

Each slice is a commit in one pull request (no tiny PRs). Checks use an owned temporary server on a port from 43331 to 43337, with `XDG_STATE_HOME` and `XDG_CONFIG_HOME` pointed at temporary directories, and leave the server on 43179 alone. Every slice ends with `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace`. Everything runs on macOS; Linux is reported as unverified unless checked there.

### 1. Shapes

**Files.** `protocol.rs`, `model.rs`, `command.rs` (names, not zoom), `state.rs`, `routes.rs` (nested bodies, `MoveTabBody`, still on `Json<T>`), `lib.rs`, `execute.rs`, `api.rs`, `draw.rs`.

**Delivers.** The new body shapes, Merge Patch names, names stored as sent, CLI tri-state `--name`, and protocol 17. Errors are still axum's plain text until slice 2.

**Done when:**
- `POST /panes {"at": {"tab": T}, "pane": {"cwd": "/tmp", "exit_behavior": "keep"}}` creates a pane with those values, and `POST /panes {"at": {"tab": T}, "cwd": "/tmp"}` is refused, creating nothing.
- `POST /tabs {"tab": {"name": "x"}}` and `POST /tabs/{id}/move {"at": {"after": T}}` work.
- `PATCH /panes/{id} {"name": null}` clears the name, `{"name": "  x "}` stores `"  x "`, `{"right_click_passthrough": null}` is refused, and `{"right_click_passthrough": "toggle"}` flips it.
- `ship pane set --name` clears the name, `--name ""` stores `""`, and the same holds for `ship tab set`.
- `ship server status` prints `protocol_version: 17`.

### 2. The error path

**Files.** `body.rs` (new), `error.rs` (`Problem`), `routes.rs`, `attach.rs`, `views.rs`, `input.rs`, `lib.rs`, both `Cargo.toml` files.

**Delivers.** Every body and query failure becomes 400 `validation` with `data.errors`, and only refused requests touch the schema.

**Done when:**
- `POST /panes {"direction": "up", "pane": {"exit_behavior": "linger", "scrollback_bytes": "lots", "exitBehavior": "keep"}}` answers 400 with one entry each for the missing `at`, the bad `direction`, `linger`, `lots` and `exitBehavior`, each with a pointer (SC-003).
- `GET /panes/{id}/text?line=50` answers 400 `validation`, and its one entry names `line`.
- Every route with a body or query, sent an unknown field or parameter, answers 400 and leaves the revision header unchanged (SC-002).
- Every valid request from slice 1's list still succeeds (SC-006).
- A body that isn't JSON gets one entry with serde's syntax message.
- A body that serde refuses but the schema accepts gets one entry whose pointer comes from `serde_path_to_error` (Q1).
- A body sent without `Content-Type: application/json` answers 400 `validation` with one entry, not 415 (Q3).
- On `POST /attach/input`, a line with an unknown field or a bad value ends the stream with 400 and its `data.errors`, and mouse input without an attachment still answers 422 (Q2).
- Every field that can't be cleared, sent `null` in each body that has it, answers 400 with that field's pointer and changes nothing. This catches a missing `present`, which the document can't show.
- The release binary size and build time are recorded before and after adding `jsonschema` (architecture risk 4).

### 3. Zoom

**Files.** `protocol.rs` (`SetTab::zoomed`, schema function if needed), `command.rs` (`--zoomed`, removing `pane::Zoom`), `state.rs`, `routes.rs`, `lib.rs`, `execute.rs`, `api.rs`, `keymap.rs`, `ui/mod.rs`, `defaults.toml`.

**Delivers.** Zoom as a tab field everywhere, and `client.pane.zoom`.

**Done when:**
- `PATCH /tabs/{id} {"zoomed": P}` zooms P, `{"zoomed": null}` unzooms, a pane in another tab gets 422 `invalid_structure`, and an unknown pane gets 404 `not_found`.
- `ship tab set --tab T --zoomed P` zooms, and a bare `--zoomed` unzooms.
- `alt-f` and the menu's `zoom` zoom and unzoom the selected pane in a real client, and moving focus out of a zoomed pane unzooms first.
- With two clients attached, both pressing `alt-f` on an unzoomed tab leaves it zoomed.
- A temporary binding with `zoomed = true` only zooms and one with `zoomed = false` only unzooms, however often each is pressed, and `client.pane.zoom = {}` fails `ship config check` for the missing argument.
- `POST /panes/{id}/zoom` answers 404 or 405, `ship pane zoom` is an unknown subcommand, and `server.pane.zoom` in a config fails `ship config check`.
- The derived document shows `zoomed` as nullable.

### 4. Docs, specs and the one-off checks

**Files.** `docs/agents/code.md` (new "Entity and request shapes" subsection, including "an input field takes `null` only if the entity field can be `null`", with `present` and `double_option` as the two ways to say it), `docs/site/panes.md`, `docs/site/tabs.md` (bare `--name` clears, not `""`), `docs/site/configuration.md` (the `server.pane.zoom` menu example), and spec deltas under `openspec/changes/entity-shapes/specs/` for keymap (FR-013's rule, `client.pane.zoom`, and `alt-f`), context-menus (the zoom item's action), session-structure (names, PATCH bodies, create bodies and zoom), and any other spec that names the old shapes. A grep finds them.

**Delivers.** FR-020 and P6, plus the SC-001 and SC-005 checks.

**Done when:**
- A scratch binary outside the repo, like the spike, loads `ship_server::openapi()`, prints every input field whose name matches an entity field but whose type differs, allowing only `toggle` on PATCH bools and `shell`, and every input field that is nullable when its entity field isn't, or the reverse, and prints nothing. It is then deleted (KD-4).
- Every `server.` action in `defaults.toml` is checked by hand against FR-013, and the result is listed in the pull request (SC-005).
- `grep -rnE 'server\.pane\.zoom|ship pane zoom|/zoom|ZoomPane|OptionalName|PaneInput|PaneSpec'` over the repo, excluding `target/`, `.git/` and `openspec/changes/archive/`, matches only this change's own papers and the main specs its deltas replace.
- `openspec validate entity-shapes --type change --strict` passes.

## Deviation log

- The archive also modifies session-structure's JSON names requirement, which 4.4 missed: it still said Ship rejects camel case keys. Following the slice 2 decision to ignore unknown fields, a camel case key is now an unknown field and is ignored, and the PATCH scenario expects the pane and revision unchanged.

- Slice 2 replaces jsonschema 0.60.1 with boon 0.6.1 at Cyan's direction above, after the scratch compatibility, release-size and build-time comparison. The server-local `ValidatorMap` holds boon's `Schemas` and component-name indices; `flatten` follows structured causes. The request path and wire contract are unchanged.

- Slice 2 switches release builds from `panic = "abort"` to unwind with Cyan's approval: a panic in a request or the saver no longer takes every pane with it. Actor panics still shut the server down through `actor_stopped`. The saver now restarts on a panic through `backon`, with exponential backoff from 1 to 60 seconds and a warning each time; a restart writes the whole tree and keeps the run's first `.json.prev` backup. The router compiles the explanation schemas when it is built, so a broken schema fails at startup rather than on the first refused request. The panic hook's message reads "thread panicked", since the process now survives.

- Slice 2 gives the router JSON fallbacks with Cyan's approval: an unknown path answers 404 and a wrong method 405 with an `AppError` (`not_found`) instead of axum's empty bodies. axum still sends `Allow` on the 405.

- Slice 2 adds a `Path<T>` extractor to `body.rs` with Cyan's approval, and every route takes path IDs through it. A malformed ID now gets 400 `validation` with one `data.errors` entry instead of axum's plain text, so each route's single 400 response truthfully carries `AppError`. This changes the GET and DELETE routes too.

- Slice 2 puts `Problem` in `crates/ship-core/src/protocol.rs` instead of `error.rs`. It is a wire shape, and nothing in `error.rs` uses it: `AppError::data` stays an untyped `Value` so error codes need no shared payload enum. `ApiDoc` registers it as a component, since no route reaches it otherwise.

- Slice 2 removes request `deny_unknown_fields` from `protocol.rs` and `Size` with Cyan's approval above. It supersedes the unknown-field checks from slice 1 and the original slice 2 plan. API schemas and serde tolerate extras; config stays strict. No input-frame special case or schema copy is needed.

- Slice 1 also updates `crates/ship-client/src/ui/mod.rs`: sidebar drag moves call `Client::move_tab` directly and need the new `MoveTabBody` wrapper.
- Slice 1 adds `deny_unknown_fields` to `Size` in `crates/ship-core/src/screen.rs`, since it is nested in attachment request bodies.
