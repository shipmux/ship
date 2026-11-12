# Proposal

Status: Locked on 2026-11-11. Assembled from `design/product.md`, `design/architecture.md` and `design/program.md`. The papers are the contract, and this proposal and the deltas under `specs/` are views of them.

## Why

Ship's first release freezes the JSON that scripts and older clients read. Today any unknown event type is a decode error, so even a pure addition needs a protocol bump, and every bump forces a server restart that ends running programs. Before 0.1.0, this change settles the names and paths, and makes the common, additive case need no bump.

## What Changes

- Clients skip attach-stream events, `ended` reasons and status records they don't know. `ship watch` passes every line through exactly as sent.
- A running client checks `/health` before each reconnect and exits with the restart guidance when the protocol differs, instead of looping.
- **BREAKING** Pane JSON carries the process state as `process`; `status` is gone.
- **BREAKING** Routes move from `/api/v0/...` to `/api/...`. `/health` stays.
- **BREAKING** `PROTOCOL_VERSION` goes from 23 to 24. A protocol-24 `ship server stop` can't stop a protocol-23 server, whose stop route is still under `/api/v0`; stop it with the older binary or a signal.
- Request bodies keep ignoring unknown fields, as today. Rejecting them was considered and dropped, because it breaks scripts that send a fetched entity back (`design/product.md`, A-1).
- The plugin docs state the compatibility promise and the key-order rule, and the glossary defines Process state.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `session-structure`: the routes move to `/api` ("Versioned HTTP API" becomes "HTTP API prefix").
- `session-observation`: a new "Additions from a newer server" requirement; the `ended` example shows the shape the server sends; paths.
- `observer-recovery`: a new "Server on another protocol" requirement.
- `state-watching`: `watch` prints unknown event types as sent, and ends when the stream closes.
- `health-exchange`: `/health` and the stop route are kept stable like the rest of the API, not frozen; paths.
- `pane-programs`: `process` replaces `status` in pane JSON.
- `session-restore`: the saved file says `process`, and files that say `status` still load.
- `client-maintenance`, `diagnostics`, `pane-automation`, `pane-stacks`, `tags`: paths only.

## Impact

- `crates/ship-core`: `PROTOCOL_VERSION`, `ProcessState`, `SseKind`, `EndReason::Unknown`, status records skipped when they don't decode.
- `crates/ship-client`: `attach_lines` under `attach`, the skipping decoder, `observe_lines`, the reattach health check.
- `crates/ship-server`: route paths.
- `crates/ship`: `watch` as a pipe.
- Docs: `docs/site/scripting/`, `GLOSSARY.md`, `examples/plugins/git.sh`, and `AGENTS.md`'s compatibility rule.
