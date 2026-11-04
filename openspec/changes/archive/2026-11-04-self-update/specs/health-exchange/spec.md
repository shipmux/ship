# Spec Delta

## MODIFIED Requirements

### Requirement: Current-client health preflight
The current client SHALL validate service/protocol compatibility before any other request, including ordinary commands with `--server-url`, `--server-name`, `SHIP_SERVER_URL` or `SHIP_SERVER_NAME`, and `server stop`. URL targets, status and stop SHALL NOT start a server. Endpoint methods SHALL remain policy-free; application dispatch owns this validation. A protocol mismatch error SHALL name both versions and both protocols and give the restart command and its cost.

#### Scenario: Incompatible or malformed health
- **WHEN** the target returns the wrong status, invalid JSON, wrong service or incompatible protocol version
- **THEN** the client reports contextual failure, emits no success result and exits 1

#### Scenario: Stop preflight
- **WHEN** the current client runs `ship server stop` against incompatible, malformed or wrong-service health
- **THEN** it fails before requesting shutdown and starts nothing

#### Scenario: Different protocol version
- **WHEN** the current client contacts a server with a different protocol version
- **THEN** it reports incompatibility before attempting any other request or attach and does not replace the running server

#### Scenario: Updated client, old server
- **WHEN** a client at version 0.2.0 speaking protocol 22 runs bare `ship` against a 0.1.0 server speaking protocol 21
- **THEN** the error names `0.2.0`, protocol 22, `0.1.0` and protocol 21, and says `restart the server: ship server stop && ship (ends running programs; tabs are restored)`
