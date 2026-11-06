# Spec Delta

## ADDED Requirements

### Requirement: Restart across protocols
`ship server status` and `ship server stop` SHALL check the service and accept a Ship server of any protocol version, so the restart the mismatch error gives always works. In every protocol version, GET `/health` SHALL keep `service`, `protocol_version`, `version`, `log` and `socket` with their types, and MAY add fields, and POST `/api/v0/server/stop` SHALL take no body and answer 202.

#### Scenario: Status of an older server
- **WHEN** a client speaking protocol 23 runs `ship server status` against a Ship server speaking protocol 22
- **THEN** it prints that server's health with `"protocol_version":22` and exits 0

#### Scenario: Stop an older server
- **WHEN** a client speaking protocol 23 runs `ship server stop` against a Ship server speaking protocol 22
- **THEN** the server shuts down and the command exits 0

## MODIFIED Requirements

### Requirement: Current-client health preflight
The current client SHALL validate service/protocol compatibility before any other request, including ordinary commands with `--server-url`, `--server-name`, `SHIP_SERVER_URL` or `SHIP_SERVER_NAME`, except as Restart across protocols allows. URL targets, status and stop SHALL NOT start a server. Endpoint methods SHALL remain policy-free; application dispatch owns this validation. A protocol mismatch error SHALL name both versions and both protocols and give the restart command and its cost.

#### Scenario: Incompatible or malformed health
- **WHEN** the target returns the wrong status, invalid JSON, wrong service or incompatible protocol version
- **THEN** the client reports contextual failure, emits no success result and exits 1

#### Scenario: Stop preflight
- **WHEN** the current client runs `ship server stop` against malformed or wrong-service health
- **THEN** it fails before requesting shutdown and starts nothing

#### Scenario: Different protocol version
- **WHEN** the current client contacts a server with a different protocol version for any command other than `server stop` and `server status`
- **THEN** it reports incompatibility before attempting any other request or attach and does not replace the running server

#### Scenario: Updated client, old server
- **WHEN** a client at version 0.2.0 speaking protocol 22 runs bare `ship` against a 0.1.0 server speaking protocol 21
- **THEN** the error names `0.2.0`, protocol 22, `0.1.0` and protocol 21, and says `restart the server: ship server stop && ship (ends running programs; tabs are restored)`
