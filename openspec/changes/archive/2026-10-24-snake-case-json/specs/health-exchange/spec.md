# Spec Delta

## MODIFIED Requirements

### Requirement: Health contract and result
A compatible server SHALL answer GET `/health` with HTTP 200 and JSON containing `service` equal to `ship`, `protocol_version` equal to 16, informational string `version`, and `log`, the path of the log part the server is writing. Health response JSON field names SHALL use snake case. `ship server status` SHALL print one health JSON line on stdout only on success, and SHALL never start a server.

#### Scenario: Compatible health
- **WHEN** `ship server status` contacts a compatible server
- **THEN** it prints the health response as one JSON line, including the server's current log part in `log`, and exits 0

#### Scenario: Log path follows rollover
- **WHEN** the server's log has moved to a new part and a user runs `ship server status`
- **THEN** `log` names the new part

#### Scenario: Status with no server
- **WHEN** no server is running and a user runs `ship server status`
- **THEN** it reports that no server is running, exits 1 and starts nothing

### Requirement: Health check before other requests
The current client SHALL validate service/protocol compatibility before any other request, including ordinary commands with `--server-url` or `SHIP_SERVER_URL` and `server stop`. Explicit targets, status and stop SHALL NOT start a server. Endpoint methods SHALL remain policy-free; application dispatch owns this validation.

#### Scenario: Incompatible or malformed health
- **WHEN** the target returns the wrong status, invalid JSON, wrong service or incompatible protocol version
- **THEN** the client reports contextual failure, emits no success result and exits 1

#### Scenario: Client from an earlier protocol
- **WHEN** a client built for protocol version 1 to 15 validates health from a server reporting protocol version 16
- **THEN** the client reports the incompatible protocol version and exits 1 without attaching

#### Scenario: Already-built protocol-7 explicit ordinary command
- **WHEN** an already-built protocol-7 client runs an ordinary command with an explicit URL against protocol 8 or later
- **THEN** its legacy health bypass can reach the ordinary endpoint, without a compatibility guarantee
- **AND** documentation instructs users to update client and server together; this change does not add a request marker or server guard to distinguish that legacy request

This legacy exception is Cyan's approved slice-1 acceptance adjustment. It does not permit the current client to bypass health.

#### Scenario: Stop preflight
- **WHEN** the current client runs `ship server stop` against incompatible, malformed or wrong-service health
- **THEN** it fails before requesting shutdown and starts nothing

#### Scenario: New client against old server
- **WHEN** a version-11 client contacts a version-10 server
- **THEN** it reports incompatibility before attempting any other request or attach and does not replace the running server

### Requirement: Error bodies
An HTTP error response's body SHALL be one error object. An internal error SHALL have `source` equal to `internal`, `message`, and `type` with the error's `kind` and, for kinds that carry it, `data`. It MAY have `data` and `caused_by`, the error it wraps as a nested error object. An external error SHALL have `source` equal to `external`, `code` shaped like `type`, `error`, a text snapshot of the foreign error, and optional `data`. Keys and kinds SHALL use snake case.

#### Scenario: Missing pane over HTTP
- **WHEN** a client requests a pane that doesn't exist
- **THEN** the 404 body has `"source":"internal"` and `"type":{"kind":"not_found"}`

#### Scenario: Error with a cause
- **WHEN** a request fails with an error that wraps another
- **THEN** the wrapped error appears under `caused_by` as its own object
