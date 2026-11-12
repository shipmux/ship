# Spec Delta

## MODIFIED Requirements

### Requirement: Restart across protocols
`ship server status` and `ship server stop` SHALL check the service and accept a Ship server of any protocol version, so the restart the mismatch error gives always works. GET `/health` and POST `/api/server/stop` SHALL be kept as stable as the rest of the API: a change to either SHALL have a stated reason, given in the release notes.

#### Scenario: Status of an older server
- **WHEN** a client speaking protocol 25 runs `ship server status` against a Ship server speaking protocol 24
- **THEN** it prints that server's health with `"protocol_version":24` and exits 0

#### Scenario: Stop an older server
- **WHEN** a client speaking protocol 25 runs `ship server stop` against a Ship server speaking protocol 24
- **THEN** the server shuts down and the command exits 0

### Requirement: Local requests only
The server SHALL answer a request only when its host, from the HTTP/2 authority or else the HTTP/1 `Host` header, is `localhost`, an IPv4 address in `127.0.0.0/8` or `[::1]`, with or without a port, and it carries no `Origin` header. Any other request, including one with no host, SHALL get status 403 with an error body whose `.type.kind` is `forbidden`, and SHALL NOT reach a route. This applies to every route, `/health` included. The client SHALL address the server as `localhost`.

#### Scenario: Ship client
- **WHEN** a user runs `ship server status` against the default server
- **THEN** health succeeds as before

#### Scenario: DNS rebinding
- **WHEN** a request over the server's socket carries `Host: attacker.example`
- **THEN** the server responds 403 with `.type.kind` equal to `forbidden` and the route doesn't run

#### Scenario: Browser request to the loopback address
- **WHEN** a request to `/api/server/stop` carries `Host: localhost` and `Origin: https://attacker.example`
- **THEN** the server responds 403 and keeps running

#### Scenario: SSH tunnel
- **WHEN** a client on another machine forwards the server's socket with `ssh -N -L <local.sock>:<remote socket>` and runs `ship --server-url unix://<local.sock>`
- **THEN** its requests carry a loopback host and succeed
