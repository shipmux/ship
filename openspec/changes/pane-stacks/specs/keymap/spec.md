## ADDED Requirements

### Requirement: Default stack keys
With no bindings in the file, normal mode SHALL bind `alt-s` to `server.pane.create = { place = "stack", select = true }`, `alt-]` to `server.pane.cycle = { direction = "next" }`, `alt-[` to `server.pane.cycle = { direction = "prev" }`, and `alt-e` to `server.pane.create = { place = "stack", select = true, exit_behavior = "keep_failed", command = ["sh", "-c", "exec \"$SHIP_BIN\" pane edit-scrollback"] }`.

#### Scenario: Stack and cycle from keys
- **WHEN** a user with no config presses `alt-s` on pane X, then `alt-]`, then `alt-[`
- **THEN** a new shell N is stacked and selected in front of X, then X is shown and selected, then N is shown and selected again

#### Scenario: Edit scrollback from a key
- **WHEN** a user with no config presses `alt-e` on a pane with output, then quits the editor
- **THEN** the editor opens in front of that pane showing its scrollback, and after quitting the pane is shown and selected with the layout unchanged

## MODIFIED Requirements

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

### Requirement: Pane creation before layout
Until layout is available, `server.pane.create` SHALL add a shell pane to the selected tab or selected pane's tab, ignoring `place`, and SHALL select it only when the binding sets `select = true`, as in Selecting creates from keys.

#### Scenario: Pane creation before layout
- **WHEN** layout is not yet available, a pane is selected and the user presses `alt-|`
- **THEN** a pane running the shell is added to the selected pane's tab and selected

### Requirement: Selecting creates from keys
`server.tab.create` and `server.pane.create` SHALL accept `select = true`, which selects what they create for the pressing client in the same edit: the new pane, or the new tab when it has none. Without it, a create from a key SHALL NOT change any selection. The defaults SHALL set `select = true` on `alt-n`, `alt-shift-n`, `alt--`, `alt-|`, `alt-s` and `alt-e`. The config SHALL NOT name an attachment. `server.pane.create` SHALL take `place = "right" | "down" | "stack"`, and the defaults SHALL use `place`.

#### Scenario: Default split lands on the new pane
- **WHEN** a user with no config presses `alt-|` then `alt-f`
- **THEN** the new pane is selected and zoomed

#### Scenario: Create without select
- **WHEN** the file binds `"alt-t" = { action = { server.pane.create = { place = "down" } } }` and the user presses `alt-t` with a pane selected
- **THEN** a pane is split below it and the original pane stays selected

#### Scenario: Attachment in the config
- **WHEN** a file binds `"alt-t" = { action = { server.tab.create = { select = true, attachment = "…" } } }` and the user runs `ship config check`
- **THEN** it reports the unknown field `attachment` and exits 1
