# Settle the wire API: architecture

Status: Locked on 2026-11-11 (Plannotator, "LGTM"), amended through A-1.

## Summary

```text
SSE body ──▶ attach_lines ──▶ ship watch          prints each line as sent; stops when the stream ends
                  │
                  ▼
             decode, skipping types it doesn't know
                  ├──▶ full client (ui::Connection)  checks /health before every reattach
                  └──▶ pane wait, send --wait
```

There's one read path. `ship watch` is a dumb pipe on its raw end and never parses events. Every other consumer gets typed events from a decoder that skips unknown types. Request bodies become strict per type with `deny_unknown_fields`, which `body::decode` already honors. The rest is mechanical: `status` → `process`, and `/api/v0` → `/api`.

## Context and constraints

- **One decode site.** `Client::attach` turns the SSE body into `SseEvent`s: `SseByteStream` → `block.data` → `serde_json::from_str`, and any failure becomes "cannot decode attach event" (`crates/ship-client/src/api.rs:262-292`). The full client, `ship watch` and the automation commands all read through it (`automation.rs:221-232` wraps it as `observe`).
- **`SseEvent` is adjacently tagged** (`crates/ship-core/src/protocol.rs:445-449`), so `type` comes before `data` and screen cells decode without buffering. `#[serde(other)]` doesn't work on adjacently tagged enums whose unknown variant carries `data`. The oracle's probe showed it fails with "invalid type: map, expected unit variant". It does work on plain unit enums like `EndReason` (`protocol.rs:485-492`) and `ReportedState` (`model.rs:135-141`).
- **The server closes the stream right after `ended`.** The unfold returns `None` after the `Ended` event (`crates/ship-server/src/attach.rs:114-119`), so a reader sees `ended` and then EOF.
- **watch's exit doesn't depend on the reason.** `ServerShutdown` and `Lost` both exit `Unavailable` and differ only in the stderr text (`crates/ship/src/commands.rs:73-81`). The `ended` line has already been printed by then (`commands.rs:70`).
- **The full client's reconnect.** `Connection::read` maps `ServerShutdown` to `Link::Shutdown`, any other event to `Link::Event`, and errors and every other `ended` to `Link::Lost` (`crates/ship-client/src/ui/connection.rs:123-152`). The backoff resets on `Attached` (`connection.rs:138`) and doubles on a failed reattach (`connection.rs:101-102`). `/health` is checked only at startup (`ui/mod.rs:173`). The mismatch text already exists as `updates::probe::mismatch` (`crates/ship-client/src/updates/probe.rs:102`).
- **Body decoding is generic.** `body::decode` runs `serde_path_to_error` and, on refusal, explains via the schema (`crates/ship-server/src/body.rs:76-99`). It accepts whatever serde accepts.
- **Only `RunCommand` among the 16 body types is strict** (`protocol.rs:84`). The other 15 (`AttachRequest`, `ViewInput`, `InputBody`, `SelectView`, `LogChange`, `CreateTab`, `SetTab`, `MoveTabBody`, `CreatePane`, `CyclePane`, `MovePane`, `SetPane`, `SwapPane`, `ResizePane`, `ScrollPane`) aren't. The other 38 `deny_unknown_fields` in the codebase are on config types (`keymap.rs`, `command.rs`, `config.rs`).
- **Nested body types.** `PaneAt`, `Place` and `MoveTab` are externally tagged enums and already reject extra keys. `TagPatch` is a map whose keys are tag names. `NewTab`, `NewPane` and `InputFrame` are request-only and lenient today. `Size` (`crates/ship-core/src/screen.rs:219`) is also inside screens the server sends (`screen.rs:15`).
- **No `#[serde(flatten)]`** in the body types, so `deny_unknown_fields` applies without its known conflict.
- **`ReportedState` matches.** `WaitState::from` (`crates/ship-core/src/command.rs:586-594`), `agents.rs:67-71`, `attention.rs:195-196` and the comparisons in `automation.rs:194,360,419` all match on it. The server only produces it from OSC 7501 (`crates/ship-server/src/records.rs:14-26`).
- **Paths.** Routes register through `api_routes!` (`crates/ship-server/src/lib.rs:77-86`) from `#[utoipa::path]` attributes. "api/v0" appears in `protocol.rs` (2), `attach.rs` (2), `routes.rs` (20), `views.rs` (3), `input.rs` (1), `app.rs` (1, a comment) and `crates/ship-client/src/api.rs` (11).
- **strum is already a dependency** of ship-core and ship-client (`Cargo.toml:49`).
- **Budget.** Code that exists only for compatibility (FR-001 to FR-005) stays at tens of lines (SC-007).

## Candidate shapes

Only decision 1, unknown event types, had more than one plausible shape. Decisions 2 and 3 are under Key decisions.

### A: raw lines underneath, a skipping decoder on top (chosen)

`Client` exposes the SSE data strings, and `attach` decodes on top of them. On a decode error, the decoder reads just `type`. If the name isn't an `SseEvent` variant, it skips the event; otherwise it reports the error as it does today. `ship watch` reads the raw strings, prints them and stops at EOF. It costs about 15 lines in the decoder plus a few in watch. If a known event fails to decode, that's still an error, which feeds FR-005.

```rust
// ship-client
fn attach_lines(..) -> impl Stream<Item = Result<String>>      // block.data, untouched
fn attach(..)       -> impl Stream<Item = Result<SseEvent>>    // attach_lines + decode, skip unknown

// ship-core: the variant names, derived next to SseEvent
#[derive(EnumDiscriminants)]
#[strum_discriminants(name(SseKind), derive(EnumString), strum(serialize_all = "snake_case"))]
pub enum SseEvent { … }
```

### B: an `Unknown` variant through a custom `Deserialize`

Hand-write `Deserialize` for `SseEvent` so an unknown `type` becomes `SseEvent::Unknown`. Every match on `SseEvent` (`connection.rs`, `ui/mod.rs`, `automation.rs`, `commands.rs`) gains an arm. The hand-written impl replaces the derive and has to keep its no-buffering property for screens. It's more code than A, and watch still re-encodes, so `Unknown` would have to keep its raw JSON to print it as sent.

### C: watch parses an envelope

watch decodes each line as `{type: &str, data: &RawValue}`, prints the original and decodes `data` only for `ended`. It works, but it's a second way of reading the stream that exists for one reason: telling `ended` apart. The server already says that by closing the stream. Rejected in chat ("i still don't know how much i like that?").

## Decision

A. A dumb pipe for watch and one decoder for everyone else. It's the least code, and every consumer reads the stream the same way. Settled in chat: "if you wanna know why the server ended the stream, it told you. the watch client remains a dumb pipe".

## Structure

```text
ship-core
  SseEvent ── SseKind (derived variant names)
  EndReason { ServerShutdown, Lost, Unknown }        unknown → reconnect
  ~~ReportedState { …, Unknown }                     unknown → drawn like no record~~ (A-1)
  Pane.program: a record that doesn't decode is left out → drawn like no record
  request bodies: deny_unknown_fields                Size stays lenient, with a comment
  Pane.process (was status)

ship-client
  Client::attach_lines ──▶ Client::attach (skip unknown types)
  ui::Connection: /health before each reattach → Link::Mismatch; backoff resets on the first event after Attached

ship
  watch: attach_lines → stdout; EOF or error → Unavailable "the server ended the stream"
  pane wait, send --wait: unchanged, through attach

ship-server
  body::decode unchanged; routes under /api
```

- **`attach_lines`** owns the SSE framing and transport errors. It doesn't know what an event is.
- **`attach`** owns the meaning of events and the one compatibility rule for them: an unknown type is skipped.
- **The body types** own their own strictness. The decoder stays generic.

## Key decisions

1. **Unknown event types.** Shape A. Rejected: B (a hand-written `Deserialize` plus an arm at every match) and C (a second parse just for `ended`).
2. **`/health` on every reattach, not only after a decode error.** A reattach is rare and the check is one small request, and it also catches a server swapped between two reconnects. A differing protocol ends the client with `updates::probe::mismatch`. The backoff now resets on the first event after `Attached`, so a server that attaches and then sends something undecodable doesn't pin reconnects at 250 ms. Rejected: checking only after a decode error, which ties the check to one kind of failure for no saving. Approved in chat: "i agree with 2".
3. **Strict bodies through `#[serde(deny_unknown_fields)]` on each request-only type**: the 15 lenient bodies plus `NewTab`, `NewPane` and `InputFrame`. utoipa then writes `additionalProperties: false` for them, so the schema matches the server. `Size` stays lenient because the client also reads it from screens: making it strict would turn any new `Size` field into a protocol bump, and a typo inside it already fails as a missing required field. ~~A test sends an unknown field to every route.~~ (A-1) A temporary probe sends an unknown field to every route; no permanent test. Rejected: `serde_ignored` in `body::decode`, which puts the policy in the decoder and leaves the schema looser than the server; and flipping `additionalProperties` in the generated schema, which would also close response schemas through shared components and misbehave under `allOf`. Settled in chat: "A is better just because it's more consistent", and for `Size`, "just leave it lenient then with a comment explaining why".
4. ~~**Unknown `EndReason` and `ReportedState` values** become an `Unknown` variant via `#[serde(other)]`, which works on these plain enums. `Connection::read` already sends every non-shutdown `ended` to reconnect. `ReportedState::Unknown` is handled at the matches listed in Context.~~ (A-1) **An unknown `EndReason`** becomes `EndReason::Unknown` via `#[serde(other)]`; `Connection::read` already sends every non-shutdown `ended` to reconnect. **An unknown status record** is skipped when `Pane.program` is decoded (`serde_with::MapSkipError`), so every client read through `Pane::root_status` sees no record. Rejected: `ReportedState::Unknown`, which needs an arm at every match, a fallible `WaitState::from`, and would let the default `pane wait` (`automation.rs:419`) end on an unknown state.
5. **No-choice renames.** `Pane.status` → `Pane.process`, with the type and shape unchanged. `/api/v0/…` → `/api/…` everywhere listed in Context. `PROTOCOL_VERSION` 23 → 24.

## Risks and unknowns

- ~~**`WaitState::from` is infallible** (`command.rs:586`). An `Unknown` reported state has no `WaitState`, so that conversion has to become fallible or skip. That gets settled in program.md and doesn't change the shape.~~ (A-1) Gone: no `Unknown` state reaches `WaitState`.
- **A field added to a type the client reads** (`Size`, `Replica`, records) is ignored by serde's default. That's the intended behavior, but only as long as nobody adds `deny_unknown_fields` to a response type. The `Size` comment carries the rule.
- **Anything that changes how the client draws** (`Layout`, `Attr`, `CursorShape`, `BlockedKind`, the process state) still fails to decode on an unknown value. It goes through FR-005 and needs a protocol bump, as the product paper intends.

## Amendments

### A-1 (2026-11-11): skip unreadable records; probe instead of a test

An unknown reported state now drops its record at decode (`MapSkipError` on `Pane.program`) instead of adding `ReportedState::Unknown`. That's one attribute rather than an arm at every match, and the default `pane wait` can't end on an unknown state. Strict bodies are checked with a temporary probe rather than a permanent test, following `docs/agents/code.md`. Approved with program.md D-3 and D-4 in review: "LGTM". Touches Structure, Key decisions 3 and 4, Risks.
