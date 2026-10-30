# Spec Delta

## MODIFIED Requirements

### Requirement: Default pane menu
With no `pane` target in the file, the pane menu SHALL use the labels, actions and order in the Default pane menu scenario.

#### Scenario: Default pane menu
- **WHEN** a user with no config right-clicks a pane
- **THEN** the menu shows split right, split down, stack, edit scrollback, zoom, right-click passthrough, rename and close, in that order
- **AND** each entry uses its matching action below

| Label | Action |
| --- | --- |
| split right | `server.pane.create = { place = "right", select = true }` |
| split down | `server.pane.create = { place = "down", select = true }` |
| stack | `server.pane.create = { place = "stack", select = true }` |
| edit scrollback | the same action as the default `alt-e` binding |
| zoom | `client.pane.zoom = { zoomed = "toggle" }` |
| right-click passthrough | `server.pane.set = { right_click_passthrough = "toggle" }` |
| rename | `client.pane.rename = {}` |
| close | `server.pane.close = {}` |

#### Scenario: Rename from the pane menu
- **WHEN** a user right-clicks a pane and picks `rename`
- **THEN** the rename field opens for that pane

### Requirement: Default tab menu
With no `tab` target in the file, the tab menu SHALL be, in order:

| Label | Action |
| --- | --- |
| new tab | `server.tab.create = { starter = "shell", select = true }` |
| new child tab | `server.tab.create = { starter = "shell", child = true, select = true }` |
| rename | `client.tab.rename = {}` |
| close | `server.tab.close = {}` |

#### Scenario: New tab beside a row
- **WHEN** the tab menu is open on tab B's row and the user picks `new tab`
- **THEN** a tab with a shell is created right after B under B's parent and its pane is selected, and picking `new child tab` instead appends it as B's last child

#### Scenario: Rename from the tab menu
- **WHEN** a user right-clicks tab B's sidebar row and picks `rename`
- **THEN** the rename field opens for B

### Requirement: Selection when a menu opens
A right-click that opens the pane or tab menu SHALL first select the clicked pane or tab for that client, as a left click on it does. The menu SHALL open, and the selection SHALL be made, at the right-click's turn in input order, so keys typed before the right-click don't reach the menu. Opening the global menu SHALL NOT change selection. When the target's list is empty, a right-click SHALL act as it did before menus existed, and SHALL NOT select anything it wouldn't have selected then.

#### Scenario: Right-click an unselected pane
- **WHEN** a user right-clicks in an unselected shell pane
- **THEN** that pane is selected and the pane menu opens at the pointer

#### Scenario: Empty target
- **WHEN** the file sets `pane = []` and a user right-clicks a pane
- **THEN** no menu opens and the right-click acts as it did before menus existed

#### Scenario: Keys typed before a right-click
- **WHEN** 500 ms of delay is added each way, and the user presses `alt-n`, types `ls`, and right-clicks a pane at once
- **THEN** `ls` reaches the new tab's shell, and the menu opens after it with its first item highlighted

### Requirement: Using an open menu
A menu SHALL open at the pointer and be moved as needed to fit fully on the client's screen. Its items SHALL appear in config order with the first highlighted. A menu SHALL belong to the client that opened it, and other clients see nothing of it. At most one menu or rename field SHALL be open. Lists taller than the terminal SHALL show a viewport that follows keyboard navigation. A terminal resize SHALL close the menu.

#### Scenario: Fits the screen
- **WHEN** a user right-clicks a pane near the bottom-right corner of the terminal
- **THEN** the whole menu is drawn on screen

#### Scenario: Other clients
- **WHEN** one client has a menu open
- **THEN** a second attached client shows no menu and its keys reach its panes as usual

#### Scenario: Right-click during a rename
- **WHEN** the rename field is open and the user right-clicks a pane
- **THEN** the field closes without sending anything, and no menu opens

### Requirement: Menu items run as key actions
An item's action SHALL take its turn in the client's input order and behave exactly as the same action bound to a key, acting on the selection at its turn. This SHALL include mode changes, the sidebar toggle and config reloads. A failure SHALL show on the status line as a key action's does.

#### Scenario: Split from the menu
- **WHEN** the pane menu is open and the user clicks `split right`
- **THEN** the pane splits to the right, the new pane is selected, and the menu closes

#### Scenario: After a pending create
- **WHEN** a delayed create is still waiting, and the user right-clicks a pane and picks `zoom`
- **THEN** the clicked pane is the one zoomed

#### Scenario: Press-time item
- **WHEN** the user picks `hide sidebar` from the global menu while earlier input is waiting
- **THEN** the sidebar hides once that input has taken effect
