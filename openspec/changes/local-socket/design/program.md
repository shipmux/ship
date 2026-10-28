# Local socket: program

Status: Locked on 2026-10-27, approved by Cyan in Plannotator ("LGTM"), which accepted the recommendation for O-1. Built on the locked `product.md` and `architecture.md`. A document-only plan: none of the files below exist yet, and none of the code has been compiled. The approval doesn't authorize implementation; Cyan requests that separately.

## Summary

The work comes in five slices. Slice 0 measures latency on today's TCP build. Slice 1 moves the server onto the socket. Slice 2 moves the client and the launcher. Slice 3 makes Windows build. Slice 4 updates the docs and specs. The only new file in ship-core is `endpoint.rs`, which holds server names and socket paths. The only new file in ship-server is `claim.rs`, which takes the name and binds. Everything else replaces port-shaped code where it already lives.

Needs Cyan's attention:

- **O-1 (resolved):** cargo-deny would refuse a Windows-only dependency, `doctest-file`, because its only license is 0BSD. Cyan's approval allows `0BSD` in `deny.toml`.
- **One correction to the architecture.** axum already implements its `Listener` trait for tokio's `UnixListener`, so the adapter is needed on Windows only. The shape doesn't change. See the deviation log.

## Rationale

Each box in the architecture's Structure table maps to one place:

| Box | Where |
| --- | --- |
| Endpoint | new `crates/ship-core/src/endpoint.rs` |
| target selection | `crates/ship/src/cli.rs` |
| connector | `crates/ship/src/main.rs` `client()`, plus `NotFound` in `crates/ship-client/src/lib.rs` `http_error` |
| launcher | `crates/ship/src/local.rs` |
| claim and listener adapter | new `crates/ship-server/src/claim.rs`, with the Windows pipe adapter in it |
| PaneEnv | `crates/ship-server/src/pane.rs` |
| platform shims | `crates/ship/src/main.rs`, `crates/ship/src/diagnostics.rs`, `crates/ship/src/local.rs`, `vendor/ratatui-ghostty/src/colors.rs` |

Names and paths are worked out once, in `Endpoint`. The server, the client and the pane environment all take an `Endpoint` and never build a path themselves. Platform differences stay in two gated modules: `endpoint::dir` and `endpoint::pipe` for paths, and `claim`'s Unix and Windows halves for binding.

## Skeleton map

### Added

**`crates/ship-core/src/endpoint.rs`** (slice 1)

```rust
//! Where a server listens and the names that pick it.

use std::{fmt, path::{Path, PathBuf}, str::FromStr};
use serde::{Deserialize, Serialize};
use crate::prelude::*;

/// Letters, digits, `-` and `_`, 1 to 32 bytes. The cap keeps
/// `$TMPDIR/ship/NAME.sock` inside macOS's 104-byte socket path.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ServerName(String);

impl ServerName {
    pub fn as_str(&self) -> &str;
}
/// `default`.
impl Default for ServerName;
impl FromStr for ServerName { type Err = AppError; }
impl TryFrom<String> for ServerName { type Error = AppError; }
impl From<ServerName> for String;
impl fmt::Display for ServerName;

/// A Unix socket path, or a named pipe path on Windows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Endpoint(PathBuf);

impl Endpoint {
    /// The current user's socket or pipe for `name`. On Unix this creates the
    /// directory `0700` if it's missing, and refuses one that is owned by
    /// someone else or open to group or others. The client checks this too.
    pub fn named(name: &ServerName) -> Result<Self>;
    pub fn path(&self) -> &Path;
    /// `unix:///abs/path`, or `npipe:////./pipe/ship-USER-NAME` on Windows.
    pub fn url(&self) -> String;
    /// `NAME.lock` beside `NAME.sock`. Never deleted.
    #[cfg(unix)]
    pub fn lock_path(&self) -> PathBuf;
}

/// Parses `unix:` URLs (`npipe:` on Windows) with an absolute path. `http:`
/// and `https:` get an error that points at the socket form (FR-006).
impl FromStr for Endpoint { type Err = AppError; }
impl fmt::Display for Endpoint;

#[cfg(unix)]
mod dir {
    /// `$XDG_RUNTIME_DIR/ship`, else `$TMPDIR/ship` on macOS, else `/tmp/ship-UID`.
    pub(super) fn socket_dir() -> Result<PathBuf>;
    /// Create `0700` if missing; refuse another owner or any group or other bits.
    pub(super) fn private(dir: &Path) -> Result<()>;
}

#[cfg(windows)]
mod pipe {
    /// `\\.\pipe\ship-USER-NAME`, with USER from `USERNAME`.
    pub(super) fn path(name: &ServerName) -> Result<PathBuf>;
}
```

**`crates/ship-server/src/claim.rs`** (slice 1; Windows half in slice 3)

```rust
//! Owning a server name for the life of the process (architecture K-1, K-2).

use ship_core::{endpoint::Endpoint, prelude::*};

#[cfg(unix)]
pub(crate) type Listener = tokio::net::UnixListener; // axum implements Listener for it
#[cfg(windows)]
pub(crate) type Listener = pipe::PipeListener;

pub(crate) struct Claim {
    endpoint: Endpoint,
    /// Held until the process exits; the kernel releases it on any death.
    #[cfg(unix)]
    lock: std::fs::File,
}

impl Claim {
    /// Unix: take `NAME.lock` with `File::try_lock`, unlink any leftover
    /// `NAME.sock`, bind. Windows: create the first pipe instance. A live
    /// server holding the name gives `Conflict`: "a server is already running at …".
    pub(crate) fn take(endpoint: Endpoint) -> Result<(Self, Listener)>;
    pub(crate) fn endpoint(&self) -> &Endpoint;
}

/// Unix: unlink `NAME.sock` on clean shutdown, before the lock field drops.
impl Drop for Claim;

#[cfg(windows)]
mod pipe {
    use interprocess::local_socket::{ListenerOptions, tokio::{Listener as Pipes, Stream}};
    use interprocess::os::windows::{local_socket::ListenerOptionsExt, security_descriptor::SecurityDescriptor};

    /// Owner-only descriptor `D:P(A;;GA;;;OW)`; interprocess opens the next
    /// instance before handing off the connected one.
    pub(crate) struct PipeListener(Pipes);

    pub(super) fn create(endpoint: &Endpoint) -> Result<PipeListener>;

    impl axum::serve::Listener for PipeListener {
        type Io = Stream;
        type Addr = ();
        async fn accept(&mut self) -> (Stream, ());
        fn local_addr(&self) -> std::io::Result<()>;
    }
}
```

### Moved

None.

### Replaced

**`crates/ship-core/src/protocol.rs`** (slice 1)

```diff
-pub const DEFAULT_PORT: u16 = 43179;
-pub const DEFAULT_SERVER_URL: &str = "http://127.0.0.1:43179";
-pub const PROTOCOL_VERSION: u32 = 18;
+pub const PROTOCOL_VERSION: u32 = 19;
 pub struct HealthResponse {
     pub service: String,
     pub protocol_version: u32,
     pub version: String,
     pub log: String,
+    /// The socket or pipe the server listens on (FR-009).
+    pub socket: String,
 }
```

`crates/ship-core/src/lib.rs` drops the two constants from its re-exports and adds `pub mod endpoint;`.

**`crates/ship-server/src/lib.rs`** (slice 1)

```diff
-pub fn loopback_addr(port: u16) -> Result<SocketAddr>;
 pub async fn serve(
-    address: SocketAddr,
+    endpoint: Endpoint,
     starter: bool, config: PathBuf, save: PathBuf, restore: bool,
     shutdown: impl Future<Output = Result<()>> + Send + 'static,
     log: impl Fn(Option<LogChange>) -> Result<LogFilter> + Send + Sync + 'static,
 ) -> Result<()>;
```

Inside `serve`, `Claim::take(endpoint)` replaces `TcpListener::bind` and the `tap_io`/`TCP_NODELAY` step. The claim lives as long as `serve`. `AppState` gains `socket: Arc<str>` for `/health`. The bind-before-restore and accept-after-starter order stays the same.

**`crates/ship-server/src/health.rs`** (slice 1): fills `socket` from `AppState`.

**`crates/ship-server/src/pane.rs`** (slice 1)

```diff
-pub fn new(address: SocketAddr, config: PathBuf) -> Result<Self>;
+pub fn new(endpoint: &Endpoint, config: PathBuf) -> Result<Self>;   // server_url = endpoint.url()
```

**`crates/ship/src/cli.rs`** (slice 1 for `ServerArgs`, slice 2 for `Cli`)

```rust
pub struct Cli {
    /// Connect only to this socket, unix:///path; never start a server.
    /// Also read from SHIP_SERVER_URL.
    #[arg(long, global = true, value_name = "URL", conflicts_with = "server_name")]
    pub server_url: Option<Endpoint>,
    /// Use the server with this name, starting it if it isn't running.
    /// Also read from SHIP_SERVER_NAME.
    #[arg(long, global = true, value_name = "NAME")]
    pub server_name: Option<ServerName>,
    // config, command unchanged
}

/// What a client command talks to.
pub enum Target {
    /// May be started if it isn't running.
    Named(ServerName),
    /// Never started.
    Url(Endpoint),
}

impl Cli {
    /// FR-007: --server-url, --server-name, SHIP_SERVER_URL, SHIP_SERVER_NAME,
    /// then `default`. The variables are read here, not through clap's `env`,
    /// so an exported variable can never conflict with a flag.
    pub fn target(&self) -> Result<Target>;
}

impl Target {
    pub fn endpoint(&self) -> Result<Endpoint>;
}

pub struct ServerArgs {
    pub command: Option<ServerCommand>,
    /// Which server: picks its socket, saved tabs and logs.
    #[arg(long, default_value_t)]
    pub name: ServerName,          // replaces `port: u16`
    // starter, no_restore, background_child, log_file unchanged
}
```

**`crates/ship/src/main.rs`** (slices 1 and 2; Windows shims in slice 3)

```diff
-fn main()  // reads value_source("server_url")
+fn main()  // no value source; Cli::target() decides
-async fn dispatch(cli, source: Option<ValueSource>, diagnostics)
+async fn dispatch(cli, diagnostics)
-async fn connect(cli: &Cli, explicit_target: bool) -> Result<Client>;
+async fn connect(cli: &Cli) -> Result<Client>;     // Named → local::ensure_running; Url → checked
-async fn checked(server_url: &str, explicit_target: bool) -> Result<(Client, HealthResponse)>;
+async fn checked(endpoint: &Endpoint, explicit_target: bool) -> Result<(Client, HealthResponse)>;
-fn client(server_url: &str, http2: bool) -> Result<Client>;
+/// reqwest over the socket or pipe, base http://localhost/.
+fn client(endpoint: &Endpoint, http2: bool) -> Result<Client>;
 #[cfg(unix)] fn detach() -> Result<()>;
+/// The launcher already started it detached (local.rs).
+#[cfg(windows)] fn detach() -> Result<()>;
```

`role()` passes `name` instead of `port`. Bare `ship server` refuses `--server-url` and `--server-name` given as flags and points at `--name`; exported variables don't stop it, as today. Error messages that showed `client.url.origin()` show the endpoint instead, since every origin is now `http://localhost`.

**`crates/ship/src/local.rs`** (slice 2; Windows flags in slice 3)

```diff
-pub async fn default_health(client: &Client, config: &Path) -> Result<bool>;
+/// Reuse the named server or launch it. True when this call launched it.
+pub async fn ensure_running(client: &Client, name: &ServerName, config: &Path) -> Result<bool>;
```

It spawns `ship server --name NAME --starter --background-child`, and its logs go under `logs/NAME/`. On Windows, the spawn adds `creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP)` through `std::os::windows::process::CommandExt`, with the two constants defined locally.

**`crates/ship/src/diagnostics.rs`** (slice 1; Windows in slice 3)

```diff
-Role::Server { port: u16, log_file: Option<PathBuf> }
+Role::Server { name: ServerName, log_file: Option<PathBuf> }
 #[cfg(unix)] pub fn shutdown() -> Result<impl Future<Output = Result<()>> + Send + 'static>;
+/// Ctrl+C or the console closing.
+#[cfg(windows)] pub fn shutdown() -> Result<impl Future<Output = Result<()>> + Send + 'static>;
```

**`crates/ship-client/src/lib.rs`** (slice 2): `http_error` also treats `io::ErrorKind::NotFound` from a connect as `ConnectionRefused`, so a missing socket file means "no server running".

**`vendor/ratatui-ghostty/src/colors.rs`** (slice 3): the terminal query (`query_colors`, `read_responses`, `poll_read` and the `AsRawFd` import) moves into a `#[cfg(unix)] mod query`, and `query_colors` is re-exported from it. `HostColors` and `Rgb` stay portable. This replaces the per-function gates from the earlier probe. The two examples that call `query_colors` (`examples/single_terminal.rs`, `examples/demo/main.rs`) fall back to `HostColors::default()` off Unix, because `cargo test` builds examples.

### Proposed settings

Not written to manifests until the slice that needs them.

```toml
# crates/ship-core/Cargo.toml (slice 1): getuid for the directory check.
[target.'cfg(unix)'.dependencies]
nix = { workspace = true, features = ["user"] }

# crates/ship-server/Cargo.toml (slice 3)
[target.'cfg(windows)'.dependencies]
interprocess = { version = "2.4", features = ["tokio"] }

# crates/ship/Cargo.toml (slice 2): reqwest needs no new feature; unix_socket
# and windows_named_pipe are always on for their platforms in 0.13.

# deny.toml (slice 3, O-1)
allow = [ ..., "0BSD", ... ]
```

```yaml
# .github/workflows/ci.yml build job (slice 3)
strategy:
  fail-fast: false
  matrix:
    os: [ubuntu-24.04, macos-15]
    experimental: [false]
    include:
      - os: windows-latest
        experimental: true
continue-on-error: ${{ matrix.experimental }}
```

The Windows job runs the same rustup, Zig, cache, clippy and test steps. The apt step is already skipped off Linux.

### Untouched

- `crates/ship-server/src/middleware.rs`: `local_only` stays as it is (FR-010, K-6). `http://localhost/` passes its Host check.
- Routes, actors, the saver's format, the screen and input pipelines, and the UI. Only their transport changes.
- `checked`'s HTTP/2-then-HTTP/1.1 fallback for explicit targets. It now matters only for a proxy placed in front of a socket, and removing it is outside this change.
- Release packaging: there are no Windows binaries (product non-goal).

### Docs and specs (slice 4)

- `docs/site/troubleshooting.md`, `docs/site/saved-tabs.md`, `docs/site/windows-and-the-server.md`: ports become names and sockets. Add the ssh-forwarding example. Drop the warning that other accounts can drive the server (FR-012).
- Spec deltas under `openspec/changes/local-socket/specs/` for `health-exchange`, `diagnostics`, `session-restore`, `local-foundation`, plus `proposal.md`, `tasks.md` and `design.md` pointing at the three papers.

## Build order

### 0. Baseline latency on TCP

Before any change, on macOS, run the input and screen workloads from `docs/research/request-ordering-probe.md` and `docs/research/screen-cost.md` against a temporary server on a spare port in 43331 to 43337, using temporary XDG directories. Record the numbers in the deviation log.
Done when: the numbers are recorded. Which probe scripts still run is unverified; if none do, I'll stop and ask before writing new ones.

### 1. The server listens on a named socket

Files: `ship-core/src/endpoint.rs`, `ship-core/src/lib.rs`, `ship-core/src/protocol.rs`, `ship-core/Cargo.toml`, `ship-server/src/claim.rs` (Unix half), `ship-server/src/lib.rs`, `ship-server/src/health.rs`, `ship-server/src/pane.rs`, `ship/src/cli.rs` (`ServerArgs`), `ship/src/main.rs` (server arm, `role`), `ship/src/diagnostics.rs` (`Role`).

Behavior: `ship server --name probe` listens on `<dir>/probe.sock` and writes `tabs/probe.json` and `logs/probe/`. Its panes see `SHIP_SERVER_URL=unix:///…/probe.sock`. Clients don't reach it yet. Between slices 1 and 2 the branch is half-migrated, so nothing merges until slice 2 is done.

Done when, on macOS, with temporary XDG directories and never touching the 43179 server:
- `curl --unix-socket <dir>/probe.sock http://localhost/health` shows `socket`. The same request with `-H 'Host: evil.example'` gets 403.
- A second `ship server --name probe` exits with "already running".
- `kill -9` the server, then `ship server --name probe` starts without manual cleanup.
- The directory is `0700`. A `chmod 755` on it makes the server refuse to start.
- `lsof -iTCP -sTCP:LISTEN` shows no Ship process.
- `cargo fmt --check` and `cargo clippy --workspace --all-targets -D warnings` pass.

### 2. Clients, launcher and target selection

Files: `ship/src/cli.rs` (`Cli`, `Target`), `ship/src/main.rs` (dispatch, `connect`, `checked`, `client`), `ship/src/local.rs`, `ship-client/src/lib.rs`.

Behavior: FR-005 to FR-008. Bare `ship` and tab and pane commands use `default`, starting it if needed. Names and URLs follow FR-007's order. `http://` URLs are refused.

Done when, on macOS:
- Every P1 and P2 product story passes its independent verification, except the two-user one.
- Ten concurrent `ship tab list` calls against a stopped temporary server leave exactly one server.
- With `SHIP_SERVER_NAME=scratch` exported, `ship tab list` inside a `default` pane lists `default`'s tabs.
- `--server-url http://127.0.0.1:1` gets the pointer message.
- `--server-url` and `--server-name` together are refused.
- `ssh -N -L` to localhost forwards a socket, and the client works through it.
- Slice 0's workloads show no regression (SC-003).
- The two-user story needs a second account, and Linux needs a Linux machine; both are reported unverified unless run.

### 3. Windows builds

Files: `ship-server/src/claim.rs` (`pipe`), `ship-core/src/endpoint.rs` (`pipe`), `ship/src/main.rs` (`detach`), `ship/src/diagnostics.rs` (`shutdown`), `ship/src/local.rs` (creation flags), `vendor/ratatui-ghostty/src/colors.rs` plus its two examples, `ship-server/Cargo.toml`, `deny.toml`, `.github/workflows/ci.yml`.

Behavior: Windows compiles, and CI runs a Windows job that can fail without failing the workflow.

Done when:
- `cargo check --workspace --all-targets --target x86_64-pc-windows-gnu` passes from macOS through the zig wrapper. This checks compilation only.
- `cargo deny check` passes.
- macOS clippy still passes.
- After a push, the Windows job's result shows on the pull request (SC-004).
- Windows behavior is reported unverified.

### 4. Docs and specs

Files: the three `docs/site/` pages, the four spec deltas, `proposal.md`, `tasks.md`, `design.md`.

Behavior: the docs describe names, sockets and ssh forwarding. The specs no longer name ports.

Done when:
- `openspec validate local-socket --type change --strict` passes.
- `grep -rn 43179 docs/site openspec/specs` finds nothing outside archived changes once the specs are synced.
- The pages follow `docs/agents/docs.md`.
- The size report gives implementation Rust and tests separately (SC-005).

## Open unknowns

None. O-1 was resolved on 2026-10-27 by Cyan's approval, which took the recommendation:

- **O-1: 0BSD in `deny.toml`.** Resolved: allow 0BSD. interprocess depends on `doctest-file`, whose only license is 0BSD. `deny.toml` checks every target, so it fails even though the dependency is Windows-only. Recommendation: allow 0BSD. It's the least restrictive license on that list and compatible with GPL-3.0-or-later. The alternative, limiting cargo-deny to Linux and macOS targets, would hide Windows dependencies from future license checks too. `recvmsg`'s license isn't in the local registry, so it's unchecked; cargo-deny will report it.

## Deviation log

- 2026-10-27, during program design: `architecture.md` says a small adapter implements axum's `Listener` on both platforms. axum 0.8.9 already implements it for `tokio::net::UnixListener` (`serve/listener.rs:46`), so the adapter is needed on Windows only. This simplifies the plan without changing the shape. Under the ripple rule, editing the locked architecture paper would reopen it, so it's recorded here rather than edited there.

### Slice 0 baseline, 2026-10-28 UTC

The historical probes were not preserved. Cyan approved temporary replacements in this apply session. This is a new baseline for the socket comparison, not a replication of the old research numbers. No implementation Rust or permanent tests changed.

Measured on macOS arm64 (Darwin 25.6.0, Mac16,6, 16 logical CPUs), rustc 1.98.0, Zig 0.17.0, release build at `e1a69dfa5a1523f6e7bb9157955e25f8fbd4ca12`, with the pre-existing `vendor/ratatui-ghostty/src/colors.rs` edit untouched. The binary SHA-256 is `d68481083dd6503b2fecbd6dfce3bcdf780af6fa0477bf2b7b7b75e823733453`.

#### Input

Three runs per transport. One POST in flight with queued frames batched into the next. Typing: 200 keys at 30 ms; drag: 500 events at 8 ms; flood: 2000 keys without a gap; paste: ten 64 KiB frames at 100 ms, or one 1 MiB frame. Ranges below span per-run percentiles, in milliseconds. PTY receipt means all bytes of the frame reached the raw-mode pane program. HTTP acknowledgment means the input route accepted/routed the request, not that the program read it.

| Transport / workload | PTY p50 | PTY p99 | HTTP ACK p50 | HTTP ACK p99 | Requests/run |
| --- | --- | --- | --- | --- | --- |
| h2/typing | 0.468 to 0.487 | 0.774 to 1.066 | 0.550 to 0.568 | 0.900 to 1.116 | 200 to 200 |
| h2/drag | 0.288 to 0.337 | 0.693 to 0.721 | 0.349 to 0.404 | 0.757 to 0.813 | 500 to 500 |
| h2/flood | 3.970 to 4.346 | 4.363 to 4.553 | 3.989 to 4.193 | 4.193 to 4.715 | 3 to 3 |
| h2/paste64k | 7.369 to 8.967 | 14.317 to 14.892 | 0.495 to 0.702 | 1.071 to 1.538 | 10 to 10 |
| h2/paste1m | 50.202 to 50.582 | 50.202 to 50.582 | 1.536 to 1.773 | 1.536 to 1.773 | 1 to 1 |
| h1/typing | 0.398 to 0.405 | 0.657 to 0.689 | 0.460 to 0.469 | 0.711 to 0.766 | 200 to 200 |
| h1/drag | 0.286 to 0.290 | 0.586 to 0.608 | 0.338 to 0.346 | 0.643 to 0.781 | 500 to 500 |
| h1/flood | 3.210 to 3.353 | 4.573 to 4.807 | 3.087 to 3.231 | 4.764 to 4.964 | 3 to 3 |
| h1/paste64k | 5.634 to 11.173 | 14.408 to 17.155 | 0.450 to 0.726 | 0.832 to 1.913 | 10 to 10 |
| h1/paste1m | 50.713 to 51.040 | 50.713 to 51.040 | 1.323 to 1.705 | 1.323 to 1.705 | 1 to 1 |

All 30 input runs completed, with exactly 16,266 frames delivered in byte-for-byte order and no request failures. A separate Python pass recomputed every PTY latency from the raw receipt logs and creation timestamps. The 1 MiB rows have only one sample per run, so their p99 is not a tail estimate.

Unlike the old synthetic input probe, receipt includes actual PTY throughput and the temporary Python reader. In particular, its logging/reading cost affects paste completion. The ACK column keeps route completion separate. The old NDJSON and unbounded-spawn modes were not repeated because today's server uses POST input.

#### Screens and attaching

Three HTTP/2 runs, 120x40 pane, zstd-compressed attach stream, decoded by reqwest, `sse-stream` and JSON. Tick rewrites one line at 60 Hz; spinner one line at 30 Hz; matrix writes 180 deterministic cells at 30 Hz; full changes 120x40 cells at 30 Hz. Each timed workload runs for 10 seconds. Generation-to-SSE latency ends after decompression and JSON decoding, not after a terminal draw.

| Workload | Generation-to-SSE p50, ms | p99, ms | Screens/run |
| --- | --- | --- | --- |
| tick | 3.006 to 3.026 | 3.239 to 3.391 | 600 to 601 |
| spinner | 3.080 to 3.108 | 3.245 to 3.788 | 301 to 301 |
| matrix | 4.517 to 4.623 | 6.505 to 7.187 | 300 to 301 |
| full | 10.369 to 11.188 | 14.923 to 15.272 | 300 to 301 |
| flood | - | - | 27 to 28 |

Twenty warm-connection attaches reached the first decoded screen in p50 0.193 ms, p99 0.359 ms. Each flood wrote exactly 32 MiB; producer elapsed was 0.166 to 0.168 seconds. Flood has no generation marker, so no per-frame latency is claimed. Screens can include the initial/stale snapshot as well as the unique marked frames.

#### Actual terminal-client CPU

One run per workload, a drained 120x42 pseudo-terminal, sidebar hidden, with the server-published pane geometry retained in the raw results. CPU is server plus the real full-screen Ship client, percent of one core, excluding the generator, pane program and PTY drainer. Ten seconds per timed workload. Flood also includes a 300 ms final-draw tail; its producer time is recorded separately. These dimensions differ from the historical 120x40 CPU runs and must stay fixed for the socket comparison.

| Workload | Server CPU | Client CPU | Total CPU | Terminal output |
| --- | --- | --- | --- | --- |
| tick | 2.69% | 2.00% | 4.69% | 28,019 bytes |
| spinner | 1.40% | 1.10% | 2.50% | 19,442 bytes |
| matrix | 3.29% | 1.80% | 5.09% | 462,934 bytes |
| full | 8.98% | 7.58% | 16.56% | 20,925,046 bytes |
| flood | 26.25% | 2.02% | 28.27% | 8,994 bytes |
| htop | 0.30% | 0.10% | 0.40% | 111,587 bytes |
| nvim | 7.90% | 7.40% | 15.30% | 186,508 bytes |

The CPU flood producer wrote 32 MiB in 0.182 seconds, while the measurement including the final-draw tail took 0.495 seconds. This short fixed-size throughput probe is not a sustained 10-second flood CPU measurement. nvim ran with `--clean -n` on 10,000 lines and received 600 Ctrl-E keys at 60/s through the real client's PTY. htop used `-d 5`.

#### Evidence and limits

The owned server was PID 8866 on `127.0.0.1:43331`, protocol 18. Runtime, state, config and home were under `/tmp/ship-socket-baseline.1bcUB2/`; the existing 43179 server was not contacted or changed. The runner completed with exit 0, all seven terminal clients remained alive until deliberately detached, and the server shut down with exit 0. Its PID and listener were then absent.

Raw results, per-frame creation and receipt logs, temporary probe sources, generated probe lockfile, run/build logs and snapshots of both executables are retained in `/tmp/ship-socket-baseline.1bcUB2/`. A verified archive is at `/Users/cyan/.cache/ship/benchmarks/local-socket/tcp-2026-10-28-1bcUB2.tar.gz`. The archive's `README.md` describes fresh-directory reproduction and the minimal transport-only changes needed for the socket comparison. reqwest 0.13.5, Tokio 1.53.3, h2 0.4.20, hyper 1.11.1 and `sse-stream` 0.3.0 match the repository lockfile.

`cargo build --workspace`, `cargo build --workspace --release`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `openspec validate local-socket --type change --strict` passed. No permanent authored tests were added. Cross-process receipt and screen timing use the same host's wall clock, not a clock-correction-resistant shared monotonic clock. Python receipt logging, the value-based probe JSON decoder and synthetic output patterns affect the absolute numbers; repeat the same probes for before/after comparisons. CPU has one run per workload and centisecond process-time resolution. Linux, Windows, subjective typing feel and rendered-display latency remain unverified. Slice 1 has not started.
