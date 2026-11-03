# Spec Delta

## MODIFIED Requirements

### Requirement: Sidebar sections
Whenever the sidebar is shown, it SHALL hold the tabs section on top and the agents section (see agent-attention) below, with a divider row between them, and the version row (see updates) as its bottom row. Each section SHALL scroll on its own when its content overflows, and the tabs section SHALL keep the selected row visible as it does today.

#### Scenario: No agents
- **WHEN** a client shows the sidebar with no pane reporting
- **THEN** the agents section below the divider row shows `no agents`

#### Scenario: Overflow
- **WHEN** a short terminal holds 30 tabs and 20 reporting panes
- **THEN** each section scrolls with the wheel without moving the other

#### Scenario: Version row
- **WHEN** a client shows the sidebar
- **THEN** its bottom row shows the client's version, below the agents section

### Requirement: Status line messages
The status line SHALL show a disconnected indicator while reconnecting, and the selected pane's exit status once its program has exited. It SHALL show the latest config error, a key action's failure, "not available yet" or an update's one-time note (see updates) until the next key action.

#### Scenario: Exited program
- **WHEN** the selected pane's program exits with code 3
- **THEN** the status line appears showing `exited (3)` and stays while that pane is selected

#### Scenario: Update note
- **WHEN** a check first finds `v0.2.0`
- **THEN** the status line shows `ship v0.2.0 is available · ≡ updates` until the next key action
