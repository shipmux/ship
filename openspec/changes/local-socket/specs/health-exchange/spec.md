## ADDED Requirements

### Requirement: Server names
A server SHALL have a name, `default` unless `ship server --name NAME` gives another. A name SHALL be 1 to 32 bytes of ASCII letters, digits, `-` and `_`. The name SHALL pick the server's socket, its saved-tabs file and its log directory. Any other name SHALL be an argument error.

#### Scenario: Named server
- **WHEN** a user runs `ship server --name scratch`
- **THEN** it listens on `scratch.sock` in the socket directory, saves to `tabs/scratch.json` and logs to `logs/scratch/`

#### Scenario: Invalid name
- **WHEN** a user runs `ship server --name ../x` or `ship --server-name "a b" tab list`
- **THEN** the argument is rejected and nothing starts

### Requirement: Socket location
The server SHALL listen only on a Unix socket, or on Windows a named pipe, and SHALL NOT listen on any TCP port. A named server's socket SHALL be `NAME.sock` in `$XDG_RUNTIME_DIR/ship/` when that variable is set, else `$TMPDIR/ship/` on macOS, else `/tmp/ship-UID/`. On Windows the pipe SHALL be `\\.\pipe\ship-USER-NAME`.

#### Scenario: No TCP port
- **WHEN** servers run under two names
- **THEN** `lsof -iTCP -sTCP:LISTEN` lists no Ship process

#### Scenario: Socket on macOS
- **WHEN** `XDG_RUNTIME_DIR` is unset on macOS and a user runs `ship server --name scratch`
- **THEN** the server listens on `$TMPDIR/ship/scratch.sock`

### Requirement: Private socket directory
Ship SHALL create the socket directory with mode `0700` when it's missing. The server SHALL refuse to start, and a client selecting a server by name SHALL refuse to connect, when the directory is owned by another user or grants any group or other permission. On Windows the pipe SHALL accept only the current user.

#### Scenario: Private directory
- **WHEN** a server has started
- **THEN** its socket directory is mode `0700` and owned by the server's user

#### Scenario: Directory open to others
- **WHEN** the socket directory is `chmod 755` and a user runs `ship server`
- **THEN** the server refuses to start with an error naming the directory

#### Scenario: Another user
- **WHEN** another user account on the same machine tries to connect to a user's server
- **THEN** the operating system refuses the connection

#### Scenario: Two users
- **WHEN** two user accounts on one machine each run `ship`
- **THEN** each gets their own `default` server and tabs, with no conflict

### Requirement: One server per name
A server SHALL own its name for its whole life. Starting a server whose name is held by a running server SHALL fail with an error saying a server is already running at that socket. A socket file left by a server that died, including by SIGKILL, SHALL NOT prevent a new server from starting under that name.

#### Scenario: Name already running
- **WHEN** a server named `scratch` is running and a user runs `ship server --name scratch`
- **THEN** the second server exits 1 saying a server is already running, and the first keeps serving

#### Scenario: Crashed server
- **WHEN** a server is killed with SIGKILL, its socket file remains, and a user runs `ship`
- **THEN** a new server starts on the same name without the user deleting anything

### Requirement: Target selection
A client command SHALL pick its target from the first of these that is set: `--server-url`, `--server-name`, `SHIP_SERVER_URL`, `SHIP_SERVER_NAME`, then the name `default`. Passing both `--server-url` and `--server-name` SHALL be an argument error; any other combination SHALL resolve by this order. A target given by name SHALL be found and started like the default.

#### Scenario: Server by name
- **WHEN** servers `default` and `scratch` are running and a user runs `ship --server-name scratch tab list`
- **THEN** it lists `scratch`'s tabs

#### Scenario: Exported name inside a pane
- **WHEN** `SHIP_SERVER_NAME=scratch` is exported in a shell config and a user runs `ship tab list` in a pane of the `default` server
- **THEN** it lists `default`'s tabs, because the pane's `SHIP_SERVER_URL` comes first

#### Scenario: Both flags
- **WHEN** a user runs `ship --server-url unix:///tmp/x.sock --server-name scratch tab list`
- **THEN** the argument parser rejects the combination and exits 2

### Requirement: Server URLs
A server URL, from `--server-url` or `SHIP_SERVER_URL`, SHALL be `unix://` followed by an absolute path, or on Windows `npipe:` followed by a pipe path such as `npipe:////./pipe/ship-USER-NAME`. A client SHALL connect only there and never start a server for it. An `http://` or `https://` URL SHALL be rejected with a message pointing at the socket form.

#### Scenario: Socket URL
- **WHEN** a server named `scratch` is running and a user runs `ship --server-url unix://<dir>/scratch.sock tab list`
- **THEN** it lists `scratch`'s tabs

#### Scenario: HTTP URL
- **WHEN** a user runs `ship --server-url http://127.0.0.1:1 tab list`
- **THEN** it fails with a message pointing at the `unix://` form and starts nothing

### Requirement: Pane server URL
Every pane SHALL receive `SHIP_SERVER_URL` set to the URL of its own server's socket or pipe.

#### Scenario: Command from a pane
- **WHEN** a user runs `ship tab create` inside a pane of a server named `scratch`
- **THEN** the tab is created on `scratch`

### Requirement: Explicit server and URL target
`ship server [--name NAME] [--starter] [--no-restore]` SHALL run a foreground server listening only on NAME's socket. A `--server-url URL` SHALL use only that socket without automatic local launch or fallback. Running a server with `--server-url` or `--server-name` given as flags SHALL be rejected with a pointer to `--name`; `ship server stop`, `ship server status` and `ship server log` accept them.

#### Scenario: Explicit named server
- **WHEN** a user runs `ship server --name scratch`, then `ship --server-url unix://<dir>/scratch.sock server status` and `ship --server-name scratch server status`
- **THEN** health succeeds against that server both times without launching a default server

#### Scenario: Explicit unreachable target
- **WHEN** the user supplies a URL whose socket doesn't exist or refuses connections, including the default server's socket
- **THEN** the client fails naming the socket and does not start or switch to a local server

#### Scenario: Invalid command combination
- **WHEN** the user runs `ship server` with `--server-url` or `--server-name`
- **THEN** the invocation is rejected rather than silently choosing a mode

### Requirement: Redirects refused
Health requests SHALL refuse redirects. Diagnostics SHALL identify a target by its socket path. Results SHALL remain separate from diagnostics.

#### Scenario: Redirect response
- **WHEN** a target redirects a health request
- **THEN** the client reports request failure without following the redirect

## REMOVED Requirements

### Requirement: Explicit server and custom target
**Reason**: The server no longer takes a port, and URL targets are sockets, not HTTP addresses.
**Migration**: Replaced by "Explicit server and URL target" above; `ship server --name NAME` replaces `--port PORT`.

### Requirement: Target integrity and diagnostic privacy
**Reason**: `http://` and `https://` targets are refused, so TLS verification, credentials in URLs and base paths no longer apply.
**Migration**: Redirect refusal and target diagnostics carry over to "Redirects refused" above.

## MODIFIED Requirements

### Requirement: Health contract and result
A compatible server SHALL answer GET `/health` with HTTP 200 and JSON containing `service` equal to `ship`, `protocol_version` equal to 19, informational string `version`, `log`, the path of the log part the server is writing, and `socket`, the path of the socket or pipe the server listens on. Health response JSON field names SHALL use snake case. `ship server status` SHALL print one health JSON line on stdout only on success, and SHALL never start a server.

#### Scenario: Compatible health
- **WHEN** `ship server status` contacts a compatible server
- **THEN** it prints the health response as one JSON line, including the server's current log part in `log` and its socket path in `socket`, and exits 0

#### Scenario: Log path follows rollover
- **WHEN** the server's log has moved to a new part and a user runs `ship server status`
- **THEN** `log` names the new part

#### Scenario: Status with no server
- **WHEN** no server is running and a user runs `ship server status`
- **THEN** it reports that no server is running, exits 1 and starts nothing

### Requirement: Health check before other requests
The current client SHALL validate service/protocol compatibility before any other request, including ordinary commands with `--server-url`, `--server-name`, `SHIP_SERVER_URL` or `SHIP_SERVER_NAME`, and `server stop`. URL targets, status and stop SHALL NOT start a server. Endpoint methods SHALL remain policy-free; application dispatch owns this validation.

#### Scenario: Incompatible or malformed health
- **WHEN** the target returns the wrong status, invalid JSON, wrong service or incompatible protocol version
- **THEN** the client reports contextual failure, emits no success result and exits 1

#### Scenario: Client from an earlier protocol
- **WHEN** a client built for protocol version 1 to 18 validates health from a server reporting protocol version 19
- **THEN** the client reports the incompatible protocol version and exits 1 without attaching

#### Scenario: Already-built protocol-7 explicit ordinary command
- **WHEN** an already-built protocol-7 client runs an ordinary command with an explicit URL against protocol 19
- **THEN** it can't reach the server, because it connects only over TCP, which the server no longer listens on

#### Scenario: Stop preflight
- **WHEN** the current client runs `ship server stop` against incompatible, malformed or wrong-service health
- **THEN** it fails before requesting shutdown and starts nothing

#### Scenario: New client against old server
- **WHEN** a version-11 client contacts a version-10 server
- **THEN** it reports incompatibility before attempting any other request or attach and does not replace the running server

### Requirement: Default-local startup and reuse
A client command whose target is a name, including bare `ship` and the tab and pane commands with no target set, SHALL connect to that name's socket, reuse compatible health, and launch a background server for that name with the starter only when the connection is refused or the socket file doesn't exist. That server SHALL restore the name's saved tabs. They SHALL NOT replace or kill an occupied, incompatible or unhealthy server.

#### Scenario: Absent default server
- **WHEN** the default server isn't running, nothing is saved for `default`, and the user runs `ship`
- **THEN** Ship launches a local background server with the starter, waits for compatible readiness and opens the client on the starter tab

#### Scenario: Absent named server
- **WHEN** no server named `scratch` is running and the user runs `ship --server-name scratch`
- **THEN** Ship launches a background server named `scratch` and opens the client on it

#### Scenario: Repeated launches
- **WHEN** the default server is stopped and started through bare `ship` ten times
- **THEN** the number of top-level tabs is the same as before the first start

#### Scenario: Existing compatible server
- **WHEN** compatible health is already available on the default socket
- **THEN** `ship` reuses it without starting another server or creating a tab

#### Scenario: Occupied or uncertain endpoint
- **WHEN** the default socket times out, fails other than refusal or a missing file, or responds incompatibly
- **THEN** `ship` fails without spawning a replacement or terminating the server

### Requirement: First selection after launch
When bare `ship` launched the server, its client SHALL open on the first top-level tab's first pane, whether that tab was restored or created by the starter. A client attaching to a server that was already running SHALL start with nothing selected.

#### Scenario: Absent default server with saved tabs
- **WHEN** the default server isn't running, `default` has saved tabs, and the user runs `ship`
- **THEN** Ship launches a local background server, which restores the saved tabs without adding a starter tab, and opens the client on the first top-level tab's first pane

#### Scenario: Attach to a running server
- **WHEN** a server is already running and a user runs `ship`
- **THEN** the client starts with nothing selected

### Requirement: Persistent daemon and bounded launch cleanup
An automatically launched ready server SHALL remain reachable after client exit and launching-terminal closure. Readiness SHALL have a five-second absolute deadline; ordinary connects and health requests SHALL be bounded by one and two seconds respectively. Failed launch cleanup SHALL affect only that invocation's child. Concurrent invocations SHALL leave at most one serving daemon and no surviving losing children.

#### Scenario: Client and terminal exit
- **WHEN** health succeeds after automatic launch and the client exits and its launching terminal closes
- **THEN** the daemon remains reachable by a later invocation

#### Scenario: Failed startup
- **WHEN** log creation, child initialization or readiness fails
- **THEN** Ship reports actionable context, cleans up its own still-live launch attempt, and does not claim readiness or kill another server

#### Scenario: Concurrent launch race
- **WHEN** ten clients run concurrently with the default server initially absent
- **THEN** at most one compatible daemon remains, losing children exit, and each client succeeds or fails with bounded contextual diagnostics

### Requirement: Observable startup and private daemon logs
Server diagnostics SHALL identify the startup socket, PID and protocol version, and, at debug detail, health-request method/path, status and duration. Automatically launched servers SHALL write to that run's private mode-0600 file under the name's log directory, as the `diagnostics` capability describes, and no temporary log SHALL be created. Launch diagnostics SHALL expose the run file's path and PID; log-creation failures SHALL name the attempted directory without claiming a file exists.

#### Scenario: Observe request outcome
- **WHEN** a health request completes with Ship debug logging enabled
- **THEN** server diagnostics show GET `/health`, status and duration without obscuring client stdout or exposing arbitrary bodies/headers

#### Scenario: Private launch log
- **WHEN** automatic launch succeeds
- **THEN** diagnostics identify the daemon PID and its run file under `logs/<name>/`, which remains available after client exit and is linked from `current.log`

### Requirement: Server starter
`ship server --starter` SHALL create one top-level tab holding one pane running the configured shell, else the login shell, in the server user's home directory before it accepts any connection, only when the server has no tabs after restoring. That includes a first run with no saved file, saved tabs that held no tabs, `--no-restore`, and an unreadable saved file. Its help SHALL say so. The starter tab SHALL be saved like any other change.

#### Scenario: Starter after a restore
- **WHEN** the server's name has saved tabs with two top-level tabs and a user runs `ship server --starter`
- **THEN** `ship tab list` shows the two restored tabs and no starter tab

#### Scenario: Starter with --no-restore
- **WHEN** the server's name has saved tabs and a user runs `ship server --starter --no-restore`
- **THEN** `ship tab list` shows one tab with one pane, and the new tab schedules a save 5 seconds later; after that save completes, the saved file holds that tab and `<name>.json.prev` holds the earlier saved tabs

### Requirement: Tabs a server creates itself
`ship server` without `--starter` SHALL start with no tabs when there is nothing to restore. A server SHALL create tabs on its own only at startup. If the starter's program cannot start, the server SHALL exit with that error instead of serving.

#### Scenario: With and without the starter
- **WHEN** nothing is saved for the server's name and a user runs `ship server`, and separately `ship server --starter`
- **THEN** `ship tab list` prints `{}` for the first and one tab with one pane for the second

#### Scenario: Broken starter
- **WHEN** nothing is saved for the server's name and a user runs `HOME=/nonexistent ship server --starter`
- **THEN** the server exits 1 with an error naming what failed and never accepts a connection

### Requirement: Local requests only
The server SHALL answer a request only when its host, from the HTTP/2 authority or else the HTTP/1 `Host` header, is `localhost`, an IPv4 address in `127.0.0.0/8` or `[::1]`, with or without a port, and it carries no `Origin` header. Any other request, including one with no host, SHALL get status 403 with an error body whose `.type.kind` is `forbidden`, and SHALL NOT reach a route. This applies to every route, `/health` included. The client SHALL address the server as `localhost`.

#### Scenario: Ship client
- **WHEN** a user runs `ship server status` against the default server
- **THEN** health succeeds as before

#### Scenario: DNS rebinding
- **WHEN** a request over the server's socket carries `Host: attacker.example`
- **THEN** the server responds 403 with `.type.kind` equal to `forbidden` and the route doesn't run

#### Scenario: Browser request to the loopback address
- **WHEN** a request to `/api/v0/server/stop` carries `Host: localhost` and `Origin: https://attacker.example`
- **THEN** the server responds 403 and keeps running

#### Scenario: SSH tunnel
- **WHEN** a client on another machine forwards the server's socket with `ssh -N -L <local.sock>:<remote socket>` and runs `ship --server-url unix://<local.sock>`
- **THEN** its requests carry a loopback host and succeed

### Requirement: HTTP/2 transport
The client SHALL speak HTTP/2 to the server without upgrade negotiation, and the server SHALL accept both that and HTTP/1.1 on its one listener. When the health check of a URL target fails over HTTP/2 for a reason other than a refused connection, a missing socket or a timeout, the client SHALL retry it once over HTTP/1.1. If that succeeds, the client SHALL log a warning naming the target and use HTTP/1.1 for the rest of the run. A server selected by name, launched or found, SHALL always use HTTP/2.

#### Scenario: Local server
- **WHEN** a user runs `ship` against the local server
- **THEN** the server's debug log shows the client's requests arriving over HTTP/2

#### Scenario: HTTP/1.1-only proxy
- **WHEN** a user runs `ship --server-url` pointing at a socket served by a proxy that speaks only HTTP/1.1 in front of a server
- **THEN** the client logs a warning that it fell back to HTTP/1.1, attaches and works

#### Scenario: Nothing listening
- **WHEN** a user runs `ship server status --server-url` at a socket path where nothing listens
- **THEN** it reports that no server is running once, without trying HTTP/1.1
