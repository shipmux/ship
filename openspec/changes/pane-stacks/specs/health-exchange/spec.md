## MODIFIED Requirements

### Requirement: Health contract and result
A compatible server SHALL answer GET `/health` with HTTP 200 and JSON containing `service` equal to `ship`, `protocol_version` equal to 20, informational string `version`, `log`, the path of the log part the server is writing, and `socket`, the path of the socket or pipe the server listens on. Health response JSON field names SHALL use snake case. `ship server status` SHALL print one health JSON line on stdout only on success, and SHALL never start a server.

#### Scenario: Compatible health
- **WHEN** `ship server status` contacts a compatible server
- **THEN** it prints the health response as one JSON line, including the server's current log part in `log` and its socket path in `socket`, and exits 0

#### Scenario: Log path follows rollover
- **WHEN** the server's log has moved to a new part and a user runs `ship server status`
- **THEN** `log` names the new part

#### Scenario: Status with no server
- **WHEN** no server is running and a user runs `ship server status`
- **THEN** it reports that no server is running, exits 1 and starts nothing

## REMOVED Requirements

### Requirement: Health check before other requests
**Reason**: Replaced by current-client validation without guarantees about historical client binaries or transports.
**Migration**: Use Current-client health preflight below. Client and server must use the same protocol.

## ADDED Requirements

### Requirement: Current-client health preflight
The current client SHALL validate service/protocol compatibility before any other request, including ordinary commands with `--server-url`, `--server-name`, `SHIP_SERVER_URL` or `SHIP_SERVER_NAME`, and `server stop`. URL targets, status and stop SHALL NOT start a server. Endpoint methods SHALL remain policy-free; application dispatch owns this validation.

#### Scenario: Incompatible or malformed health
- **WHEN** the target returns the wrong status, invalid JSON, wrong service or incompatible protocol version
- **THEN** the client reports contextual failure, emits no success result and exits 1

#### Scenario: Stop preflight
- **WHEN** the current client runs `ship server stop` against incompatible, malformed or wrong-service health
- **THEN** it fails before requesting shutdown and starts nothing

#### Scenario: Different protocol version
- **WHEN** the current client contacts a server with a different protocol version
- **THEN** it reports incompatibility before attempting any other request or attach and does not replace the running server
