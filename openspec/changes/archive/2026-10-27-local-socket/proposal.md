# Proposal

This is the planning artifact for a locked plan. Cyan locked the three papers in `design/` on 2026-10-27. The papers are the source of truth; this proposal and the deltas under `specs/` are views of them. Approval does not start implementation.

## Why

The server listens on TCP `127.0.0.1:43179` with no login, so any program on the machine can drive it, including programs run by other user accounts, and two users on one machine collide on the port. Moving the server onto a Unix socket in a directory only its user can open lets the operating system decide who connects, the way tmux, Zellij, Emacs and Docker do, with no tokens or passwords to manage.

## What Changes

- **BREAKING: no TCP listener.** The server listens only on a Unix socket, or a named pipe on Windows. `ship server --port` is gone.
- **Server names.** `ship server --name NAME` replaces the port, `default` unless given. The name picks the socket, `tabs/NAME.json` and `logs/NAME/`.
- **Private sockets.** Sockets live in a per-user directory: `$XDG_RUNTIME_DIR/ship/` on Linux, `$TMPDIR/ship/` on macOS, `/tmp/ship-UID/` elsewhere. The directory is `0700`, and Ship refuses one owned by another user or open to others. Windows pipes accept only the current user.
- **One server per name.** A second server with a running name fails saying so. A socket left by a killed server doesn't block a new one, and concurrent launches still leave one server.
- **BREAKING: socket URLs.** `--server-url` and `SHIP_SERVER_URL` take `unix:///path` (Windows: `npipe:////./pipe/...`) and refuse `http://` and `https://` with a pointer to the socket form.
- **Select by name.** `--server-name` and `SHIP_SERVER_NAME` pick a named server, starting it like the default. Order: `--server-url`, `--server-name`, `SHIP_SERVER_URL`, `SHIP_SERVER_NAME`, `default`. Both flags together are an error.
- **Panes** get `SHIP_SERVER_URL` naming their server's socket.
- **`ship server status`** reports `socket`.
- **BREAKING: protocol 19.** Old clients are refused by the health check, and clients from before this change can't reach the socket at all.
- **Windows CI.** A Windows job builds and tests and may fail without failing the workflow.
- Unchanged: HTTP, the `Origin` and `Host` checks, and the HTTP/1.1 fallback for explicit targets.

The specs drop what only made sense for HTTP URLs: TLS verification, credential redaction and base paths in targets. The protocol-7 legacy scenario now says those clients can't reach the server at all, since they speak only TCP.

Not in this change (the product paper lists them all):
- Any TCP listener, tokens, passwords or TLS.
- `ssh://` URLs, browser clients and `tailscale serve` support.
- `ship server list`.
- Moving `tabs/43179.json` and `logs/43179/` to the new names. Cyan renames the file once.
- Confirming Windows behavior and Windows release binaries.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `health-exchange`: names, socket location and privacy, one server per name, target selection, pane `SHIP_SERVER_URL`, `socket` in health, protocol 19, and port wording in the startup, starter and local-request requirements.
- `diagnostics`: server logs live under `logs/NAME/`.
- `session-restore`: saved tabs live in `tabs/NAME.json`, with `NAME.json.prev` beside it.
- `local-foundation`: help names `--name` and `--server-name`, the Windows CI job, and the guide's socket and ssh-forwarding docs.

## Impact

- `ship-core`: new `endpoint.rs` (`ServerName`, `Endpoint`), `PROTOCOL_VERSION` 19, `HealthResponse.socket`, the port constants removed, `nix` with `user` on Unix.
- `ship-server`: new `claim.rs` (lock, bind, Windows pipe listener), `serve` takes an `Endpoint`, `PaneEnv` builds the socket URL, health fills `socket`. `interprocess` on Windows only.
- `ship`: target selection in `cli.rs`, the reqwest client over the socket or pipe, the launcher keyed by name, Windows `detach` and `shutdown`.
- `ship-client`: a missing socket counts as no server running.
- `vendor/ratatui-ghostty`: the terminal color query gated to Unix.
- `deny.toml` allows `0BSD`. CI gains the Windows job.
- Docs: `troubleshooting.md`, `saved-tabs.md` and `windows-and-the-server.md` in `docs/site/`.
- Measured line counts go in [tasks.md](tasks.md).
