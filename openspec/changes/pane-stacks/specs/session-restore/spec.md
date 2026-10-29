## MODIFIED Requirements

### Requirement: Saved tabs contents
The saved tabs SHALL hold the tab tree with its order and nesting, tab and pane IDs and names, each tab's split layout, proportions and zoomed pane, each slot's panes in order with its front first, and each pane's `right_click_passthrough` setting and current directory, else its starting directory. A shared tab or pane attribute added later SHALL be saved unless its change says otherwise.

#### Scenario: Current directory is saved
- **WHEN** a pane's shell started in `~` changes directory to `~/projects/ship` and the saved tabs are written
- **THEN** that pane's saved directory is `~/projects/ship`

### Requirement: Restore on startup
On startup, before it accepts any connection, the server SHALL rebuild the saved tabs with the same IDs, names, structure, slots and their order, proportions, zoom and each pane's `right_click_passthrough`, `exit_behavior` and `scrollback_bytes`. Every pane in every slot SHALL be restarted. Saved tabs whose leaves are single panes, written before slots existed, SHALL load with each pane alone in its slot. A missing saved file SHALL start with no restored tabs.

#### Scenario: Structure comes back
- **WHEN** a server with nested tabs, named tabs and panes, custom proportions and a zoomed pane restarts
- **THEN** `ship tab list` shows the same IDs, names, nesting, proportions and zoom

#### Scenario: Stack comes back
- **WHEN** a tab with a slot holding `[A, X]` is saved and the server restarts
- **THEN** the slot holds A then X, with A in front, each running a fresh shell

#### Scenario: Saved before slots existed
- **WHEN** a saved file written by a release before slots, with `"kind": "pane"` leaves, is restored
- **THEN** every pane comes back, each alone in its slot, and the next save writes `"kind": "slot"` leaves

#### Scenario: Pane settings come back
- **WHEN** a pane with `exit_behavior: keep` and `scrollback_bytes: 50000000` is saved and the server restarts
- **THEN** the restored pane, a fresh shell, shows the same two values

#### Scenario: Saved before pane settings existed
- **WHEN** a saved file from before exit behavior and per-pane history size is restored
- **THEN** its panes get `exit_behavior: close` and `scrollback_bytes: 10000000`, the built-in defaults

#### Scenario: Reconnecting client
- **WHEN** a client stays open while the server stops and starts again
- **THEN** after reconnecting it keeps the tab or pane it had selected
