# Proposal

Status: Draft. Assembled from `design/product.md`, `design/architecture.md` and `design/program.md`. The papers are the contract, and this proposal and the deltas under `specs/` are views of them.

## Why

Programs that ask their terminal for focus reports, such as editors and agent CLIs, never hear any inside Ship. The client already knows when its terminal gains and loses focus, but uses it only for unseen marks.

## What Changes

- A pane has focus while any attachment whose terminal has focus has it selected.
- Programs that enabled focus reports (mode 1004) get `\e[I` and `\e[O` as that changes, and their current focus as soon as they enable reports. Programs that didn't get nothing.
- Attach requests, `PUT /attach/view` and published viewing records gain `focused`.
- Focus doesn't change how cursors are drawn.

## Capabilities

### Modified Capabilities

- `session-observation`: each attachment's focus, and focus reports to programs.
- `health-exchange`: protocol version 23.

## Impact

- `ship-core`: `focused` on `AttachRequest`, `ViewInput` and `ViewingRecord`; `PROTOCOL_VERSION` 22 to 23, so a client and server from either side of this change refuse each other with the existing restart guidance.
- `ship-server`: `apply_focus` in `commit`, and `PaneCommand::Focus`.
- `ship-client`: the view PUT carries focus.
- `vendor/ratatui-ghostty`: the session reports only to programs that ask, and stops tying focus to cursor rendering.
- Docs: one sentence in `docs/site/windows-and-the-server.md`.
