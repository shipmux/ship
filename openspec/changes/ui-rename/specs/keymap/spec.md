# Spec Delta

## MODIFIED Requirements

### Requirement: Default view keys
With no bindings in the file, normal mode SHALL bind `alt-left`/`alt-right` (previous/next visible tab row), `alt-b` (sidebar), `alt-t`/`alt-r` (tabs/resize mode), `alt-pageup`/`alt-pagedown` (scroll the selected pane up/down one page) and `alt-a` (`client.agents.next`). `alt-g` SHALL NOT be bound by default.

#### Scenario: Tabs mode mnemonic
- **WHEN** a user with no config presses `alt-t`
- **THEN** the client enters tabs mode

#### Scenario: Former tabs chord reaches the program
- **WHEN** a pane runs a program that prints its input and the user presses `alt-g` in normal mode
- **THEN** the client stays in normal mode and the program receives the chord

#### Scenario: Scroll defaults
- **WHEN** a user with no config presses `alt-pageup` in a shell pane with a long history, then `alt-pagedown`
- **THEN** the pane scrolls up one page and back down, without either chord reaching the shell

#### Scenario: Jump default
- **WHEN** a user with no config has a blocked agent in another tab and presses `alt-a`
- **THEN** the blocked agent's pane is selected, and the chord doesn't reach the program

### Requirement: Default pane keys
With no bindings in the file, normal mode SHALL bind `alt--`/`alt-|` (split down/right), `alt-shift-x` (close pane), `alt-h/j/k/l` (pane focus), `alt-tab` (next pane), `alt-f` (`client.pane.zoom = { zoomed = "toggle" }`) and `alt-p` (`client.mode = "panes"`). Normal mode SHALL NOT bind `alt-shift-h/j/k/l`.

#### Scenario: Zoom and swap defaults
- **WHEN** a user with no config presses `alt-f` in an unzoomed split tab, and later presses `alt-p`
- **THEN** `alt-f` invokes `client.pane.zoom = { zoomed = "toggle" }`, and `alt-p` enters `panes` mode

#### Scenario: Shifted focus chord reaches the program
- **WHEN** a pane runs a program that prints its input and a user with no config presses `alt-shift-l` in normal mode
- **THEN** nothing happens in Ship and the program receives the chord

### Requirement: Client actions
A `client.` action SHALL change only that client's view and SHALL be reachable only from keys and the mouse, except `client.pane.swap`, which SHALL resolve direction locally and submit the explicit server swap pair, `client.pane.zoom`, which SHALL resolve its switch locally and submit the tab's zoomed pane, `client.tab.move`, which SHALL resolve its direction locally and submit one explicit tab move, and `client.tab.rename` and `client.pane.rename`, which SHALL submit the name typed into the rename field. None of them SHALL have a CLI form.

#### Scenario: Local resolution submits shared swap
- **WHEN** a user invokes `client.pane.swap` right in an unzoomed tab
- **THEN** that client resolves the neighbor using published geometry and local recency, while all viewers observe the resulting server-owned layout edit

#### Scenario: Local resolution submits shared zoom
- **WHEN** a user invokes `client.pane.zoom = { zoomed = "toggle" }` on a pane in an unzoomed tab
- **THEN** that client decides to zoom from the tab it shows and sends the pane as the tab's zoomed pane, and all viewers see the pane fill the tab

#### Scenario: Local resolution submits a shared tab move
- **WHEN** a user invokes `client.tab.move.direction = "up"` on a tab with a previous sibling
- **THEN** that client sends one `server.tab.move` placing the tab before that sibling, and all viewers see the new order

#### Scenario: A rename reaches every viewer on Enter
- **WHEN** a user invokes `client.tab.rename`, types `frontend` and presses Enter
- **THEN** that client sends one `server.tab.set` naming the tab by ID with the name `frontend`, and all viewers see the new name

### Requirement: Default tabs keys
Default `tabs` mode SHALL bind `j`/`k` to `client.tab.next`/`prev`, `h`/`l` to `client.tab.collapse`/`expand`, `r` to `client.tab.rename`, `esc` to normal, and `shift-k`, `shift-j`, `shift-l` and `shift-h` to `client.tab.move` `up`, `down`, `in` and `out`, with the labels `move up`, `move down`, `move in` and `move out`.

#### Scenario: Labels in tabs mode
- **WHEN** a user with no config presses `alt-t`
- **THEN** the status line shows `move up`, `move down`, `move in` and `move out` for the four shifted keys, and `rename` for `r`

### Requirement: Modes
`normal` SHALL always exist and SHALL be the mode a client starts in. Setting `kind` on `normal` SHALL change nothing. Users SHALL be able to define modes under `[client.modes.<name>]` with `kind` set to `sticky` or `oneshot`. `client.mode` SHALL enter the named mode. Naming an undefined mode SHALL load, and pressing the key SHALL show the error on the status line and change nothing. The defaults SHALL define sticky `tabs`, `panes` and `resize` and an empty one-shot `prefix` that no key enters.

#### Scenario: Undefined mode
- **WHEN** a binding is `{ action = { client.mode = "pnaes" } }`, no `pnaes` mode is defined, and the user presses it
- **THEN** the status line says there is no mode named `pnaes` and the client stays in its mode

#### Scenario: tmux-style prefix
- **WHEN** the file binds `"ctrl-b" = { action = { client.mode = "prefix" } }` in normal and `"c" = { action = { server.tab.create = {} } }` in `prefix`, and the user presses `ctrl-b` then `c`
- **THEN** a tab is created and the client is back in normal mode

### Requirement: Key actions act on the selection
A key binding SHALL act on the client's selection as it is at the binding's turn in the client's input order, not at press (see terminal-client's Ordered input). A `server.` action whose target is not selected, such as `server.pane.close` with only a tab selected, SHALL report that on the status line and change nothing. Every action SHALL take its turn, including `client.mode`, `client.sidebar.toggle`, `client.sidebar.resize` and `client.config.reload`.

#### Scenario: Close the selected pane
- **WHEN** a pane is selected and the user presses `alt-shift-x`
- **THEN** that pane closes

#### Scenario: Pane action with a tab selected
- **WHEN** a tab without panes is selected and the user presses `alt-shift-x`
- **THEN** the status line reports that no pane is selected and nothing changes

#### Scenario: Close what the previous key made
- **WHEN** on a slow link the user presses `alt-|` then `alt-shift-x` before the split appears
- **THEN** the new pane closes, not the one that was selected at the first press

#### Scenario: Mode change while input waits
- **WHEN** on a slow link the user presses `alt-n` then `alt-r` before the tab appears
- **THEN** the tab is created as usual, and the status line shows `resize` once it has

#### Scenario: Mode change on an idle link
- **WHEN** nothing is waiting and the user presses `alt-t`
- **THEN** the status line shows `tabs` at once

### Requirement: Sidebar resize availability
`client.sidebar.resize` SHALL take its turn like any action, SHALL do nothing while the sidebar is hidden, and SHALL have no CLI command.

#### Scenario: Hidden sidebar
- **WHEN** the sidebar is hidden and the user presses `shift-l` in resize mode
- **THEN** nothing changes, and showing the sidebar again shows its earlier width

## ADDED Requirements

### Requirement: Rename actions
`client.tab.rename = {}` SHALL open the rename field for the selected tab, or the selected pane's tab. `client.pane.rename = {}` SHALL open it for the selected pane. Each SHALL resolve its target at its turn, and SHALL do nothing when nothing it can rename is selected. Both SHALL be usable from any mode's bindings and from menu items.

#### Scenario: Rename the tab of a selected pane
- **WHEN** a pane in tab `web` is selected and the user invokes `client.tab.rename`
- **THEN** the rename field opens for `web`

#### Scenario: Nothing to rename
- **WHEN** a tab without panes is selected and the user invokes `client.pane.rename`
- **THEN** no field opens and nothing changes

#### Scenario: Target follows input order
- **WHEN** on a slow link the user presses `j` then `r` in `tabs` mode before the selection moves
- **THEN** the field opens for the tab `j` selected

### Requirement: Default panes keys
Default `panes` mode SHALL be sticky and bind `h`, `j`, `k` and `l` to `client.pane.focus` left, down, up and right; `shift-h`, `shift-j`, `shift-k` and `shift-l` to `client.pane.swap` in those directions, labeled `swap left`, `swap down`, `swap up` and `swap right`; `.` and `,` to `server.pane.cycle` next and previous; `r` to `client.pane.rename`; and `esc` to normal.

#### Scenario: Focus stays in panes mode
- **WHEN** a tab has two panes side by side, the left one is selected, and the user presses `alt-p` then `l`
- **THEN** the right pane is selected and the client is still in `panes` mode

#### Scenario: Swap from panes mode
- **WHEN** the same tab is shown and the user presses `shift-l` in `panes` mode
- **THEN** the two panes swap places and the moved pane stays selected

#### Scenario: Cycle a stack
- **WHEN** a stack holds three panes and the user presses `.` in `panes` mode
- **THEN** the next pane in the stack comes to the front

#### Scenario: Labels in panes mode
- **WHEN** a user with no config presses `alt-p`
- **THEN** the status line shows `swap left`, `swap down`, `swap up` and `swap right` for the four shifted keys, and `rename` for `r`
