# Spec Delta

## MODIFIED Requirements

### Requirement: Health contract and result
A compatible server SHALL answer GET `/health` with HTTP 200 and JSON containing `service` equal to `ship`, `protocol_version` equal to 21, informational string `version`, `log`, the path of the log part the server is writing, and `socket`, the path of the socket or pipe the server listens on. Health response JSON field names SHALL use snake case. `ship server status` SHALL print one health JSON line on stdout only on success, and SHALL never start a server.

#### Scenario: Compatible health
- **WHEN** `ship server status` contacts a compatible server
- **THEN** it prints the health response as one JSON line, including the server's current log part in `log` and its socket path in `socket`, and exits 0

#### Scenario: Log path follows rollover
- **WHEN** the server's log has moved to a new part and a user runs `ship server status`
- **THEN** `log` names the new part

#### Scenario: Status with no server
- **WHEN** no server is running and a user runs `ship server status`
- **THEN** it reports that no server is running, exits 1 and starts nothing
