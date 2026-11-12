# observer-recovery Specification

## Purpose

Keep a running client useful across temporary connection loss by keeping its last screens, reconnecting automatically and resuming from current server state.

## Requirements

### Requirement: Report and reconnect
When a running client loses its connection, it SHALL keep showing the last screens, show a disconnected indicator in its status line and automatically retry, waiting between attempts with increasing delays of at most five seconds. Reconnecting SHALL NOT replay or repeat any mutation.

#### Scenario: Connection drops
- **WHEN** the network path between an attached client and a running server is cut
- **THEN** the client keeps its last screen, its status line shows that it is disconnected and reconnecting, and it keeps retrying at intervals no longer than five seconds

### Requirement: Resume with current state
After reconnecting, the client SHALL replace its view with the server's current state and screens, including every change made while it was disconnected, without requiring any user action.

#### Scenario: Edits during an outage
- **WHEN** edits are made and the selected pane prints output while a client is disconnected, and the connection is then restored
- **THEN** the client shows the current structure and the pane's current screen without any further action

### Requirement: Selection after reconnecting
On reconnecting, a client SHALL keep its previous selection if that tab or pane still exists anywhere on the server, subject to shared zoom repair onto the zoomed pane when the previous pane is hidden, and otherwise SHALL select nothing. Retention SHALL NOT depend on disk persistence. Reconnecting SHALL NOT create a tab.

#### Scenario: Selection still present
- **WHEN** a client reconnects and its selected pane still exists and is not hidden by shared zoom
- **THEN** the pane remains selected

#### Scenario: Selection removed or moved away
- **WHEN** a client reconnects after its selected pane's tab moved to the top level, and another client reconnects after its selected pane's tab was removed
- **THEN** the first keeps the pane selected unless shared zoom hides it, and the second selects nothing and shows the nothing-selected hint

#### Scenario: Zoom changed while disconnected
- **WHEN** a client reconnects with a surviving pane selection hidden by its tab's shared zoom
- **THEN** the server repairs it to the zoomed pane and the client shows that pane without replaying a mutation

### Requirement: Keys during an outage
While disconnected, the client SHALL discard keys and pastes the user produces. They SHALL NOT be delivered after the connection returns.

#### Scenario: Typing while disconnected
- **WHEN** a user types `ls` and Enter while the client is disconnected, and the connection is then restored
- **THEN** the pane never receives those keys

### Requirement: Server on another protocol
Before each reconnect attempt, a running client SHALL read the server's `/health`. If its `protocol_version` differs from the client's, the client SHALL exit with the protocol mismatch error `ship` gives at startup, without attaching. When an event of a known type doesn't decode on a server whose protocol matches, the client SHALL exit with the decode error and SHALL NOT reconnect, since a new attach would get the same event.

#### Scenario: Server replaced by another protocol
- **WHEN** a client is attached, the server stops, and a server speaking another protocol starts under the same name
- **THEN** the client's next reconnect attempt ends it with the mismatch error naming both protocols and the restart command, and the new server receives no attach

#### Scenario: Events that don't decode
- **WHEN** a server on the client's protocol sends an event of a known type the client can't decode after every attach
- **THEN** the client exits non-zero with the decode error after that one attach
