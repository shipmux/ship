# Local socket

Status: Locked on 2026-10-27, approved by Cyan in Plannotator ("LGTM"), which accepted the recommendations for U-1 to U-4. Written the same day from a design conversation with Cyan about local-server security and remote access. During review Cyan also directed two edits in conversation: the target precedence in FR-007, and dropping the uid from the macOS socket path. The approval doesn't authorize implementation; Cyan requests that separately.

## Summary

Today the server listens on TCP `127.0.0.1:43179` with no login. Any program on the machine can drive it, including programs run by other user accounts, and two users on one machine collide on the port. `docs/site/windows-and-the-server.md` already warns about this.

This change moves the server onto a Unix socket in a directory only its user can open, the way tmux, Zellij, Emacs and Docker do. The operating system then decides who can connect, so Ship never needs tokens, passwords or certificates. HTTP stays the protocol; only the connection underneath changes.

Decided so far, in conversation:

- The server never listens on TCP. No `--port` and no `--host` mode. Remote access goes through ssh, and a browser on another device can go through `tailscale serve`, which can forward to a Unix socket. Neither needs code in this change.
- Servers are told apart by a name instead of a port. The name picks the socket file, the saved tabs and the logs, and defaults to `default`. All of a user's sockets sit in one directory, so listing it shows them.
- A crashed server's leftover socket never blocks a new one, and two `ship` commands starting at once still end with one server.
- Windows uses a named pipe restricted to the current user. Windows joins CI as a job that warns on failure instead of blocking.
- The existing checks that refuse web pages (`Origin`) and non-localhost addresses (`Host`) stay.

## Goal

Make a Ship server reachable only by the user who runs it, with no login to manage, so a shared machine is safe and two users can each run Ship. It serves Cyan first, and anyone running Ship on a machine with other accounts or untrusted local programs.

## Interface

```text
before                                       after
ship server [--port PORT]                    ship server [--name NAME]
logs/43179/, tabs/43179.json                 logs/NAME/, tabs/NAME.json
ship --server-url http://127.0.0.1:43179     ship --server-url unix:///run/user/1000/ship/default.sock
                                             ship --server-name NAME   or   SHIP_SERVER_NAME=NAME      (U-1)
ship server status   {"service":…,"log":…}   {"service":…,"log":…,"socket":"/run/user/1000/ship/default.sock"}   (U-3)
```

Where sockets live:

```text
Linux     $XDG_RUNTIME_DIR/ship/NAME.sock
macOS     $TMPDIR/ship/NAME.sock              (no runtime dir; $TMPDIR is already per-user)
other     /tmp/ship-UID/NAME.sock             (shared /tmp, so the uid keeps users apart)
Windows   \\.\pipe\ship-USER-NAME             (U-4 for the URL form)
```

The directory is readable only by its owner. Ship refuses to use one that is owned by someone else or open to others.

Using Ship on another machine, as the docs would show it:

```sh
ssh box -t ship
# or attach from here through a forwarded socket
ssh -N -L ~/.ship-box.sock:/run/user/1000/ship/default.sock box
ship --server-url unix://$HOME/.ship-box.sock
```

## User stories

### P1: Only I can reach my server

**Given** my server is running, **when** another user account on the same machine tries to connect to it, **then** the connection is refused by the operating system.

**Given** two user accounts on one machine, **when** both run `ship`, **then** each gets their own server and tabs, with no conflict.

Independent verification: on Linux, run `ship` as two users and check each sees only their own tabs, and that the second user can't open the first user's socket directory.

### P1: Ship works as before

**Given** no server is running, **when** I run `ship`, **then** a server starts on the `default` socket, restores `tabs/default.json`, and the client opens, as it does today on TCP.

**Given** a pane in my server, **when** I run `ship tab create` inside it, **then** it reaches the server that owns the pane.

Independent verification: stop the server, run `ship`, check `ship server status` shows the socket path, and create a tab from inside a pane.

### P1: A crash doesn't leave me stuck

**Given** my server was killed with SIGKILL and its socket file is still there, **when** I run `ship`, **then** a new server starts on the same name without me deleting anything.

**Given** no server is running, **when** ten `ship` commands start at once, **then** exactly one server is left running, as today.

Independent verification: `kill -9` the server, run `ship`, and repeat the existing concurrent-launch check.

### P2: Several servers side by side

**Given** my `default` server is running, **when** I run `ship server --name scratch`, **then** a second server runs on `scratch.sock` with its own saved tabs and logs, and `ship --server-name scratch` opens it (U-1).

**Given** `SHIP_SERVER_NAME=scratch` exported in my shell config, **when** I run `ship tab list` in a pane of my `default` server, **then** it lists `default`'s tabs, and `ship --server-name scratch tab list` in the same pane lists `scratch`'s.

**Given** a server named `scratch` is already running, **when** I start another `ship server --name scratch`, **then** it refuses and says the server is already running.

Independent verification: run two named servers, create different tabs in each, restart both, and check each restores its own.

### P2: Reach a server on another machine

**Given** Ship running on `box`, **when** I forward its socket with `ssh -L` and point `--server-url` at the local end, **then** the client works as if the server were local.

Independent verification: forward a socket over ssh to localhost and run the client through it.

### P3: Windows builds

**Given** the CI matrix, **when** a change is pushed, **then** a Windows job builds and tests Ship and reports failures without blocking the merge.

## Non-goals

- Any TCP listener: `--port`, `--host`, tokens, passwords, TLS or certificates.
- `ssh://` server URLs and a `dial-stdio` bridge. A later change, milestone 7 at the earliest.
- Browser clients and `tailscale serve` support. The `Origin` and `Host` checks stay as they are, so a proxy in front of the socket is still refused.
- `ship server list`. The socket directory can be listed by hand.
- Moving `tabs/43179.json` or `logs/43179/` to the new names. Ship has one user; Cyan renames the file once.
- Unix sockets on Windows. Named pipes instead.
- Confirming Windows behavior. CI shows whether it builds and tests pass; using it is reported as unverified.
- Windows release binaries.

## Functional requirements

- **FR-001:** The server MUST listen only on a Unix socket (Windows: a named pipe). It MUST NOT listen on any TCP port.
- **FR-002:** The socket MUST live in a per-user directory, chosen as in the Interface section, that only its owner can open. The server MUST refuse to start when that directory is owned by another user or accessible to others. On Windows, the pipe MUST accept only the current user.
- **FR-003:** A server MUST have a name, `default` unless `ship server --name` gives another. The name MUST be limited to plain characters (letters, digits, `-`, `_`) and MUST pick the socket file, the saved-tabs file and the log directory.
- **FR-004:** Starting a server whose name is already running MUST fail with a message saying so. A socket left behind by a server that died MUST NOT prevent a new one from starting.
- **FR-005:** Bare `ship` and the tab and pane commands without an explicit target MUST use the `default` server and start it when it isn't running, keeping today's launch rules, including at most one server from concurrent launches.
- **FR-006:** `--server-url` and `SHIP_SERVER_URL` MUST accept `unix:///absolute/path` (Windows: the pipe form, U-4) and connect only there, never starting a server. They MUST reject `http://` and `https://` with a message pointing at the socket form (U-2).
- **FR-007:** `--server-name` and `SHIP_SERVER_NAME` MUST select a server by name the way the default is selected (U-1). The target MUST be chosen in this order, the first one set winning: `--server-url`, `--server-name`, `SHIP_SERVER_URL`, `SHIP_SERVER_NAME`, then `default`. Passing both `--server-url` and `--server-name` MUST be an error; any other combination resolves by this order, so a `SHIP_SERVER_NAME` exported in a shell config doesn't redirect commands run in a pane, whose `SHIP_SERVER_URL` names its own server.
- **FR-008:** Every pane MUST receive `SHIP_SERVER_URL` naming its own server's socket.
- **FR-009:** `ship server status` MUST report the socket path (U-3).
- **FR-010:** Requests carrying an `Origin` header, or addressed to a host other than localhost, MUST still be refused as today.
- **FR-011:** CI MUST build and test on Windows, and a Windows failure MUST NOT fail the workflow. Linux and macOS stay required.
- **FR-012:** `docs/site/` MUST describe the socket, names and ssh forwarding in place of ports, and MUST drop the warning that other accounts can drive the server. The specs that name ports (health-exchange, diagnostics, session-restore, local-foundation) MUST be updated.

## Success criteria

- **SC-001:** On Linux and macOS, every P1 and P2 story's verification passes.
- **SC-002:** No Ship process listens on a TCP port: `lsof -iTCP -sTCP:LISTEN` shows nothing from Ship while servers run.
- **SC-003:** Typing, screen updates and attaching feel at least as fast as on TCP, and a before/after measurement on the existing input and screen workloads shows no regression on macOS. Linux is reported unverified unless measured there.
- **SC-004:** The Windows CI job runs and its result is visible on pull requests. Windows behavior is reported as unverified.
- **SC-005:** The size report gives implementation Rust and tests separately.
- **SC-006:** Checks not run on a platform are reported as unverified there.

Verification is through real workflows and temporary probes; this paper does not authorize permanent test code.

## Open unknowns

None. All four were resolved on 2026-10-27 by Cyan's approval, which took the recommendation each time:

- **U-1: Select servers by name from the client.** Yes: `--server-name` and `SHIP_SERVER_NAME` now, so reaching a named server doesn't mean typing its socket path.
- **U-2: Drop `http://` and `https://` from `--server-url`.** Yes. Once no server listens on TCP, nothing can answer them. `ssh://` comes back as its own change.
- **U-3: How to find the socket path.** A `socket` field in `ship server status`, next to `log`, not a separate `ship server socket` command.
- **U-4: URL form for Windows pipes.** Docker's `npipe:////./pipe/ship-USER-NAME`, the only established form.
