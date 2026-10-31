# Proposal

Status: Locked through its design papers. `design/product.md`, `design/architecture.md` and `design/program.md` are locked with Cyan's recorded approval, and they are the contract. This proposal and the deltas under `specs/` are views assembled from them.

## Why

Tabs and panes can't be renamed from the UI. Bindings and menu items send actions with fixed arguments, so renaming means running `ship tab set --name ...` from a shell ([#84](https://github.com/shipmux/ship/issues/84), the last item in milestone 5). A field that takes typing needs every key after the rename key to reach it, but today the client decides what a key means when it's pressed, while the action before it may still be waiting on the server.

## What Changes

- **Input is interpreted at its turn.** Keys and pastes wait in the client's input lane unread, and the open overlay or the active mode reads each one once everything before it has taken effect. `client.mode`, `client.sidebar.*` and `client.config.reload` take their turn like other actions instead of applying at press. A right-click opens its menu at its turn. Queued clicks close an overlay that opened before their turn; other queued mouse input that finds one open is dropped.
- **New `client.tab.rename` and `client.pane.rename` actions** open a rename field on the mode bar's bottom row for the selected tab or pane. It holds the stored name, or shows the automatic label as a dimmed placeholder. Enter sets the trimmed text, or clears the name when it's empty. Esc and a click cancel. Line editing comes from `tui-input`.
- **Panes mode** (`alt-p`): `h/j/k/l` focus, `shift-h/j/k/l` swap, `.` and `,` cycle the stack, `r` renames, `esc` leaves. Normal mode no longer binds `alt-shift-h/j/k/l`.
- **Tabs mode `r`** renames the selected tab, and the default tab and pane menus gain a `rename` item.

Not in this change: a general prompt or command line, configurable field keys, editing a label where it's drawn, live renaming, setting an empty label from the UI, creating, closing or zooming panes from panes mode, and any server or protocol change.

## Capabilities

### Modified Capabilities

- `terminal-client`: ordered input decides a key's meaning at its turn, and new requirements cover the rename field.
- `keymap`: the press-time exceptions go, the rename actions are added, and the default keys gain panes mode and tabs mode's `r` and lose normal mode's `alt-shift-h/j/k/l`.
- `context-menus`: menus open and are picked at their turn, at most one overlay is open, and the default pane and tab menus gain `rename`.
- `mouse-input`: clicks that find an overlay open at their turn close it before running.

## Impact

- `ship-client`: `ui/keys.rs`, `ui/lane.rs` and `ui/mod.rs` for input at its turn; a new `ui/rename.rs`; `ui/draw.rs` for split labels and the field row; the sidebar and pane title rows for the accent; `ClientTab::Rename` and `ClientPane::Rename` in `keymap.rs`; a new `tui-input` dependency.
- `ship-core`: `defaults.toml`.
- Docs: `docs/site/keys.md`, `docs/site/tabs.md` and `docs/site/panes.md`.
- Size: about 160 to 180 implementation lines, estimated, not measured. No tests are planned.
