# keymap Specification

## Purpose

Let users drive the client from the keyboard through modes of chord bindings, where each binding names a shared server action or a client-only view action, with Alt-chord defaults.

## Requirements

### Requirement: Default keys
With no bindings in the file, normal mode SHALL bind `alt-n` (new sibling tab with shell), `alt-shift-n` (new child tab with shell), `alt-x` (close tab) and `alt-q` (detach). `alt-1..9` SHALL NOT be bound by default.

#### Scenario: Herdr chords on the first run
- **WHEN** a user with no config runs `ship` and presses `alt-n`, `alt-|`, `alt-right`, `alt-left`, `alt-tab` and `alt-q`
- **THEN** a tab with a shell is created and selected, a pane is split right and selected, the selection moves between visible tab rows and panes, and the client detaches

#### Scenario: Unbound key reaches the program
- **WHEN** a pane runs `cat -v` and the user presses `ctrl-b` in normal mode
- **THEN** the pane shows `^B`

#### Scenario: Row-number defaults removed
- **WHEN** a user with no config presses `alt-1` in normal mode with a pane selected
- **THEN** the chord reaches the program rather than attempting row selection

### Requirement: Default pane keys
With no bindings in the file, normal mode SHALL bind `alt--`/`alt-|` (split down/right), `alt-shift-x` (close pane), `alt-h/j/k/l` (pane focus), `alt-tab` (next pane), `alt-f` (`client.pane.zoom = { zoomed = "toggle" }`) and `alt-shift-h/j/k/l` (client directional swap).

#### Scenario: Zoom and swap defaults
- **WHEN** a user with no config presses `alt-f` and later uses `alt-shift-l` in an unzoomed split tab
- **THEN** `alt-f` invokes `client.pane.zoom = { zoomed = "toggle" }`, and `alt-shift-l` invokes `client.pane.swap` to resolve the right neighbor and submit its explicit ID pair

### Requirement: Default view keys
With no bindings in the file, normal mode SHALL bind `alt-left`/`alt-right` (previous/next visible tab row), `alt-b` (sidebar), `alt-g`/`alt-r` (tabs/resize mode), `alt-pageup`/`alt-pagedown` (scroll the selected pane up/down one page) and `alt-a` (`client.agents.next`).

#### Scenario: Scroll defaults
- **WHEN** a user with no config presses `alt-pageup` in a shell pane with a long history, then `alt-pagedown`
- **THEN** the pane scrolls up one page and back down, without either chord reaching the shell

#### Scenario: Jump default
- **WHEN** a user with no config has a blocked agent in another tab and presses `alt-a`
- **THEN** the blocked agent's pane is selected, and the chord doesn't reach the program

### Requirement: Default resize keys
The default resize mode's `h/j/k/l` SHALL omit `amount`, moving one cell per press. Resize mode SHALL also bind `shift-h` and `shift-l` to `client.sidebar.resize` left and right, with the hints `sidebar left` and `sidebar right`, and `shift-j` and `shift-k` to `client.sidebar.resize` down and up, with the hints `split down` and `split up`.

#### Scenario: Sidebar keys in resize mode
- **WHEN** a user with no config presses `alt-r` and then `shift-l` three times
- **THEN** the sidebar is 3 columns wider for that client

#### Scenario: Split keys in resize mode
- **WHEN** a user with no config presses `alt-r` and then `shift-j` twice
- **THEN** the divider between the sidebar's sections moves 2 rows down for that client

### Requirement: Server and client actions
Every binding SHALL name one action, under `server.` or `client.`. A `server.` action SHALL change shared state and SHALL be the `ship` command at the same path without `server.`, taking the same action arguments, except `server.run`, which SHALL start a command on the server's machine and SHALL have no CLI form (see run-actions).

#### Scenario: Same action from a key and the CLI
- **WHEN** a user binds `"alt-y" = { action = { server.tab.close = {} } }`, presses it with a tab selected, and on another tab runs `ship tab close --tab <id>`
- **THEN** both tabs are closed the same way

#### Scenario: Server run has no CLI form
- **WHEN** a user binds `server.run = { command = ["touch", "/tmp/hit"] }` and runs `ship run`
- **THEN** the binding works from its key, and `ship run` is reported as an unknown command

### Requirement: Client actions
`client.` actions SHALL have no CLI form and SHALL be reachable only from keys and the mouse. They SHALL change only that client's view, except: `client.pane.swap` SHALL resolve direction locally and submit the explicit server swap pair; `client.pane.zoom` SHALL resolve its switch locally and submit the tab's zoomed pane; `client.tab.move` SHALL resolve direction locally and submit one explicit tab move; `client.run` SHALL start a command on the client's machine (see run-actions).

#### Scenario: Local resolution submits shared swap
- **WHEN** a user invokes `client.pane.swap` right in an unzoomed tab
- **THEN** that client resolves the neighbor using published geometry and local recency, while all viewers observe the resulting server-owned layout edit

#### Scenario: Local resolution submits shared zoom
- **WHEN** a user invokes `client.pane.zoom = { zoomed = "toggle" }` on a pane in an unzoomed tab
- **THEN** that client decides to zoom from the tab it shows and sends the pane as the tab's zoomed pane, and all viewers see the pane fill the tab

#### Scenario: Local resolution submits a shared tab move
- **WHEN** a user invokes `client.tab.move.direction = "up"` on a tab with a previous sibling
- **THEN** that client sends one `server.tab.move` placing the tab before that sibling, and all viewers see the new order

#### Scenario: Client run starts a local command
- **WHEN** a user invokes `client.run = { command = ["touch", "/tmp/hit"] }`
- **THEN** `/tmp/hit` exists on the client's machine and no other client sees any change

### Requirement: Relative tab moves
`client.tab.move` SHALL take a required `direction` of `"up"`, `"down"`, `"in"` or `"out"`. It SHALL move the selected tab, or the selected pane's tab, with its subtree and panes, in one tab move: `up` to just before its previous sibling, `down` to just after its next sibling, `in` to the last child of its previous sibling, and `out` to just after its parent, leaving its siblings in place. Folded siblings SHALL count. The moved tab, or the pane selected in it, SHALL stay selected.

#### Scenario: Move up
- **WHEN** the top level is `api`, `web`, `notes`, `notes` is selected, and the user presses `shift-k` in `tabs` mode
- **THEN** the top level is `api`, `notes`, `web`, and `notes` stays selected

#### Scenario: Move in
- **WHEN** `api` has children `server` and `tests`, `tests` is selected, and the user presses `shift-l` in `tabs` mode
- **THEN** `tests` becomes `server`'s last child with its panes, and its programs keep running

#### Scenario: Move into a folded sibling
- **WHEN** `web` is folded, `notes` follows it and is selected, and the user presses `shift-l` in `tabs` mode
- **THEN** `notes` becomes `web`'s last child, `web` unfolds in that client, and `notes`'s row is visible and selected

#### Scenario: Move out
- **WHEN** `api` has children `server` and `tests`, `server` is selected, and the user presses `shift-h` in `tabs` mode
- **THEN** `server` sits at the top level right after `api`, and `tests` stays under `api`

#### Scenario: Move from a selected pane
- **WHEN** a pane in `tests` is selected and the user presses `shift-k` in `tabs` mode
- **THEN** `tests` moves above `server`, and the same pane stays selected

#### Scenario: Folded siblings still count
- **WHEN** `web` is folded with children, `notes` follows it and is selected, and the user presses `shift-k`
- **THEN** `notes` moves to just before `web`, stepping over `web`'s hidden rows as one sibling

### Requirement: Tab moves at their turn
`client.tab.move` SHALL resolve its direction against the tree at its turn in the client's input order, then SHALL be handled exactly as a mouse drop to the same destination, including unfolding a folded new parent in that client. With no selection, or `up` on a first sibling, `down` on a last sibling, `in` without a previous sibling, or `out` on a top-level tab, it SHALL send no request and show no error.

#### Scenario: Repeated presses
- **WHEN** a tab with three previous siblings is selected and the user presses `shift-k` three times quickly
- **THEN** the tab moves up three places

#### Scenario: Nothing happens at an edge
- **WHEN** a top-level first tab is selected and the user presses `shift-k`, `shift-h` or `shift-l`, or a last sibling is selected and the user presses `shift-j`
- **THEN** no move request is sent, `ship tab list` output is unchanged, and no error is shown

#### Scenario: Same as a drop
- **WHEN** one tab is moved `in` from the keyboard, and the same tab in an identical tree is dropped on the same new parent with the mouse
- **THEN** both produce the same tree

### Requirement: Default tabs keys
Default `tabs` mode SHALL bind `j`/`k` to `client.tab.next`/`prev`, `h`/`l` to `client.tab.collapse`/`expand`, `esc` to normal, and `shift-k`, `shift-j`, `shift-l` and `shift-h` to `client.tab.move` `up`, `down`, `in` and `out`, with the labels `move up`, `move down`, `move in` and `move out`.

#### Scenario: Labels in tabs mode
- **WHEN** a user with no config presses `alt-g`
- **THEN** the status line shows `move up`, `move down`, `move in` and `move out` for the four shifted keys

### Requirement: Bindings in any mode
Any action SHALL be bindable in any mode.

#### Scenario: Resize action in normal mode
- **WHEN** a user binds a `server.pane.resize` action in normal mode
- **THEN** the file loads without error

### Requirement: Chord spelling
Chords SHALL be case-insensitive and written as crokey parses them: `"alt-X"` means `alt-x`, Shift is written as `shift-`, and modifier order does not matter. When two spellings of one chord appear in the same mode, one of them SHALL win; which one is unspecified.

#### Scenario: Two spellings of one chord
- **WHEN** a mode binds both `"alt-n"` and `"Alt-N"`
- **THEN** the file loads and `alt-n` runs one of the two bindings

### Requirement: Overriding defaults
A chord in the file SHALL replace the default binding on that chord in that mode, and other default chords SHALL stay bound. `{}` SHALL unbind a chord. `clear_defaults = true` SHALL drop all of a mode's default bindings before the file's apply.

#### Scenario: Unbind a default
- **WHEN** the file binds `"alt-q" = {}` in normal mode and the user presses `alt-q`
- **THEN** the client stays attached and the program receives `alt-q`

#### Scenario: Rebind a default
- **WHEN** the file binds `"alt-n" = { action = { server.tab.create = {} } }` and the user presses `alt-n`
- **THEN** an empty tab is created and the selection stays where it was: the default's `starter` and `select` don't carry over

#### Scenario: Override under another spelling
- **WHEN** the file binds `"Alt-N" = { action = { server.tab.close = {} } }` in normal mode
- **THEN** the file loads without error and `alt-n` closes the selected tab

#### Scenario: Clear a mode
- **WHEN** the file sets `clear_defaults = true` on `resize` and binds only `esc`
- **THEN** in resize mode `h` does nothing and `esc` returns to normal

### Requirement: Modes
`normal` SHALL always exist and SHALL be the mode a client starts in. Setting `kind` on `normal` SHALL change nothing. Users SHALL be able to define modes under `[client.modes.<name>]` with `kind` set to `sticky` or `oneshot`. `client.mode` SHALL enter the named mode. Naming an undefined mode SHALL load, and pressing the key SHALL show the error on the status line and change nothing. The defaults SHALL define sticky `tabs` and `resize` and an empty one-shot `prefix` that no key enters.

#### Scenario: Undefined mode
- **WHEN** a binding is `{ action = { client.mode = "pnaes" } }`, no `pnaes` mode is defined, and the user presses it
- **THEN** the status line says there is no mode named `pnaes` and the client stays in its mode

#### Scenario: tmux-style prefix
- **WHEN** the file binds `"ctrl-b" = { action = { client.mode = "prefix" } }` in normal and `"c" = { action = { server.tab.create = {} } }` in `prefix`, and the user presses `ctrl-b` then `c`
- **THEN** a tab is created and the client is back in normal mode

### Requirement: Unbound keys by mode
In normal mode, an unbound key SHALL reach the selected pane's program. In a sticky mode other than normal, a mode SHALL stay active until another mode is entered, and an unbound key SHALL do nothing. A one-shot mode SHALL return to normal after one key, bound or not, unless that key's action enters a mode.

#### Scenario: Sticky mode swallows keys
- **WHEN** the client is in `tabs` mode and the user presses `x`, which `tabs` does not bind
- **THEN** nothing is sent to the program and the client stays in `tabs`

#### Scenario: One-shot mode exits on an unbound key
- **WHEN** the client is in a one-shot mode and the user presses an unbound key
- **THEN** the client returns to normal and nothing else happens

### Requirement: Key actions act on the selection
A key binding SHALL act on the client's selection as it is at the binding's turn in the client's input order, not at press (see terminal-client's Ordered input). A `server.` action whose target is not selected, such as `server.pane.close` with only a tab selected, SHALL report that on the status line and change nothing. `client.mode`, `client.sidebar.toggle`, `client.sidebar.resize` and `client.config.reload` SHALL apply at press, ahead of any input still waiting.

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
- **THEN** the status line shows `resize` at once, and the tab is created as usual

### Requirement: New tabs from keys
`server.tab.create` from a key with no placement SHALL create the tab after the selected tab under the same parent, or at the end of the top level when nothing is selected. Child placement SHALL append under the selected tab or the selected pane's tab, falling back to the top level with nothing selected. With `starter = "shell"`, the tab SHALL open with one pane running the configured shell. Whether the client selects the new tab or its pane follows Selecting creates from keys.

#### Scenario: New tab after the selected one
- **WHEN** the first of three top-level tabs is selected and the user presses `alt-n`
- **THEN** a new tab with a shell sits second among the top-level tabs and its pane is selected

#### Scenario: New last child
- **WHEN** a pane's tab already has two children and the user presses `alt-shift-n`
- **THEN** a new tab with a shell is appended as its third child and its pane is selected

#### Scenario: Child key with no selection
- **WHEN** nothing is selected and the user presses `alt-shift-n`
- **THEN** a top-level tab with a shell is created and its pane is selected

### Requirement: Actions before their feature
Every action SHALL be accepted in the config even when its feature isn't built. Using one whose feature isn't built SHALL report "not available yet" and change nothing. Layout, focus, folding and sidebar actions SHALL work once provided by this change. `client.tab.select` SHALL remain accepted and unavailable, with no default row-number bindings.

#### Scenario: Not available yet
- **WHEN** a user binds `client.tab.select` with a valid row and invokes it
- **THEN** the status line shows "not available yet" and the selection stays unchanged

#### Scenario: Pane creation with layout
- **WHEN** layout is available, a pane is selected and the user presses `alt-|`
- **THEN** a pane running the shell is split right from that pane and selected

### Requirement: Pane cycling
`client.pane.next` and `client.pane.prev` SHALL select the next and previous slot front in the viewed tab's layout order, wrapping around, skipping panes behind a front. With a tab selected rather than a pane, they SHALL select its first or last own slot front. Next from a zoomed pane SHALL first unzoom the tab for everyone.

#### Scenario: Cycle panes
- **WHEN** a tab has three panes, each alone in its slot, and the user presses `alt-tab` three times
- **THEN** selection visits each pane in layout order and returns to the first

#### Scenario: Skip panes behind
- **WHEN** a tab has a slot holding `[X, A]` beside a slot holding C, X is selected, and the user presses `alt-tab` twice
- **THEN** selection visits C, then X, and the slot still holds `[X, A]`

#### Scenario: Empty parent with populated child
- **WHEN** an empty tab with a populated child is selected and pane-next is invoked
- **THEN** selection stays on the parent rather than entering the child's panes

### Requirement: Send a chord
`client.send` SHALL send its chord as if typed at its turn in the client's input order, to the selected pane or the landing pane of a selected tab, without changing the selection.

#### Scenario: Pass the prefix through
- **WHEN** `prefix` binds `"ctrl-b" = { action = { client.send = "ctrl-b" } }`, a pane runs `cat -v`, and the user enters `prefix` and presses `ctrl-b`
- **THEN** the pane shows one `^B`

### Requirement: Directional client swap
`client.pane.swap` SHALL resolve its direction using published geometry and client-local recency with focus's tie rules, and send an explicit same-tab pair. In a zoomed tab it SHALL first unzoom the tab for everyone and wait for the unzoomed geometry before resolving; with no neighbor the tab SHALL stay unzoomed. With no neighbor it SHALL make no swap request. `ship pane swap` SHALL require an explicit `--other` pane and SHALL NOT accept a direction.

#### Scenario: Interactive recency tie
- **WHEN** B and C are adjacent to the right of A and the client last selected C before swapping right from A
- **THEN** it sends the pair A and C, not direction or recency

#### Scenario: Direct CLI pair
- **WHEN** the CLI names a source and an explicit other pane
- **THEN** it submits that pair without neighbor resolution or a geometry read

#### Scenario: Swap out of zoom
- **WHEN** a client invokes `client.pane.swap` right from a zoomed pane with a right neighbor
- **THEN** the tab unzooms for every viewer, and the pane is then exchanged with the neighbor found in the unzoomed geometry

#### Scenario: Zoomed swap without a neighbor
- **WHEN** a client invokes `client.pane.swap` left from a zoomed pane at the tab's left edge
- **THEN** the tab unzooms for every viewer and no swap request is sent

#### Scenario: Missing neighbor
- **WHEN** a directional swap has no neighbor in an unzoomed tab
- **THEN** no swap request or layout change occurs

#### Scenario: CLI without a target
- **WHEN** a user runs `ship pane swap` without `--other`
- **THEN** it rejects the arguments before submitting a swap

### Requirement: Binding syntax
Documented binding syntax SHALL be a table with optional `action`, holding exactly one action, and optional `label`, its status-line hint: `"alt-r" = { action = { client.mode = "resize" } }`. `{}` SHALL unbind the chord. An action written without `action`, or any other key in the table, SHALL be a config error with its line and column. Strict rejection of positional list forms is not required, per Cyan's slice-1 amendment; their incidental acceptance by Serde is not documented binding syntax.

#### Scenario: Accepted forms
- **WHEN** a file binds `"alt-t" = { action = { server.tab.create = {} } }`, `"alt-y" = { action = { server.tab.close = {} }, label = "close" }` and `"alt-q" = {}`, and the user runs `ship config check`
- **THEN** it prints nothing and exits 0

#### Scenario: An action without `action`
- **WHEN** a file binds `"alt-t" = { server.tab.create = {} }` and the user runs `ship config check`
- **THEN** it reports the unknown key `server` at that binding's line and column and exits 1

#### Scenario: A misspelled key
- **WHEN** a file binds `"alt-t" = { action = { server.tab.create = {} }, hnt = "x" }` and the user runs `ship config check`
- **THEN** it reports the unknown key `hnt` with its line and column and exits 1

#### Scenario: Obsolete hint field
- **WHEN** a binding contains `hint = "fill"` instead of `label = "fill"` and the user runs `ship config check`
- **THEN** it reports the unknown field with its line and column and exits 1; no compatibility alias is accepted

### Requirement: Selecting creates from keys
`server.tab.create` and `server.pane.create` SHALL accept `select = true`, which selects what they create for the pressing client in the same edit: the new pane, or the new tab when it has none. Without it, a create SHALL NOT explicitly select its result; ordinary front and zoom repair still apply. A selecting create SHALL unzoom the tab in the same edit if needed to keep its new pane visible. The config SHALL NOT name an attachment.

#### Scenario: Create without select
- **WHEN** the file binds `"alt-t" = { action = { server.pane.create = { place = "down" } } }` and the user presses `alt-t` with a pane selected
- **THEN** a pane is split below it and the original pane stays selected

#### Scenario: Stack without select repairs the front
- **WHEN** X is selected and a binding stacks N onto X's slot without `select = true`
- **THEN** N becomes the front and selection repairs to N, without a separate selecting-create effect

#### Scenario: Selecting create behind another slot's zoom
- **WHEN** X is zoomed and a binding stacks N onto another slot with `select = true`
- **THEN** the tab unzooms and the pressing client's selection stays on N

#### Scenario: Attachment in the config
- **WHEN** a file binds `"alt-t" = { action = { server.tab.create = { select = true, attachment = "…" } } }` and the user runs `ship config check`
- **THEN** it reports the unknown field `attachment` and exits 1

### Requirement: Default selecting create bindings
The defaults SHALL set `select = true` on `alt-n`, `alt-shift-n`, `alt--`, `alt-|`, `alt-s` and `alt-e`.

#### Scenario: Default split lands on the new pane
- **WHEN** a user with no config presses `alt-|` then `alt-f`
- **THEN** the new pane is selected and zoomed

### Requirement: Pane placement in create bindings
`server.pane.create` SHALL take `place = "right" | "down" | "stack"`, and the defaults SHALL use `place`.

#### Scenario: Create binding stacks a pane
- **WHEN** the file binds `"alt-t" = { action = { server.pane.create = { place = "stack", select = true } } }` and the user presses `alt-t` with a pane selected
- **THEN** a new pane appears at the front of that slot without adding a split

### Requirement: Sidebar resize action
`client.sidebar.resize` SHALL take a `direction` of `left`, `right`, `up` or `down` and an optional `amount`, defaulting to 1. `left` and `right` SHALL move the sidebar's right edge that many columns under config-file's Client sidebar width, as a dragged width does. Any other direction SHALL be a config error with its line and column.

#### Scenario: Bound amount
- **WHEN** the file binds `"alt-[" = { action = { client.sidebar.resize = { direction = "left", amount = 5 } } }` and the user presses it
- **THEN** the sidebar is 5 columns narrower

#### Scenario: Unknown direction
- **WHEN** the file binds `client.sidebar.resize = { direction = "sideways" }` and the user runs `ship config check`
- **THEN** it reports the bad value with its line and column and exits 1

### Requirement: Vertical sidebar resize
For `client.sidebar.resize`, `up` and `down` SHALL move the divider between the sidebar's sections by `amount` rows, as a dragged divider row does, within terminal-client's Sidebar split limits.

#### Scenario: Vertical direction
- **WHEN** the file binds `"alt-]" = { action = { client.sidebar.resize = { direction = "up", amount = 3 } } }` and the user presses it
- **THEN** the divider between the sidebar's sections moves 3 rows up, unless the tabs section would drop below its limit

### Requirement: Sidebar resize availability
`client.sidebar.resize` SHALL apply at press, SHALL do nothing while the sidebar is hidden, and SHALL have no CLI command.

#### Scenario: Hidden sidebar
- **WHEN** the sidebar is hidden and the user presses `shift-l` in resize mode
- **THEN** nothing changes, and showing the sidebar again shows its earlier width

### Requirement: Set actions replace rename
`server.tab.set` and `server.pane.set` SHALL be bindable with the fields and toggle behavior of session-structure's Set attribute fields and Two-valued attributes, using snake_case field names, such as `server.pane.set = { right_click_passthrough = "toggle" }`. `server.tab.rename` and `server.pane.rename` SHALL NOT exist, so a binding naming either SHALL be a config error with its line and column, as any unknown action is.

#### Scenario: Toggle from a key
- **WHEN** the file binds `"alt-p" = { action = { server.pane.set = { right_click_passthrough = "toggle" } } }` and the user presses it twice
- **THEN** the selected pane's passthrough turns on and then off

#### Scenario: Old rename binding
- **WHEN** the file binds `"alt-e" = { action = { server.tab.rename = { name = "x" } } }` and the user runs `ship config check`
- **THEN** it reports the unknown action with its line and column and exits 1

### Requirement: Agents action
`client.agents.next = {}` SHALL select the next pane that needs the user, as agent-attention's Jump to what needs the user describes. It SHALL have no CLI command.

#### Scenario: Custom binding
- **WHEN** the file binds `"alt-y" = { action = { client.agents.next = {} } }` and an agent is blocked
- **THEN** pressing `alt-y` selects the blocked agent's pane

### Requirement: What makes an action a server action
A `server.` action's request SHALL depend only on its arguments and the caller's context: its attachment, its selected tab or pane, and its current directory. An action that reads other shared state to decide what to send SHALL be a `client.` action.

#### Scenario: Zoom reads the tab
- **WHEN** an action has to know whether the selected pane's tab is zoomed to decide what to send
- **THEN** it is `client.pane.zoom`, not a `server.` action

#### Scenario: Set reads only its arguments
- **WHEN** a user binds `server.pane.set = { right_click_passthrough = "toggle" }`
- **THEN** its request is the selected pane and `"toggle"`, and the server resolves the toggle against the stored value

### Requirement: Zoom action
`client.pane.zoom` SHALL take a required `zoomed` argument of `true`, `false` or `"toggle"`. `true` SHALL zoom the selected pane, `false` SHALL unzoom its tab, and `"toggle"` SHALL zoom the selected pane unless its tab is already zoomed on it, in which case it SHALL unzoom. The client SHALL decide from the tab it shows and send the zoom through the tab's set route.

#### Scenario: Toggle twice
- **WHEN** a user with no config presses `alt-f` twice on a pane in a split tab
- **THEN** the pane fills the tab, then the tab shows every pane again

#### Scenario: Only zoom
- **WHEN** the file binds `"alt-z" = { action = { client.pane.zoom = { zoomed = true } } }` and the user presses it three times on a pane
- **THEN** the pane fills the tab after the first press and stays zoomed

#### Scenario: Only unzoom
- **WHEN** the file binds `"alt-u" = { action = { client.pane.zoom = { zoomed = false } } }` and the user presses it on an unzoomed tab, then on a zoomed one
- **THEN** the first press changes nothing, and the second unzooms the tab

#### Scenario: Missing argument
- **WHEN** the file binds `"alt-z" = { action = { client.pane.zoom = {} } }` and the user runs `ship config check`
- **THEN** it reports the missing `zoomed` field with its line and column and exits 1

#### Scenario: Two clients toggle at once
- **WHEN** two clients viewing the same unzoomed tab press `alt-f` on the same pane at the same time
- **THEN** the tab ends up zoomed on that pane rather than flipping twice

#### Scenario: Nothing selected
- **WHEN** a tab without panes is selected and the user presses `alt-f`
- **THEN** nothing is sent and nothing changes

### Requirement: No server zoom action
`server.pane.zoom` SHALL NOT exist, so a binding naming it SHALL be a config error with its line and column, as any unknown action is.

#### Scenario: Old zoom binding
- **WHEN** the file binds `"alt-f" = { action = { server.pane.zoom = {} } }` and the user runs `ship config check`
- **THEN** it reports the unknown action with its line and column and exits 1

### Requirement: Default stack keys
With no bindings in the file, normal mode SHALL bind `alt-s` to `server.pane.create = { place = "stack", select = true }`, `alt-.` to `server.pane.cycle = { direction = "next" }`, `alt-,` to `server.pane.cycle = { direction = "prev" }`, and `alt-e` to `server.pane.create = { place = "stack", select = true, exit_behavior = "keep_failed", name = "scrollback", command = ["sh", "-c", "exec \"$SHIP_BIN\" pane edit-scrollback"] }`.

#### Scenario: Stack and cycle from keys
- **WHEN** a user with no config presses `alt-s` on pane X, then `alt-.`, then `alt-,`
- **THEN** a new shell N is stacked and selected in front of X, then X is shown and selected, then N is shown and selected again

#### Scenario: Edit scrollback from a key
- **WHEN** a user with no config presses `alt-e` on a pane with output, then quits the editor
- **THEN** the editor opens in front of that pane showing its scrollback, and after quitting the pane is shown and selected with the layout unchanged

#### Scenario: Scrollback pane label
- **WHEN** a user with no config presses `alt-e` on a pane with output
- **THEN** the new pane's label is `scrollback`, not `sh`

### Requirement: Server-authoritative pane placement
Create actions SHALL send their target and `place` to the server. The server SHALL resolve placement, and clients SHALL render the published result rather than substitute a different placement based on local layout availability.

#### Scenario: Placement comes from the server
- **WHEN** a binding targets an existing pane with `place = "stack"`
- **THEN** the server stacks the new pane there and the client renders the published layout
