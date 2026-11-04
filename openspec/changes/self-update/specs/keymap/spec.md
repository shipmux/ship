# Spec Delta

## MODIFIED Requirements

### Requirement: Client actions
`client.` actions SHALL have no CLI form and be reachable only from keys and the mouse. They SHALL change only this client's view, except: `client.pane.swap`, `client.pane.zoom` and `client.tab.move` resolve locally and submit one explicit swap pair, zoomed pane or tab move; rename actions submit the typed name; `client.updates.update` creates a tab (see updates); `client.run` starts a command locally (see run-actions).

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

#### Scenario: A rename reaches every viewer on Enter
- **WHEN** a user invokes `client.tab.rename`, types `frontend` and presses Enter
- **THEN** that client sends one `server.tab.set` naming the tab by ID with the name `frontend`, and all viewers see the new name

#### Scenario: Update action creates a shared tab
- **WHEN** a user invokes `client.updates.update` in a release build with a newer release
- **THEN** every viewer sees a new top-level tab named `update`, and only this client's panel follows it

## ADDED Requirements

### Requirement: Updates actions
`client.updates.open = {}` SHALL open the updates panel. `client.updates.check = {}` SHALL check for the latest release at once, as the panel's `[check]` does. `client.updates.update = {}` SHALL start an update, as the panel's `[update]` does, and SHALL do nothing when this install has no update method. None SHALL be bound to a key by default, and each SHALL be usable from any mode's bindings and from menu items.

#### Scenario: Custom binding
- **WHEN** the file binds `"alt-u" = { action = { client.updates.open = {} } }` and the user presses `alt-u`
- **THEN** the updates panel opens

#### Scenario: No default key
- **WHEN** a user with no config presses `alt-m`
- **THEN** no panel opens, and the key reaches the selected pane's program
