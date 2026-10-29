# Proposal

This is the planning artifact for a locked plan. Cyan locked the three papers in `design/` on 2026-10-29. The papers are the source of truth; this proposal and the deltas under `specs/` are views of them. Approval does not start implementation. Cyan approved removing backward-compatibility requirements on 2026-10-29 because Ship has not had its first release; the earlier compatibility commitments are superseded.

## Why

A layout position holds exactly one pane, so the only way to open something next to a pane is to split it and give up screen space. Letting a position hold a stack of panes fixes that, and makes editing a pane's scrollback in your editor ([#85](https://github.com/shipmux/ship/issues/85)) a small client command: open the editor in front of the pane, quit it, and the pane is back where it was.

## What Changes

- **Slots and stacks.** Each layout leaf is a slot holding one or more panes in order. The front pane is drawn and sized to the slot. The panes behind it keep running at their last size. The front is shared by every client.
- **BREAKING: layout shape.** A leaf serializes as `{"kind": "slot", "panes": [...]}`, front first, instead of `{"kind": "pane", ...}`. The protocol version goes to 20.
- **BREAKING: create body.** `POST /api/v0/panes` takes `place` (`{"split": "right"}`, `{"split": "down"}` or `"stack"`) instead of `direction`. The CLI and bindings take a flat `place = right | down | stack`.
- **Cycle.** `ship pane cycle --direction next|prev`, `server.pane.cycle` and `POST /api/v0/panes/{id}/cycle` rotate a slot one step.
- **Move.** `ship pane move --pane X --onto Y | --beside Y --direction D` and `POST /api/v0/panes/{id}/move` move a pane into another slot or split, in any tab, keeping its program and ID.
- **Selection and zoom follow the front.** Selecting a pane behind the front brings it forward. A selection on a pane pushed behind moves to the new front. Zoom belongs to a slot and stays through cycling, stacking and closing panes in the zoomed slot or in other slots. It clears on split, swap and resize as before.
- **Swap** exchanges two panes' slot and position in the stack.
- **Titles.** A slot's top border lists all its panes' titles, front first and marked. A lone stacked slot gets a title row. Clicking a title selects that pane.
- **Edit-scrollback.** `ship pane edit-scrollback` opens the scrollback of the pane behind it in `VISUAL`, else `EDITOR`, else `vi`, from a private temp file it deletes afterwards. Edits are discarded. It's Unix-only for now.
- **Defaults.** `alt-s` stacks a shell, `alt-]` and `alt-[` cycle, `alt-e` opens edit-scrollback in a stacked pane. The pane menu gains "stack" and "edit scrollback". Existing bindings move from `direction` to `place`.
- **Docs.** `docs/site/panes.md`, `docs/site/keys.md` and `GLOSSARY.md` describe slots, stacks, cycling, moving and edit-scrollback.

Not in this change (the product paper lists them all):
- A key to take a pane out of a stack, and moving a pane into an empty tab.
- Dragging panes into or out of stacks.
- A different front per client.
- Saving edits from edit-scrollback, or restoring edit-scrollback panes as editors.
- Hidden and floating panes ([#86](https://github.com/shipmux/ship/issues/86)), and pane rows in the sidebar.

## Capabilities

### New Capabilities

- `pane-stacks`: slots, the front pane, stacking creates, cycling, bringing a pane forward on select, closing within a stack, moving panes, and the slot layout shape.

### Modified Capabilities

- `session-structure`: create's `place`, layout order through stacks, closing and swapping in slots, the create body, and zoom naming the slot's front.
- `session-observation`: selection repair to the front, geometry for fronts only, zoom kept through stack edits and other slots' closes, and selecting within a zoomed slot.
- `session-restore`: slots saved and restored in order.
- `health-exchange`: protocol 20.
- `keymap`: `alt-s`, `alt-]`, `alt-[` and `alt-e`, `place` in bindings, and pane cycling over fronts only.
- `context-menus`: the default pane menu gains "stack" and "edit scrollback", and uses `place`.
- `mouse-input`: title clicks, and a behind pane's title winning over a border grab.
- `terminal-client`: stack titles in borders and the lone stacked slot's title row.
- `pane-history`: edit-scrollback.

## Impact

- `ship-core`: `Layout::Slot(Slot)` with `Slot` and its methods in `layout.rs`, `Place`, `CyclePane`, `MovePane` and `PROTOCOL_VERSION` 20 in `protocol.rs`, `Placement`, `Cycle`, `Move` and `EditScrollback` in `command.rs`, new bindings in `defaults.toml`. `mitsein` with `serde` becomes a dependency.
- `ship-server`: `state/layout.rs` gains `stack`, `cycle`, `bring_to_front` and `take` (replacing `close`), geometry emits fronts and the title row, `state.rs` gains the cycle and move handlers and the front repair rule, and `routes.rs` gains two routes.
- `ship-client`: `api.rs` and `execute.rs` for cycle, move and edit-scrollback, `automation.rs` for the editor launch, and `ui/` for titles, clicks, `alt-tab` and tab landing. `etcetera`, `tempfile` and tokio's `process` feature are added.
- Docs: `docs/site/panes.md`, `docs/site/keys.md`, `GLOSSARY.md`, `docs/agents/code.md`.
- Measured line counts go in [tasks.md](tasks.md).
