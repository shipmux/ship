# Rename tabs and panes from the UI, and a panes mode

Status: Locked on 2026-10-30. Written from a design conversation with Cyan on 2026-10-30 for [#84](https://github.com/shipmux/ship/issues/84), which settled the choices below ("perfect! draft it!"), then approved in Plannotator review ("LGTM"). Re-opened the same day to narrow FR-007's editing keys to those that come without extra work, and approved again ("LGTM").

## Goal

You can't rename a tab or pane from the UI. Key bindings and menu items send actions with fixed arguments, so today renaming means running `ship tab set --name ...` from a shell. This change adds a one-line rename field in the mode bar, opened from tabs mode, a new panes mode, or the right-click menus. It's the last item in milestone 5, getting Ship ready to share ([#42](https://github.com/shipmux/ship/issues/42)).

Panes get a mode of their own, the counterpart of tabs mode, so pane rename has a home and pane moves sit beside it:

```diff
 normal mode
   alt-hjkl         focus a pane                       (unchanged)
   alt-. / alt-,    cycle the stack                    (unchanged)
-  alt-shift-hjkl   swap with the neighbor
+  alt-p            enter panes mode

 tabs mode
+  r                rename the selected tab

+panes mode (sticky)
+  h j k l          focus left / down / up / right
+  shift-h/j/k/l    swap with the neighbor in that direction
+  . / ,            cycle the stack
+  r                rename the selected pane
+  esc              back to normal
```

The field, while open:

```text
┌──────────┬──────────────────────────────────────────┐
│ api      │ ~/web $ npm run dev                      │
│▌web      │   ready on :3000                         │
│ docs     │                                          │
│          │ RENAME TAB  █web                         │  ← dimmed: the automatic label
└──────────┴──────────────────────────────────────────┘
```

The conventions behind this:

- Editing in the status line follows tmux, whose `prefix ,` opens `(rename-window)` there. Ship's labels can't be edited where they're drawn: the sidebar can be hidden, and a lone pane has no title row.
- An empty field means "automatic". Herdr leaves an unchanged automatic name unsent so it isn't pinned by accident; showing the automatic label as a placeholder gets the same result without a special case, and an emptied field clears the name as a bare `--name` does on the CLI.
- Nothing reaches the server until Enter. Zellij renames live on every keystroke and undoes on Esc; in Ship that would show half-typed names to other clients.
- A pane mode with focus and swap keys follows Zellij's pane mode and mirrors Ship's tabs mode, where moves already live outside normal mode.

## User stories

### P1: Rename a tab

Tabs used below: `api` named, `web` unnamed with its first pane in `~/web`.

- **Given** `web` is selected in tabs mode, **when** the user presses `r`, **then** the mode bar shows `RENAME TAB` and an empty field with `web` dimmed as a placeholder, and `web`'s sidebar row takes the accent.
- **Given** the field is open for `web`, **when** the user types `frontend` and presses Enter, **then** the field closes, the sidebar shows `frontend`, `ship tab get` reports the name `frontend`, and tabs mode is still active.
- **Given** `api` is selected, **when** the user presses `r`, **then** the field holds `api` in normal brightness with the cursor at its end.
- **Given** the field holds `api`, **when** the user clears it with `ctrl-u` and presses Enter, **then** `api`'s name is cleared and its label falls back to the automatic one.
- **Given** the field is open, **when** the user presses Esc, **then** the field closes and nothing is sent.

Tested independently by renaming tabs in a running client and reading them back with `ship tab get`.

### P2: Rename a pane

- **Given** a pane running `npm` is selected, **when** the user presses `alt-p` then `r`, **then** the mode bar shows `RENAME PANE` with the pane's automatic label dimmed.
- **Given** the selected pane is the only pane in its tab, so it has no title row, **when** the user renames it, **then** the field works the same and `ship pane get` reports the new name.
- **Given** the user right-clicks a pane, **when** they pick `rename`, **then** the field opens for that pane.
- **Given** the user right-clicks a tab's sidebar row, **when** they pick `rename`, **then** the field opens for that tab.

Tested independently by renaming panes from panes mode and from the menu, and reading them back with `ship pane get`.

### P3: Get around panes in panes mode

- **Given** a tab with two panes side by side and the left one selected, **when** the user presses `alt-p` then `l`, **then** the right pane is selected and panes mode is still active.
- **Given** the same tab, **when** the user presses `shift-l` in panes mode, **then** the two panes swap places and the moved pane stays selected.
- **Given** a stack of three panes, **when** the user presses `.` in panes mode, **then** the next pane in the stack comes to the front.
- **Given** normal mode, **when** the user presses `alt-shift-l`, **then** nothing happens in Ship, and the key reaches the pane's program as any unbound chord does.

Tested independently by walking and swapping panes in a running client and reading the layout back with `ship tab get`.

### P4: The target goes away

- **Given** the field is open for a pane, **when** that pane's program exits and the pane closes, **then** the field closes, the mode bar shows `pane closed`, and nothing is sent.
- **Given** the field is open for a tab, **when** another client closes that tab, **then** the field closes with `tab closed`.

Tested independently by closing the target from a second shell with `ship pane close` or `ship tab close` while the field is open.

## Non-goals

- A general prompt, command line or prompt kind of mode. The field serves rename only; scripts that need input will use floating panes ([#86](https://github.com/shipmux/ship/issues/86)).
- Configurable keys inside the field. Its editing keys are fixed, as the menu's are.
- Editing a label where it's drawn, in the sidebar row or the pane title row.
- Live renaming: other clients see the name only after Enter.
- Setting an empty label from the UI. `ship tab set --name ""` still does that; an empty field clears.
- Creating, closing or zooming panes from panes mode. Those stay normal-mode chords, as tab create and close stay out of tabs mode.
- Moving a pane to another stack or tab from the keyboard.
- Renaming tabs or panes the user hasn't selected or right-clicked.
- Any server change. Renames use the existing `server.tab.set` and `server.pane.set`.

## Functional requirements

### Actions

- **FR-001** `client.tab.rename = {}` MUST open the rename field for the selected tab, or the tab of the selected pane. `client.pane.rename = {}` MUST open it for the selected pane. With nothing to rename selected, they MUST do nothing.
- **FR-002** Each MUST resolve its target at its turn, as `client.tab.move` does, and the field MUST keep that target while open even if the selection moves.
- **FR-003** Both MUST be usable from any mode's bindings and from menu items.

### The field

- **FR-004** The field MUST appear on the mode bar's bottom row with a pill reading `RENAME TAB` or `RENAME PANE`, whether or not the sidebar is shown and whether or not the pane has a title row.
- **FR-005** It MUST open holding the target's stored name, with the cursor at the end. With no stored name it MUST open empty and show the label the target would display without a name, dimmed, as a placeholder.
- **FR-006** While open it MUST take every key, paste and mouse event. No other binding runs, and nothing reaches a pane's program.
- **FR-007** Its keys MUST be fixed: Enter submits, Esc cancels, printable keys and paste insert text, and the usual line-editing keys edit it, including Backspace, left and right, and `ctrl-u` to empty the field. Pasted line breaks MUST become spaces. Editing keys beyond these are kept only where they come without extra work.
- **FR-008** A mouse click MUST cancel the field, as a click outside the menu closes it.
- **FR-009** While open, the target's label MUST take the accent wherever it's drawn: its sidebar row for a tab, its title row for a pane.

### Submitting

- **FR-010** Enter MUST trim surrounding whitespace. If the result is empty it MUST clear the target's name; otherwise it MUST set the name to the result. Both go through `server.tab.set` or `server.pane.set`, as the CLI's `--name` does.
- **FR-011** Esc MUST close the field and send nothing.
- **FR-012** If the target is closed while the field is open, the field MUST close, send nothing, and show `tab closed` or `pane closed` in the mode bar.
- **FR-013** If the server rejects the rename, the field MUST close and the mode bar MUST show the error, as for any failed action.
- **FR-014** While the client is disconnected, Enter MUST drop the rename and close the field, as other input is dropped while disconnected.
- **FR-015** The mode that was active when the field opened MUST still be active after it closes.

### Panes mode

- **FR-016** The defaults MUST define a sticky `panes` mode binding `h`, `j`, `k`, `l` to `client.pane.focus` left, down, up and right; `shift-h`, `shift-j`, `shift-k`, `shift-l` to `client.pane.swap` in those directions, labeled `swap left`, `swap down`, `swap up` and `swap right`; `.` and `,` to `server.pane.cycle` next and previous; `r` to `client.pane.rename`; and `esc` to normal mode.
- **FR-017** Normal mode MUST bind `alt-p` to enter panes mode and MUST NOT bind `alt-shift-h`, `alt-shift-j`, `alt-shift-k` or `alt-shift-l`.
- **FR-018** Tabs mode MUST bind `r` to `client.tab.rename`.

### Menus and docs

- **FR-019** The default tab and pane menus MUST each gain a `rename` item.
- **FR-020** `docs/site/keys.md`, `docs/site/tabs.md` and `docs/site/panes.md` MUST describe renaming and panes mode, and MUST no longer list `alt-shift-hjkl` as a normal-mode key.

## Success criteria

- **SC-001** A user can name, rename and clear the name of any tab or pane using only keys, with no shell and no mouse.
- **SC-002** Every story in P1 and P2 ends with `ship tab get` or `ship pane get` reporting the name the user saw in the field.
- **SC-003** Esc, a click, and a closed target each leave `ship tab list` output byte-for-byte unchanged.
- **SC-004** A second client attached to the same tab sees no change until Enter, then sees the new name.
- **SC-005** Every pane focus, swap and cycle that worked from normal mode before this change works from panes mode, and focus and cycle still work from normal mode.

## Open unknowns

None.
