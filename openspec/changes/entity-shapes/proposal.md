# Proposal

Status: Locked through its design papers. `design/product.md`, `design/architecture.md` and `design/program.md` are locked with Cyan's recorded approval, and they are the contract. This proposal and the deltas under `specs/` are views assembled from them.

## Why

The API spells the same thing several ways. A pane's `right_click_passthrough` is a bool from `get`, a bool or `"toggle"` in PATCH, and a field mixed in with `at` and `direction` on create. Zoom is a pane ID on the tab but `true`/`false` on a pane route. A blank name clears in one place, and `null` is ignored in another. Typos in create bodies are silently dropped (#75). Scripts and agents have to learn each route on its own.

## What Changes

- **BREAKING: one shape per entity.** Every input is a subset of what `get` returns, with the same names, paths and types. Config's `shell` is the one exception.
- **BREAKING: nested create bodies.** `POST /panes` takes `{at, direction, select, pane: {…}}` and `POST /tabs` takes `{at, starter, select, tab: {…}}`. `POST /tabs/{id}/move` takes `{"at": …}`, the same type as create's `at`.
- **BREAKING: PATCH is JSON Merge Patch.** An absent field is left alone, and `null` clears. Only `name` and the tab's `zoomed` take `null`; any other field sent `null` gets 400. `"toggle"` stays on PATCH bools.
- **BREAKING: names are stored as sent.** `""` and `"  x "` are names. Only `null` clears one, and on the CLI a bare `--name` clears.
- **BREAKING: zoom is a tab field.** `PATCH /tabs/{id} {"zoomed": "pane:…"}` zooms and `null` unzooms. `ship tab set --zoomed` replaces `ship pane zoom`. `POST /panes/{id}/zoom` and `server.pane.zoom` are removed.
- **New `client.pane.zoom` action** with a required `zoomed` argument (`true`, `false` or `"toggle"`), resolved from what the client shows. `alt-f` and the pane menu's `zoom` item pass `"toggle"`.
- **One validation error.** Every JSON body and query string rejects unknown fields. A bad body gets 400 `validation` listing every problem, each with a JSON Pointer. A bad query string gets the same error with one problem. Input stream lines decode the same way.
- **A rule for `server.` and `client.` actions.** A `server.` action's request depends only on its arguments and the caller's context. One that reads other shared state is `client.`.
- **BREAKING: protocol 17.**

Not in this change: listing every problem in a query string, compatibility with protocol 16, the saved tabs file's format, swap/resize/scroll shapes beyond rejecting unknown fields, clearing a name from a keybinding, a `"toggle"` value for the tab's `zoomed`, and an ADR.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `session-structure`: names stored as sent, Merge Patch set fields, nested create and move bodies, the tab's `zoomed` attribute, tri-state CLI flags, and 400 `validation` for bodies and queries.
- `session-observation`: shared zoom is set through the tab, not toggled on a pane.
- `keymap`: `alt-f` binds `client.pane.zoom = { zoomed = "toggle" }`, the new zoom action, and the `server.`/`client.` rule.
- `context-menus`: the default pane menu's `zoom` item and the examples that used `server.pane.zoom`.
- `terminal-client`: hint examples that used `server.pane.zoom`, and clearing a tab name with a bare `--name`.
- `pane-automation`: a bad input stream line gets 400 `validation`; mouse input without an attachment keeps 422.
- `health-exchange`: the protocol version is 17.

## Impact

- `ship-core`: `protocol.rs` (new `NewPane`, `NewTab`, `MoveTabBody`; `SetTab` gains `zoomed`; `PaneInput`, `PaneSpec`, `ZoomPane` go; `deny_unknown_fields` and `present` across request types; protocol 17), `model.rs` (`OptionalName` goes), `error.rs` (`Problem`), `command.rs` (tri-state `--name`, `--zoomed`, `pane zoom` goes), `defaults.toml`.
- `ship-server`: new `body.rs` with `Body<T>` and `Query<T>` extractors and the schema explainer; routes, attach, views, input stream and state actor move onto them; `jsonschema` 0.60 (default features off) and `serde_path_to_error` added.
- `ship-client`: `client.pane.zoom` in the keymap and UI, the unzoom-before-moving rule matched by an explicit list, CLI execute and API builders for the new bodies.
- Docs: `docs/agents/code.md` gains "Entity and request shapes"; `docs/site/panes.md`, `tabs.md` and `configuration.md` drop the old shapes.
- Breaks every protocol 16 client and script. Saved tabs files still load.
