# Tasks

These are checkpoints for implementing the locked plan, one slice at a time. Cyan locked the papers in `design/` on 2026-10-27, and they are the contract. `design/program.md` holds the file map, signatures and checks for each slice. Complete and verify one slice, then stop for Cyan's review before starting the next. Record surprises in the program paper's deviation log.

Verification uses real workflows and temporary probes, not permanent authored tests. For each slice, run these checks:
- `cargo fmt --all -- --check`;
- `cargo clippy --workspace --all-targets -- -D warnings`;
- debug and release builds;
- `openspec validate local-socket --type change --strict`.

Use owned temporary servers with `XDG_RUNTIME_DIR`, `XDG_STATE_HOME` and `XDG_CONFIG_HOME` pointed at temporary directories. Slice 0's TCP server uses a spare port in 43331 to 43337. Don't touch existing user work, including the server on 43179. Checks run on macOS. Linux and Windows count only when run there and are otherwise reported unverified.

## 0. Baseline latency on TCP

- [x] 0.1 On today's build, run the input and screen workloads from `docs/research/request-ordering-probe.md` and `docs/research/screen-cost.md` against an owned temporary server. If none of their probe scripts still run, stop and ask Cyan before writing new ones.
- [x] 0.2 Record the numbers in the program paper's deviation log.

## 1. The server listens on a named socket

- [x] 1.1 Add `crates/ship-core/src/endpoint.rs` with `ServerName`, `Endpoint` and the Unix `dir` module, export it from `lib.rs`, and add `nix` with `user` as a Unix dependency of ship-core.
- [x] 1.2 In `protocol.rs`, remove `DEFAULT_PORT` and `DEFAULT_SERVER_URL`, set `PROTOCOL_VERSION` to 19, and add `HealthResponse.socket`.
- [x] 1.3 Add `crates/ship-server/src/claim.rs` (Unix half): take `NAME.lock`, unlink a stale `NAME.sock`, bind a `UnixListener`, return `Conflict` when the name is held, and unlink the socket on drop.
- [x] 1.4 Make `serve` take an `Endpoint` and a `Claim`, removing `loopback_addr` and `tap_io`. Add `AppState.socket`, fill it in `health.rs`, and build `SHIP_SERVER_URL` from `endpoint.url()` in `PaneEnv::new`.
- [x] 1.5 Replace `ServerArgs.port` with `name: ServerName`, and key `Role::Server`, `role()`, `tabs/NAME.json` and `logs/NAME/` on the name.
- [x] 1.6 Verify slice 1's checks in `design/program.md`: `curl --unix-socket` health shows `socket` and a foreign `Host` gets 403, a second server with the name exits "already running", a `kill -9`ed server restarts without cleanup, the directory is `0700` and a `chmod 755` makes the server refuse, a pane sees `SHIP_SERVER_URL=unix:///…`, and `lsof -iTCP -sTCP:LISTEN` shows no Ship process.

## 2. Clients, launcher and target selection

- [x] 2.1 Add `Cli.server_url: Option<Endpoint>`, `Cli.server_name: Option<ServerName>`, `Target` and `Cli::target()` with FR-007's order, reading the two variables by hand. Refuse both flags together and `http(s)` URLs with the pointer message.
- [x] 2.2 Build the reqwest client over the socket (and pipe) with base `http://localhost/`, keeping the HTTP/2-then-HTTP/1.1 fallback for URL targets. Show the endpoint in errors in place of the origin.
- [x] 2.3 Replace `default_health` with `ensure_running(client, name, config)`, spawning `ship server --name NAME --starter --background-child`. Make `ship server` refuse the two target flags.
- [x] 2.4 In `ship-client`'s `http_error`, treat `NotFound` on connect as `ConnectionRefused`.
- [x] 2.5 Verify slice 2's checks: the local P1 and P2 product stories except the two-user one, ten concurrent `ship tab list` calls leave one server, `SHIP_SERVER_NAME=scratch` in a `default` pane still lists `default`, an `http://` URL gets the pointer, both flags are refused, and slice 0's workloads are compared. Cyan approved skipping SSH-forwarding verification and accepting the measured roughly 1 ms extra acknowledgment time for a 1 MiB paste; see `design/program.md`. The two-user story and Linux remain unverified.

## 3. Windows builds

- [ ] 3.1 Add the Windows `pipe` modules to `endpoint.rs` and `claim.rs`, with the owner-only `D:P(A;;GA;;;OW)` descriptor and the axum `Listener` adapter, and `interprocess` as a Windows dependency of ship-server.
- [ ] 3.2 Add Windows `detach` and `shutdown`, and the launcher's `DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP` creation flags.
- [ ] 3.3 Move the `colors.rs` terminal query into `#[cfg(unix)] mod query`, replacing the earlier probe's gates, and fall back to `HostColors::default()` in the two examples off Unix.
- [ ] 3.4 Allow `0BSD` in `deny.toml` and add the `continue-on-error` Windows entry to the CI matrix.
- [ ] 3.5 Verify slice 3's checks: `cargo check --workspace --all-targets --target x86_64-pc-windows-gnu` passes from macOS through the zig wrapper, `cargo deny check` passes, macOS clippy still passes, and after a push the Windows job's result shows on the pull request. Report Windows behavior unverified.

## 4. Docs and specs

- [ ] 4.1 Update `docs/site/troubleshooting.md`, `docs/site/saved-tabs.md` and `docs/site/windows-and-the-server.md`, following `docs/agents/docs.md`: ports become names and sockets, add the ssh-forwarding example, and drop the warning that other accounts can drive the server.
- [ ] 4.2 Check the deltas under `specs/` against what slices 1 to 3 built, and update them and `proposal.md` for any deviation.
- [ ] 4.3 Verify: `openspec validate local-socket --type change --strict` passes, `grep -rn 43179 docs/site openspec/specs` finds nothing once the specs are synced, and the size report gives implementation Rust and tests separately.
