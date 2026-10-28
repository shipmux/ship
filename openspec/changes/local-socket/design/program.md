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
