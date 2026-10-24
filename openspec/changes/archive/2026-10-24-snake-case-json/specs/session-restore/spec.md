# Spec Delta

## MODIFIED Requirements

### Requirement: Restore on startup
On startup, before it accepts any connection, the server SHALL rebuild the saved tabs with the same IDs, names, structure, proportions, zoom and each pane's `right_click_passthrough`, `exit_behavior` and `scrollback_bytes`. A missing saved file SHALL start with no restored tabs.

#### Scenario: Structure comes back
- **WHEN** a server with nested tabs, named tabs and panes, custom proportions and a zoomed pane restarts
- **THEN** `ship tab list` shows the same IDs, names, nesting, proportions and zoom

#### Scenario: Pane settings come back
- **WHEN** a pane with `exit_behavior: keep` and `scrollback_bytes: 50000000` is saved and the server restarts
- **THEN** the restored pane, a fresh shell, shows the same two values

#### Scenario: Saved before pane settings existed
- **WHEN** a saved file from before exit behavior and per-pane history size is restored
- **THEN** its panes get `exit_behavior: close` and `scrollback_bytes: 10000000`, the built-in defaults

#### Scenario: Reconnecting client
- **WHEN** a client stays open while the server stops and starts again
- **THEN** after reconnecting it keeps the tab or pane it had selected
