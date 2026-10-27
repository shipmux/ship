# Spec Delta

## MODIFIED Requirements

### Requirement: Health contract and result
A compatible server SHALL answer GET `/health` with HTTP 200 and JSON containing `service` equal to `ship`, `protocol_version` equal to 18, informational string `version`, and `log`, the path of the log part the server is writing. Health response JSON field names SHALL use snake case. `ship server status` SHALL print one health JSON line on stdout only on success, and SHALL never start a server.

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
- **WHEN** a client built for protocol version 1 to 17 validates health from a server reporting protocol version 18
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

## ADDED Requirements

### Requirement: HTTP/2 transport
The client SHALL speak HTTP/2 to the server without upgrade negotiation, and the server SHALL accept both that and HTTP/1.1 on its one listener. When the health check of an explicit target fails over HTTP/2 for a reason other than a refused connection or a timeout, the client SHALL retry it once over HTTP/1.1. If that succeeds, the client SHALL log a warning naming the target and use HTTP/1.1 for the rest of the run. A local server, launched or found, SHALL always use HTTP/2.

#### Scenario: Local server
- **WHEN** a user runs `ship` against the local server
- **THEN** the server's debug log shows the client's requests arriving over HTTP/2

#### Scenario: HTTP/1.1-only proxy
- **WHEN** a user runs `ship --server-url` pointing at a proxy that speaks only HTTP/1.1 in front of a server
- **THEN** the client logs a warning that it fell back to HTTP/1.1, attaches and works

#### Scenario: Nothing listening
- **WHEN** a user runs `ship server status --server-url` at a port where nothing listens
- **THEN** it reports the refused connection once, without trying HTTP/1.1
