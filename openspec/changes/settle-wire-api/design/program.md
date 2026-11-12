# Settle the wire API: program

Status: Locked on 2026-11-11 (Plannotator, "LGTM", with the four slices merged into two as asked in review), amended through A-1.

## Summary

```rust
// ship-client/src/api.rs: one read path, two ends
pub async fn attach_lines(&self, request: &AttachRequest) -> Result<impl Stream<Item = Result<String>> + use<>>;
pub async fn attach(&self, request: &AttachRequest)       -> Result<impl Stream<Item = Result<SseEvent>> + use<>>; // lines + decode, unknown types skipped

// ship/src/commands.rs: watch is a pipe
while let Some(line) = lines.next().await { writeln!(out, "{}", line?)?; }
```

There are two slices. The first settles the shapes: renames and the protocol bump~~, and strict bodies~~ (A-1). The second makes the client tolerant: one decoder that skips unknown types, watch as a pipe, and a `/health` check on reattach. This paper is a document-only plan: nothing below exists as source yet, and none of it has been compiled.

## Rationale

Architecture shape A (`architecture.md`, Decision) maps onto the code like this:

- **`Client::attach_lines`** is today's `attach` (`crates/ship-client/src/api.rs:262-292`) with the JSON decode taken out. It owns SSE framing and transport errors, and nothing else.
- **`Client::attach`** keeps its signature, so the UI (`ui/connection.rs`) and automation (`automation.rs:291-330`) callers don't change. It maps each line through one `decode` function, which skips a line only when its `type` names no `SseEvent` variant.
- **`SseKind`** is derived from `SseEvent` with strum's `EnumDiscriminants`, so the list of known names can't drift from the enum. strum is already used in ship-core for `IntoStaticStr` (`model.rs:131`).
- **watch** reads `attach_lines` through a new `Client::observe_lines`, the raw sibling of `observe` (`automation.rs:221`). Both build the same observer request through one private function.
- ~~**Strict bodies** use `#[serde(deny_unknown_fields)]`, the attribute `RunCommand` already carries (`protocol.rs:84`). `body::decode` doesn't change.~~ (A-1) Request bodies stay lenient.
- **Reattach** checks `/health` inside the reattach future. On a mismatch, the client exits through the error path main already uses for this, `err!(Validation, "{}", updates::mismatch(..))` (`crates/ship/src/main.rs:106`).

Patterns reused: `err!` and `.context` for errors, `serde_with` attributes on model fields (`tag.rs:251` uses `serde_as` the same way), `#[serde(default, ...)]` field style, and `Link` variants for anything the connection reports to the loop.

## Skeleton map

### Added

None. Every change lands in an existing file.

### Changed

**`crates/ship-core/src/model.rs`** (slices 1 and 2)

```diff
 pub struct Pane {
     …
-    pub status: PaneStatus,
+    /// `alias`: saved tabs written before 0.1.0 say `status` (D-2).
+    #[serde(alias = "status")]
+    pub process: ProcessState,
     /// OSC 7501 records keyed by id, oldest update first; ROOT is the root.
-    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
+    /// A record this client can't read is left out, as if never reported (FR-004).
+    #[serde_as(as = "MapSkipError<_, _>")]
+    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
     pub program: IndexMap<String, StatusRecord>,
 }

-pub enum PaneStatus {
+pub enum ProcessState {      // D-1; shape and `tag = "state"` unchanged
     Running,
     Exited(ExitStatus),
 }
```

`Pane` gains `#[serde_with::serde_as]` above its derives. `ReportedState` is unchanged (D-3).

**`crates/ship-core/src/protocol.rs`** (slices 1 and 2)

```diff
-pub const PROTOCOL_VERSION: u32 = 23;
+pub const PROTOCOL_VERSION: u32 = 24;

-/// `POST /api/v0/runs`: …      (and the other path in a doc comment)
+/// `POST /api/runs`: …

// (A-1) dropped: #[serde(deny_unknown_fields)] on the 19 request-only structs

+#[derive(strum::EnumDiscriminants)]
+#[strum_discriminants(name(SseKind), derive(strum::EnumString), strum(serialize_all = "snake_case"))]
 #[serde(tag = "type", content = "data", rename_all = "snake_case")]
 pub enum SseEvent { … }

 pub enum EndReason {
     ServerShutdown,
     Lost,
+    /// A reason from a newer server; clients reconnect (FR-003).
+    #[serde(other, skip_serializing)]
+    Unknown,
 }
```

~~`LogChange`, `PaneAt`, `Place` and `MoveTab` are externally tagged enums and already reject extra keys. `InputFrame` is internally tagged, so its strictness comes from the three input structs.~~ (A-1)

~~**`crates/ship-core/src/screen.rs`** (slice 1): a comment on `Size` saying why it stays lenient.~~ (A-1) Untouched: with every body lenient, `Size` is no exception.

**`crates/ship-client/src/api.rs`** (slices 1 and 2)

```diff
-const COLLECTION: &'static str = "/api/v0/tabs";   // and the other 10 paths
+const COLLECTION: &'static str = "/api/tabs";

+/// The attach stream's data lines as sent; SSE framing removed.
+pub async fn attach_lines(&self, request: &AttachRequest)
+    -> Result<impl Stream<Item = Result<String>> + use<>>;

 /// Open the attach stream. … Events whose type this client doesn't know are skipped.
 pub async fn attach(&self, request: &AttachRequest)
     -> Result<impl Stream<Item = Result<SseEvent>> + use<>>;   // attach_lines + try_filter_map(decode)

+/// One event, or `None` when its `type` names no `SseKind`. A known type that
+/// fails to decode stays an error, which sends the client to its /health check.
+fn decode(data: &str) -> Result<Option<SseEvent>>;
```

`decode` reads `type` from a failed line through a small named shape (`struct Kind<'a> { #[serde(rename = "type")] kind: &'a str }`), then `SseKind::from_str`.

**`crates/ship-client/src/automation.rs`** (slices 1 and 2)

```diff
+/// The attach request of an observer: no selection or area.
+fn observer(feed: FeedMode) -> AttachRequest;

 pub async fn observe(&self, feed: FeedMode) -> Result<impl Stream<Item = Result<SseEvent>> + use<>>;
+/// `observe`, undecoded, for `ship watch`.
+pub async fn observe_lines(&self, feed: FeedMode) -> Result<impl Stream<Item = Result<String>> + use<>>;

 // next(): unchanged arms, plus
+Some(Ok(SseEvent::Ended(Ended { reason: EndReason::Unknown }))) => // same error as Lost
```

`PaneStatus` → `ProcessState` and `.status` → `.process` at lines 22, 374 and 407.

**`crates/ship-client/src/ui/connection.rs`** (slice 2)

```diff
 pub(super) struct Connection<'a> {
     …
-    reconnect: Option<Request<'a, Events>>,
+    reconnect: Option<Request<'a, Reattached>>,
     backoff: Duration,
+    /// Attached, but no event since. The backoff resets on the first one, so a
+    /// server whose events don't decode can't pin reconnects at the shortest delay.
+    unproven: bool,
 }

 pub(super) enum Link {
     Event(SseEvent),
     Shutdown,
     Lost,
+    /// /health before a reattach reported another protocol.
+    Incompatible(HealthResponse),
     PutFailed(AppError),
 }

+/// What a reattach found.
+enum Reattached {
+    Events(Events),
+    Incompatible(HealthResponse),
+}

-/// Attach after `delay`.
-async fn attach_after(client: &Client, request: AttachRequest, delay: Duration) -> Result<Events>;
+/// After `delay`, check /health, then attach if the protocol matches.
+async fn reattach_after(client: &Client, request: AttachRequest, delay: Duration) -> Result<Reattached>;
```

`Connection::open` calls `client.attach` directly. Its protocol is already checked at startup (`ui/mod.rs:173`, `main.rs:106`). In `read`, `EndReason::Unknown` already falls into the `lost` arm (`connection.rs:142-151`).

**`crates/ship-client/src/ui/mod.rs`** (slice 2)

```diff
 fn take(link: Link, …) -> Result<Option<Effect>> {
+    Link::Incompatible(server) => return Err(err!(Validation, "{}", updates::mismatch(&server))),
```

**`crates/ship/src/commands.rs`** (slice 2)

```diff
-/// … After `ended`, or a cut stream, it fails (FR-004) …
+/// `ship watch`: the health answer, then every attach line as the server sent
+/// it. The server closes the stream after `ended`, which says why; watch then
+/// fails. A closed stdout is a normal end.
 pub async fn watch(client: &Client, health: &HealthResponse, feed: FeedMode) -> Result<()>;
 // body: observe_lines; Some(Ok(line)) → print; Some(Err) → context as today;
 // None → err!(Unavailable, "the server ended the stream")
```

**Renames only** (slice 1): `/api/v0` → `/api` in `crates/ship-server/src/{attach,views,routes,input,app}.rs`. `PaneStatus` → `ProcessState` and `.status` → `.process` in `crates/ship-client/src/ui/{draw,updates/mod}.rs`, `crates/ship-server/src/{pane,saver,state,state/geometry}.rs`.

**Docs and examples** (slice 1)

- `docs/site/scripting/README.md:11`: `jq .status` → `jq .process`.
- `docs/site/scripting/writing-a-plugin.md:88`: the promise and the key-order bullet, exactly as in product.md, Interface.
- `GLOSSARY.md`: **Process state** after **Status record**: "Whether a pane's program is running, or its exit code and signal once it exited. _Avoid_: Status (alone)".
- `examples/plugins/git.sh:17`: `protocol=24`.

### Moved

None.

### Replaced

- `attach_after` (`ui/connection.rs:185-189`) → `reattach_after`, as above.
- The JSON decode inside `Client::attach` (`api.rs:276-282`) → `decode`.

### Untouched

- ~~`body::decode` (`crates/ship-server/src/body.rs:76-99`). Strictness lives on the types.~~ (A-1) `crates/ship-server/src/body.rs`, and every request body type.
- `ReportedState`, `BlockedKind`, `Layout`, `Attr`, `CursorShape`: no new variants.
- `ship pane wait --status` and `send --if`: `WaitState` and its `From<ReportedState>` stay as they are (D-3).
- Server-side status records (`records.rs`). The server never produces a record it can't read.
- `docs/research/`: history, which keeps its `/api/v0` paths.
- `openspec/specs/`: the spec text changes (FR-009, FR-012's `ended` example, `/api/v0` paths, `status` → `process` in pane-programs and session-restore) go into this change's delta specs when spec artifacts are requested, and land at sync. They aren't in a slice.

## Decisions to confirm

- **D-1: rename the type `PaneStatus` → `ProcessState`.** The field becomes `process`, and the glossary term is Process state. The type name also shows up as the OpenAPI component name. It touches about 20 lines, all mechanical. Recommend yes, so the code, the schema and the glossary use one name.
- **D-2: `#[serde(alias = "status")]` on `process`.** Without it, every saved tabs file written before this release fails to parse (`process` is missing), and the server starts empty after the upgrade. The saver ignores the value anyway (`saver.rs:91`). It's one line of compatibility code, the cheap 80%. Recommend yes; it can go once nobody has an old file.
- **D-3: FR-004 through `MapSkipError` on `program`, not `ReportedState::Unknown`.** This changes the locked architecture (Structure; Key decision 4). Every client read of a record goes through `Pane::root_status` (`automation.rs:185,358,375,414`, `agents.rs:66,100`, `attention.rs:214`). An `Unknown` variant would need an arm at each exhaustive match, and `WaitState::from` (`command.rs:586`) would have to become fallible. On top of that, `NotWorking` would match an unknown state (`automation.rs:419`), which contradicts "like no record". Skipping the record when decoding is exactly "like no record", costs one attribute, and serde_with 3.24 (already a dependency) supports it for `IndexMap`. As a side effect, a record with an unknown `kind` is also dropped rather than failing the whole event. Recommend yes, as amendment A-1 to the architecture paper.
- ~~**D-4: verify strict bodies with a temporary probe, not a permanent test.** The architecture paper says "A test sends an unknown field to every route", but Ship has no authored tests, and `docs/agents/code.md` says to verify with probes and propose a permanent test only after something turns out flaky. Recommend a throwaway script (curl over the socket, one unknown field per route) run in slice 1, with its output kept as evidence. That makes the same A-1 amendment cover decision 3's wording.~~ (A-1) Moot: no strict bodies.
- **D-5: `#[serde(other, skip_serializing)]` on `EndReason::Unknown`.** utoipa-gen 6.0.1 drops `skip_serializing` variants from the schema (`component/serde.rs:49-50`), so the published schema doesn't list `unknown`. The cost: serializing `Unknown` is an error, and only a client ever holds it. Recommend yes.

## Build order

1. **Settle the shapes: renames and protocol bump.** ~~strict bodies~~ (A-1)
   Files: `ship-core` `model.rs` (`process`, `ProcessState`, alias), `protocol.rs` (`PROTOCOL_VERSION`, doc paths); ~~19 `deny_unknown_fields`; `screen.rs` (the `Size` comment)~~ (A-1); all `/api/v0` and `PaneStatus` sites listed above; docs and examples.
   Delivers: `ship pane get` prints `process` and no `status`; every route answers under `/api/`; `ship --version` says protocol 24; a saved tabs file from main still restores ~~; an unknown field in any body gets a 400 `validation` with its pointer and changes nothing~~ (A-1) (FR-007 to FR-011).
   Needs: nothing.
   Checks: `cargo build`, `cargo clippy --all-targets`, `cargo fmt --check`; `rg -n 'api/v0' crates docs/site examples` is empty (SC-006, apart from specs); `ship pane get … | jq -c '{status, process}'` shows `{"status":null,"process":{"state":"running"}}`; `ship pane wait --status exited` still ends on exit (SC-005); restore a saved file written by the main-branch build; ~~the D-4 probe: `PATCH /api/panes/$P` with `{"nmae":"x"}` gets a 400 with pointer `/nmae` and `ship pane get` shows the pane unchanged (SC-004), plus one request per remaining body type;~~ (A-1) a normal session in the full client (attach, split, type, resize, select), since the client sends most of these bodies.

2. **Tolerant client: skipping decoder, watch pipe, health on reattach.**
   Files: `protocol.rs` (`SseKind`, `EndReason::Unknown`), `model.rs` (`MapSkipError`), `api.rs`, `automation.rs`, `ui/connection.rs`, `ui/mod.rs`, `ship/src/commands.rs`.
   Delivers: FR-001 to FR-005. Unknown event types are skipped, unknown end reasons reconnect, unknown records are left out, watch prints lines as sent, and a protocol mismatch on reattach exits with the startup mismatch text; otherwise reconnects back off.
   Needs: slice 1.
   Checks: build, clippy and fmt; a temporary fake server (a small script on a Unix socket answering `/health` and `/api/attach` with a canned SSE body: `attached`, an unknown `bell` event, `state` with a root record in state `thinking`, `ended` with reason `rebooting`). Against it, `ship watch` output equals the canned data lines byte for byte (SC-002), and the full client stays up, draws the pane like one without a report, and logs exactly one reconnect (SC-001). The same fake server sending an undecodable known event shows the reconnect delays growing in the client log. Then attach the full client to a real server, stop it, and start one built with `PROTOCOL_VERSION = 25` (a temporary local edit, not committed) under the same name: the client exits with the mismatch message after one reconnect, and the server log shows no repeated attaches (SC-003).

## Deviation log

- ~~**2026-11-11, while writing the delta specs (slice 1).** For an unknown field, schema validation reports the error on the parent object, so `explain` would answer pointer `""` rather than `/nmae` (FR-006). `flatten` in `crates/ship-server/src/body.rs:209` gets an `ErrorKind::AdditionalProperties` arm that gives each field its own pointer, mirroring its `Required` arm. `body::decode` itself is unchanged. This fills a gap with an existing pattern and changes no decision.~~ (A-1) Moot: unknown fields are ignored, so `body.rs` is untouched.
- **2026-11-11, slice 2: the backoff grows after an unproven attach.** Resetting on the first event after `Attached` stops a reset at 250 ms, but nothing doubled the delay when a reattach succeeded and its events then failed to decode, so it would have stayed flat. `Connection::cut` now doubles the backoff (capped at `MAX_BACKOFF`) when the connection was still unproven, the same doubling a failed reattach gets. Three lines; it delivers architecture decision 2's "doesn't pin reconnects at 250 ms". Also, `observer`'s doc comment drops a stray `(FR-003)` tag.

## Amendments

### A-1 (2026-11-11): request bodies stay lenient

Follows product A-1 and architecture A-2: no `deny_unknown_fields` on request types, no `Size` comment, no `flatten` arm in `body.rs`, and D-4's probe is moot. Slice 1 shrinks to the renames and the protocol bump. Approved in chat: "fair enough". Touches Summary, Rationale, the `protocol.rs` and `screen.rs` entries, Untouched, D-4, slice 1, the deviation log.
