# Spec Delta

## MODIFIED Requirements

### Requirement: Default pane keys
With no bindings in the file, normal mode SHALL bind `alt--`/`alt-|` (split down/right), `alt-shift-x` (close pane), `alt-h/j/k/l` (pane focus), `alt-tab` (next pane), `alt-f` (`client.pane.zoom = { zoomed = "toggle" }`) and `alt-shift-h/j/k/l` (client directional swap).

#### Scenario: Zoom and swap defaults
- **WHEN** a user with no config presses `alt-f` and later uses `alt-shift-l` in an unzoomed split tab
- **THEN** `alt-f` invokes `client.pane.zoom = { zoomed = "toggle" }`, and `alt-shift-l` invokes `client.pane.swap` to resolve the right neighbor and submit its explicit ID pair

### Requirement: Client actions
A `client.` action SHALL change only that client's view and SHALL be reachable only from keys and the mouse, except `client.pane.swap`, which SHALL resolve direction locally and submit the explicit server swap pair, and `client.pane.zoom`, which SHALL resolve its switch locally and submit the tab's zoomed pane. Neither SHALL have a CLI form.

#### Scenario: Local resolution submits shared swap
- **WHEN** a user invokes `client.pane.swap` right in an unzoomed tab
- **THEN** that client resolves the neighbor using published geometry and local recency, while all viewers observe the resulting server-owned layout edit

#### Scenario: Local resolution submits shared zoom
- **WHEN** a user invokes `client.pane.zoom = { zoomed = "toggle" }` on a pane in an unzoomed tab
- **THEN** that client decides to zoom from the tab it shows and sends the pane as the tab's zoomed pane, and all viewers see the pane fill the tab

## ADDED Requirements

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
