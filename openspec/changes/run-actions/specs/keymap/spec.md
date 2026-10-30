# Spec Delta

## MODIFIED Requirements

### Requirement: Server and client actions
Every binding SHALL name one action, under `server.` or `client.`. A `server.` action SHALL change shared state and SHALL be the `ship` command at the same path without `server.`, taking the same action arguments, except `server.run`, which SHALL start a command on the server's machine and SHALL have no CLI form (see run-actions).

#### Scenario: Same action from a key and the CLI
- **WHEN** a user binds `"alt-y" = { action = { server.tab.close = {} } }`, presses it with a tab selected, and on another tab runs `ship tab close --tab <id>`
- **THEN** both tabs are closed the same way

#### Scenario: Server run has no CLI form
- **WHEN** a user binds `server.run = { command = ["touch", "/tmp/hit"] }` and runs `ship run`
- **THEN** the binding works from its key, and `ship run` is reported as an unknown command

### Requirement: Client actions
A `client.` action SHALL change only that client's view and SHALL be reachable only from keys and the mouse, except `client.pane.swap`, which SHALL resolve direction locally and submit the explicit server swap pair, `client.pane.zoom`, which SHALL resolve its switch locally and submit the tab's zoomed pane, `client.tab.move`, which SHALL resolve its direction locally and submit one explicit tab move, and `client.run`, which SHALL start a command on the client's machine (see run-actions). None of them SHALL have a CLI form.

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
