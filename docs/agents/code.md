# Writing Code

Rules for agents that write or change Ship's Rust. `AGENTS.md` still applies; this adds what only matters when editing code.

## Size

Get small by expressing each invariant once through Rust types, derives and macros, not through opaque compression or generic frameworks built for hypothetical features.

## Verification

Ship starts with zero authored test code. Verify work with builds, formatting, Clippy, real workflows and temporary probes. Propose a permanent test only after encountering flaky behavior, and add it once Cyan agrees on its coverage.

## Design

Keep one source of truth. Store what can't be derived and compute the rest; duplicated or cached state needs a reason.

Model state as state. When something has a current value, send and store the value, not the sequence of changes that produced it. Reserve event streams for things where each occurrence matters.

Name your shapes. Prefer named types over anonymous inline structure, so data can be reused, documented and passed around. Reuse a named shape instead of restating it.

Say what things mean. A default, an override or a special case gets its own name in the type or config, rather than being implied by position, absence or a bare flag.

Earn every layer. A wrapper, abstraction or coordination mechanism should do something the thing beneath it doesn't. If it only forwards or restates, remove it.

Combine only what needs the result. An endpoint takes a second effect as a parameter only when that effect needs something the call creates, usually the new ID, and stopping between the two would leave a state nobody asked for, like `--starter` and `--name` on create. Everything else is separate calls. Invariants are different: the server repairs those itself, as when closing the selected tab moves the selection to its parent.

Check what's already guaranteed. Before building machinery, find out what the platform and dependencies already handle, verify it, and cover only the gap.

## Entity and request shapes

An entity's `get` output is its one shape. Every input that describes the entity is a subset of it, so a script learns each field once.

- Each entity field has one name, one path and one type in `get` output, PATCH bodies, create bodies and `[server.defaults.*]`. A request field that reuses an entity field's name has that field's type. Config's `shell` is the one exception: it's a path, the default for `command`, an argv.
- PATCH bodies and create's `pane` and `tab` objects use the entity's own paths.
- Create bodies put the request's own fields (`at`, `place`, `starter`, `select`) at the top level and the new entity's fields under `pane` (`NewPane`) or `tab` (`NewTab`). Placing a tab is always `at: MoveTab`, on create and on move.
- Create bodies and config defaults take plain values. Only PATCH bodies and `server.` and `client.` action arguments may also take `"toggle"`, and only on bools, through `Switch`.
- PATCH follows JSON Merge Patch: an absent field is left alone and `null` clears it.
- An input field takes `null` only if the entity field can be `null`. Say it one of two ways in `protocol.rs`:
  - A field that can be empty, such as `name` or `zoomed`, is `Option<Option<T>>` with `#[serde(default, with = "double_option")]`: absent, `null` and a value are three cases, and the schema shows it nullable.
  - Any other optional field is `Option<T>` with `#[serde(default, deserialize_with = "present")]` and `#[schema(nullable = false)]`, so serde and the schema both refuse `null`. Absent means the default.
- Names are stored exactly as sent, including `""` and surrounding spaces. Only `null` clears a name.
- A CLI flag for a field that can be empty has three cases: absent leaves it alone, a bare flag clears it, and a value, including `""`, sets it. Clap reads `Option<Option<T>>` this way, as on `--name` and `--zoomed`.
- A `server.` action's request depends only on its arguments and the caller's context: its attachment, its selected tab or pane, and its current directory. An action that reads other shared state to decide what to send, such as whether the tab is zoomed, is a `client.` action, like `client.pane.zoom`.

## Errors and results

Construct errors with `err!`. Wrap an existing `AppError` with `.context(...)`, which keeps its error code.

Discard a result you don't need with `.ok()`, not `let _ =`.

## Platform-specific code

Keep platform-specific code behind `cfg`. Use the portable std or dependency API wherever one exists. Confine each Unix-only mechanism to one small function or module gated with `#[cfg(unix)]`, and don't call `nix` or `libc` from otherwise portable code. On other platforms, give the item a plain fallback or leave it missing, so a port fails to compile exactly at that boundary.
