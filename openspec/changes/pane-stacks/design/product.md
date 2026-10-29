# Stacked panes

Status: Locked on 2026-10-29. Written from a design conversation with Cyan on 2026-10-28 for [#85](https://github.com/shipmux/ship/issues/85), then approved in Plannotator review ("LGTM!"). The feature name changed from "edit history" to "edit scrollback" at Cyan's request during that review. Reopened on 2026-10-28 during architecture design to replace `--stack` with `--place right|down|stack` in FR-005 and FR-006 ("sounds good to me"); re-locked with that change. Reopened on 2026-10-29 during program design to replace `--next|--prev` in FR-007 with `--direction next|prev`, because `docs/agents/code.md` rules out meaning carried by bare flags and swap, resize and scroll already take `--direction` ("go for it"). Re-locked on 2026-10-29 with the program paper ("lock it!"). Amended and re-approved in chat on 2026-10-29 ("yes please. g for it") to remove backward-compatibility requirements before Ship's first release. Earlier commitments to legacy saved formats and creation aliases are superseded. Amended and approved in chat on 2026-10-29 to use `alt-,` for previous and `alt-.` for next after the legacy bracket-key probe failed.

## Goal

Today a layout position holds exactly one pane, so the only way to open something next to a pane is to split it and give up screen space. This change lets a position hold a stack of panes. You see the front one, the rest are listed in the border, and you can cycle through them. Editing a pane's scrollback ([#85](https://github.com/shipmux/ship/issues/85)) is then a small command built on stacks: it opens your editor on top of the pane, in the same place and at the same size, and closing the editor brings the pane back.

The place in the layout that holds a stack is called a slot. Some operations act on the slot and some on a pane in it:

| Operation | Acts on | Effect |
| --- | --- | --- |
| zoom, resize | slot | the slot fills the tab, or its edge moves |
| cycle, stacking create | slot's stack | changes which pane is in front |
| select, swap, move, close | pane | a pane behind the front is brought to the front when selected |

```text
one slot, stack [X, A, B], X in front

┌─ X · A · B ─────────┬─ C ──────┐
│ X's screen           │          │
│                      │          │
└──────────────────────┴──────────┘

alt-. → [A, B, X]          alt-e on X → [edit-scrollback, X, A, B]
```

## User stories

### P1: Stack and cycle panes

- **Given** pane `X` is selected, **when** the user presses `alt-s`, **then** a new shell pane opens in `X`'s slot, in front, at the slot's size, and is selected. `X`'s program keeps running behind it.
- **Given** a slot holds `[X, A, B]`, **when** the user presses `alt-.`, **then** the order is `[A, B, X]`, `A` is shown and selected, and the border lists `A · B · X`.
- **Given** a slot holds `[X, A, B]`, **when** the user presses `alt-,`, **then** the order is `[B, X, A]` and `B` is shown and selected.
- **Given** a slot holds `[X, A]`, **when** the user closes `X`, **then** `A` is shown and selected. **When** the user then closes `A`, **then** the slot is gone and its neighbor takes the space, as closing a pane does today.
- **Given** two clients look at the same tab, **when** one cycles a stack, **then** both see the same front pane.
- **Given** the user clicks `B`'s title in the border of a slot holding `[X, A, B]`, **then** `B` is shown and selected, and the order is `[B, X, A]`.

Tested independently by stacking, cycling and closing panes in a running client and reading the layout back with `ship tab get`.

### P2: Edit a pane's scrollback

- **Given** pane `X` is selected and has output, **when** the user presses `alt-e`, **then** the user's editor opens in a new pane in front of `X`, in `X`'s slot, showing `X`'s full scrollback as plain text.
- **Given** the editor is open, **when** the user quits it, **then** the editor's pane closes and `X` is shown and selected again.
- **Given** `X` was zoomed, **when** the user presses `alt-e` and later quits the editor, **then** the tab stays zoomed throughout.
- **Given** `X` is alone in its slot and the user runs `ship pane edit-scrollback` in it directly, **then** the command fails with a message saying there is no pane behind it, and nothing else changes.
- **Given** the editor can't be started, **then** the pane stays open showing the error until the user closes it.

Tested independently by pressing `alt-e` on a pane with known output, checking the text in the editor, and quitting.

### P3: Zoom a stack

- **Given** `X`'s slot is zoomed, **when** the user cycles, stacks a new pane or opens edit-scrollback, **then** the tab stays zoomed and shows the slot's new front.
- **Given** `X`'s slot holds `[X, A]` and is zoomed, **when** `X` closes, **then** the tab stays zoomed on `A`.
- **Given** `X`'s slot is zoomed, **when** a pane in another slot of the same tab closes, **then** the tab stays zoomed.

Tested independently by zooming a stacked slot and running each of the steps above.

### P4: Move panes between stacks and splits

- **Given** panes `X` and `Y` in different slots, **when** the user runs `ship pane move --pane X --onto Y`, **then** `X` is in front of `Y`'s slot, its program still running, and if `X` was alone in its slot, that slot is gone.
- **Given** `X` is in a stack, **when** the user runs `ship pane move --pane X --beside Y --direction right`, **then** `Y`'s slot is split and `X` sits alone in the new slot to its right.
- **Given** `Y` is in another tab, **then** both forms work the same way.

Tested independently by moving panes with the CLI and reading both tabs back with `ship tab get`.

### P5: Stacks survive a restart

- **Given** a tab with a slot holding `[A, X]`, **when** the server restarts and restores saved tabs, **then** the slot holds the same panes in the same order with `A` in front, each running a fresh shell, as restored panes do today.

Tested independently by saving stacked tabs and restarting the server.

## Non-goals

- A key to take a pane out of a stack. `ship pane move --beside` does it; a default key can come later if it's missed.
- Moving a pane into an empty tab. `move` needs a target pane.
- Dragging panes into or out of stacks with the mouse.
- A different front pane per client. The front is shared, like zoom.
- Saving edits made in edit-scrollback, or sending them back to the pane.
- Restoring edit-scrollback panes as editors. Like every pane with a command, they restore as shells.
- Any server feature specific to editors or edit-scrollback. It is a client command that creates, reads and closes panes through the existing interface.
- Hidden panes and floating panes ([#86](https://github.com/shipmux/ship/issues/86)).
- Pane rows in the sidebar.
- Changing `ship pane swap`. It keeps its flags and its meaning.
- Backward compatibility with earlier development builds' saved formats, flags or config fields.

## Functional requirements

### Stacks

- **FR-001** Every slot MUST hold one or more panes in order. The first pane is the front, and only the front is drawn. A slot MUST never be empty.
- **FR-002** A slot's size MUST come from the layout alone. The front pane's terminal MUST be sized to the slot. Panes behind the front MUST keep the size they last had, and MUST be resized to the slot when they come to the front.
- **FR-003** The front pane of a slot MUST be the same for every client.
- **FR-004** `ship tab get` and every other layout output MUST show each slot's panes in order, front first.

### Creating and cycling

- **FR-005** `ship pane create` MUST accept `--place right|down|stack`, defaulting to `right`. `stack` puts the new pane in front of the anchor pane's slot instead of splitting it. With a tab target, `stack` MUST stack onto the pane a split would have split.
- **FR-006** `server.pane.create` MUST accept the same `place` for key bindings and menus.
- **FR-007** `ship pane cycle --pane X --direction next|prev` and the key action `server.pane.cycle` with the same `direction` MUST rotate `X`'s slot one step, so `[X, A, B]` becomes `[A, B, X]` with `next` and `[B, X, A]` with `prev`. `direction` is required. Cycling a slot with one pane MUST do nothing and succeed.

### Selection

- **FR-008** Selecting a pane behind the front MUST rotate its slot until that pane is in front, keeping the order of the others.
- **FR-009** When a pane stops being the front, every selection on it MUST move to the slot's new front.
- **FR-010** Clicking a pane's title in a slot's border MUST select that pane.
- **FR-011** Selecting a pane in a different slot of a zoomed tab MUST unzoom the tab, as today. Selecting a pane in the zoomed slot MUST NOT.

### Closing

- **FR-012** Closing a pane MUST remove it from its slot. If other panes remain, the next one becomes the front. If it was the last pane, the slot MUST be removed and its neighbor takes the space, as today.

### Zoom

- **FR-013** Zoom MUST belong to a slot: the zoomed tab shows that slot's front pane.
- **FR-014** Zoom MUST stay through cycling, stacking a new pane onto the zoomed slot, and closing or moving out any pane of the zoomed slot while others remain, except when preserving a moved pane's selection requires unzooming under FR-020.
- **FR-015** Zoom MUST stay when a pane in another slot of the same tab closes or moves away, except when preserving a moved pane's selection requires unzooming under FR-020. This changes today's behavior, where any close unzooms.
- **FR-016** Zoom MUST clear when the zoomed slot is removed, and on any split in the tab (including `move --beside`), swap or resize, as today.

### Swap and move

- **FR-017** Swap MUST exchange two panes' places, slot and position in the stack, leaving the layout's shape and sizes unchanged. Directional swap MUST target the neighboring slot's front pane.
- **FR-018** `ship pane move --pane X --onto Y` MUST take `X` out of its slot and put it in front of `Y`'s slot.
- **FR-019** `ship pane move --pane X --beside Y --direction right|down` MUST take `X` out of its slot and split `Y`'s slot, putting `X` alone in the new slot on that side.
- **FR-020** A move MUST keep `X`'s program, terminal contents and ID. `Y` MAY be in any tab. If `X`'s old slot is left empty, it MUST be removed as in FR-012. Selections on `X` MUST stay on `X`. If the destination tab's zoom would hide `X` and any client has `X` selected, the destination MUST unzoom in the same edit, including a move within that tab. Otherwise moving onto a stack MUST preserve its zoom.
- **FR-021** Moving a pane onto or beside itself MUST fail with an error and change nothing.

### Edit-scrollback

- **FR-022** `ship pane edit-scrollback` MUST target the pane directly behind the pane it runs in, or `--pane X` when given. With no such pane it MUST fail with an error and change nothing.
- **FR-023** It MUST open the target's full scrollback and screen as plain text with soft-wrapped lines joined, in the editor named by `VISUAL`, else `EDITOR`, else `vi`. Editor values with arguments, such as `code --wait`, MUST work.
- **FR-024** The temporary file MUST be readable only by the user, MUST be deleted after the editor exits, and files left by a crashed run MUST be cleaned up by the next run. Edits MUST be discarded.
- **FR-025** It MUST exit with a non-zero status when it can't read the scrollback or start the editor.

### Restore

- **FR-026** Saved tabs MUST record each slot's panes in order with its front. On restore, every pane MUST come back in its slot and position, restarted as a shell as today.
- **FR-027** Superseded on 2026-10-29: no backward-compatibility requirement for saved tabs.

### Drawing

- **FR-028** A slot's top border MUST list the titles of all its panes, front first, with the front's title marked. Titles MUST be the labels panes show today.
- **FR-029** A slot without a border, the only slot in a tab, MUST show the title row only while it holds more than one pane.

### Defaults and docs

- **FR-030** Default keys: `alt-s` stacks a new shell onto the selected pane and selects it; `alt-.` and `alt-,` cycle the selected pane's slot next and previous; `alt-e` creates a stacked pane running `ship pane edit-scrollback` with exit behavior `keep_failed`, and selects it.
- **FR-031** The pane menu MUST offer "stack" and "edit scrollback" next to the split items.
- **FR-032** `docs/site/panes.md` and `docs/site/keys.md` MUST describe stacks, cycling, moving and edit-scrollback, and `GLOSSARY.md` MUST define slot and stack.

## Success criteria

- **SC-001** From a pane with output, a user can open its scrollback in their editor with one key and get back to the pane by quitting the editor, without the layout changing in between.
- **SC-002** Stacking, cycling and closing panes never leaves an empty slot or a pane missing from the layout, checked by reading `ship tab get` after each step in P1.
- **SC-003** Every step in P3 leaves the tab zoomed.
- **SC-004** Saved tabs in the slot format load with every pane present, preserving slot order.
- **SC-005** Cycling a stack feels as quick as moving between split panes.

## Open unknowns

None.
