# Run actions: program

Status: Locked on 2026-10-30 by Cyan in chat, without a Plannotator pass ("the change is trivial. just change everything and don't make me reread"), then reviewed in Plannotator the same day. That review asked why the runner's output is a function; the answer (`RunLog` rotates and lives in the `ship` binary) kept it, and the field was renamed `log_file`. Added while writing the OpenSpec artifacts: `PROTOCOL_VERSION` goes to 21 for the new route, following the precedent of the log filter route (12 to 13). Built on the relocked `product.md` and `architecture.md`: commands are fire-and-forget and argv only. Two earlier drafts were never locked: one stopped runs with their owner, and one also had a string form.

## Rationale

This is a document-only plan. The code blocks below are a virtual skeleton: declarations only, never created as files, never compiled. Signatures will change where the compiler disagrees, and anything that changes behavior goes in the deviation log.

The skeleton expresses shape A from the architecture paper:

- **Commands are argv everywhere.** `Run.command` is `Vec<String>`, like `pane::Create.command`, `NewPane.command` and the CLI's `-- COMMAND`. Nothing parses a shell string (architecture decision 2).
- **The runner is `ship-core/src/run.rs`, behind a `run` feature.** It takes a prepared `tokio::process::Command`, so each side decides its own argv, directory and environment. The runner owns only the process group, where output goes, and the exit (decisions 4, 5 and 10). It keeps no list of runs, because nothing stops them.
- **Output goes to a clone of the side's log file.** The `ship` binary owns `RunLog`, so it hands each side a function that clones the current part's file handle, and the runner calls it once per run (decision 10).
- **`server.run` is spawned by the state actor.** The actor already holds `PaneEnv` and the tabs for the anchor-pane directory, so runs get the pane environment and the pane directory rule without new plumbing (decisions 6, 8 and 9).
- **`client.run` is spawned by the UI's `Local`.** It already handles each action's turn and owns `message` (decision 7).

## Skeleton map

### Added

#### `crates/ship-core/src/run.rs` (slice 1)

```rust
//! Commands started by `server.run` and `client.run`: each in its own
//! process group, stdin empty, output written into the owner's log file,
//! exit reported. Nothing stops a run; it outlives its owner (FR-013).

use std::{fs::File, process::ExitStatus};
use tokio::{process::Command, sync::mpsc};

/// How a run ended, for an owner that reports it.
pub struct Ended {
    /// argv joined with spaces, for messages and log lines.
    pub label: String,
    pub status: ExitStatus,
}

/// What the owner does with a run's exit.
pub enum OnExit {
    /// The server: the exit is logged only (FR-012).
    Log,
    /// The client: the exit is logged, and a failure is also sent here for
    /// the status line (FR-012).
    Report(mpsc::UnboundedSender<Ended>),
}

/// Starts runs on one side.
pub struct Runner {
    /// Opens the owner's current log part for one run's stdout and stderr.
    /// A function because the log rotates (FR-010, decision 10).
    log_file: Box<dyn Fn() -> Result<File> + Send + Sync>,
    on_exit: OnExit,
}

impl Runner {
    pub fn new(log_file: impl Fn() -> Result<File> + Send + Sync + 'static, on_exit: OnExit) -> Self;

    /// Sets stdin to null and stdout and stderr to `log_file()`, detaches the
    /// child into its own process group, spawns, and logs the label and
    /// process id. Returns once the process exists; a spawn error is returned
    /// (FR-011). A detached task waits for the exit and handles it per
    /// `on_exit`. When the owner exits, the task is dropped and the child
    /// carries on.
    pub fn spawn(&self, label: String, command: Command) -> Result<()>;
}

/// Own process group, so terminal signals aimed at the owner miss the run
/// (decision 5).
#[cfg(unix)]
fn detach(command: &mut Command);

/// No process groups to set.
#[cfg(not(unix))]
fn detach(command: &mut Command);
```

#### `crates/ship-client/src/ui/run.rs` (slice 1)

```rust
/// Build a `client.run` command: argv, the client's own directory (FR-008),
/// and `SHIP_BIN`, `SHIP_SERVER_URL`, `SHIP_ATTACHMENT_ID` and, when a pane
/// is selected, `SHIP_PANE_ID` (FR-007). The label is argv joined with spaces.
pub(super) fn start(runner: &Runner, client: &Client, observer: &Observer, command: Vec<String>) -> Result<()>;

/// "`gh pr view --web` exited with 3", "... was killed by signal 9".
pub(super) fn describe(ended: &Ended) -> String;
```

### Changed

#### `crates/ship-core/Cargo.toml` (slice 1)

```toml
[features]
run = ["tokio/process", "tokio/rt", "tokio/sync"]
```

`ship-client` enables `ship-core/run` in slice 1, and `ship-server` in slice 2.

#### `crates/ship-core/src/lib.rs` (slice 1)

```rust
#[cfg(feature = "run")]
pub mod run;
```

#### `crates/ship-core/src/command.rs` (`Run` in slice 1, `Command::Run` in slice 2)

```rust
/// `server.run` and `client.run`. Not a CLI command (product non-goal).
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Run {
    /// Argv, required and non-empty (FR-002).
    #[serde(deserialize_with = "non_empty")]
    pub command: Vec<String>,
}

/// Rejects an empty array when the config loads.
fn non_empty<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<String>, D::Error>;

pub enum Command {
    Tab(tab::TabCommand),
    Pane(pane::PaneCommand),
    /// Skipped by clap: no `ship run` (architecture decision 1). (slice 2)
    #[cfg_attr(feature = "clap", command(skip))]
    Run(Run),
}
```

#### `crates/ship-core/src/protocol.rs` (slice 2)

```rust
/// A new route, so the version moves, as it did for the log filter route.
pub const PROTOCOL_VERSION: u32 = 21;

/// `POST /api/v0/runs`.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RunCommand {
    pub command: Vec<String>,
    /// The selected pane: `SHIP_PANE_ID` and the directory (FR-007, FR-008).
    #[serde(default, deserialize_with = "present")]
    #[schema(nullable = false)]
    pub pane: Option<IdOf<Pane>>,
    /// `SHIP_ATTACHMENT_ID` (FR-007).
    #[serde(default, deserialize_with = "present")]
    #[schema(nullable = false)]
    pub attachment: Option<IdOf<Attachment>>,
}
```

#### `crates/ship-core/src/defaults.toml` (slice 1)

```toml
{ label = "edit scrollback", action = { server.pane.create = { place = "stack", select = true, exit_behavior = "keep_failed", name = "scrollback", command = ["sh", "-c", "exec \"$SHIP_BIN\" pane edit-scrollback"] } } },
"alt-e"       = { action = { server.pane.create = { place = "stack", select = true, exit_behavior = "keep_failed", name = "scrollback", command = ["sh", "-c", "exec \"$SHIP_BIN\" pane edit-scrollback"] } } }
```

Both gain `name = "scrollback"`; nothing else changes.

#### `crates/ship/src/runlog.rs` and `diagnostics.rs` (slice 1)

```rust
impl RunLog {
    /// A clone of the current part's handle, for a run's stdout and stderr.
    /// Bytes written through it skip `written`, so rotation doesn't count
    /// them (architecture risk "Log size and rotation").
    pub fn output(&self) -> io::Result<File>;
}

impl Diagnostics {
    /// For `Runner::new` on either side. Errors when this process has no run
    /// log, which only the plain CLI lacks.
    pub fn run_output(&self) -> impl Fn() -> Result<File> + Send + Sync + 'static;
}
```

`main.rs` passes `diagnostics.run_output()` to `ui::run` (slice 1) and to `ship_server::serve` (slice 2).

#### `crates/ship-client/src/keymap.rs` (slice 1; server label in slice 2)

```rust
pub enum ClientAction {
    // ...
    Run(Run),
}

impl Action {
    pub fn label(&self) -> Cow<'static, str> {
        // Self::Server(Command::Run(_)) => "run"   (slice 2)
        // Self::Client(ClientAction::Run(_))       → "run" through strum
    }
}
```

#### `crates/ship-client/src/ui/mod.rs` (slice 1)

```rust
pub async fn run(client: &Client, open_first: bool, config: &Path, run_output: impl Fn() -> Result<File> + Send + Sync + 'static) -> Result<()>;

/// Runs from `client.run`. Quitting leaves them running (FR-013).
struct Local<'a> {
    // ...
    runner: Runner,                             // OnExit::Report
    ended: mpsc::UnboundedReceiver<Ended>,
}

// In the main `select!`:
//   ended = local.ended.recv() => { local.message = Some(run::describe(&ended)); true }

// In `act`:
//   ClientAction::Run(Run { command }) => run::start(&self.runner, client, &self.observer, command)
```

#### `crates/ship-client/src/execute.rs` and `api.rs` (slice 2)

```rust
pub enum Outcome {
    // ...
    /// A `server.run` command started. The CLI never produces it.
    Started,
}

// execute: Command::Run(Run { command }) under Scope::Keys sends RunCommand
// with the selected pane and attachment. Under Scope::Cli it is unreachable,
// because clap skips the variant; it returns a Validation error, not a panic.

impl Client {
    /// `POST /api/v0/runs`; 202 with no body.
    pub async fn run(&self, body: &RunCommand) -> Result<()>;
}
```

#### `crates/ship-server/src/routes.rs` and `lib.rs` (slice 2)

```rust
#[utoipa::path(
    post,
    path = "/api/v0/runs",
    operation_id = "run",
    request_body = RunCommand,
    responses(
        (status = 202, description = "The command started; its exit is only logged"),
        (status = 400, description = "Empty command, a program that can't start, or no usable directory", body = AppError),
        (status = 404, description = "Pane not found", body = AppError),
        (status = 503, body = AppError),
    ),
)]
pub(crate) async fn run(State(app): App, Body(body): Body<RunCommand>) -> Result<StatusCode>;
```

`lib.rs` adds `routes::run` to `api_router`, and `serve` gains a `run_output` parameter that it hands to the state actor's `Runner`.

#### `crates/ship-server/src/state.rs` (slice 2)

```rust
pub struct ServerState {
    // ...
    /// `server.run` commands. `OnExit::Log`; never stopped (FR-013).
    runner: Runner,
}

/// Directory from the pane's tracked `dir`, else its start `cwd`; shared
/// by `CreatePane` (around line 656) and `RunCommand`.
fn anchor_dir(tab: &Tab, anchor: IdOf<Pane>) -> Option<String>;

impl Message<RunCommand> for ServerState {
    type Reply = Result<()>;
    // pane_cwd(anchor_dir(..)), else home; PaneEnv; runner.spawn
}
```

#### `crates/ship-server/src/pane.rs` (slice 2)

```rust
/// What `PaneEnv` writes into: portable-pty's builder or tokio's `Command`.
pub(crate) trait ChildEnv {
    fn set(&mut self, key: &str, value: impl AsRef<OsStr>);
    fn remove(&mut self, key: &str);
}
impl ChildEnv for CommandBuilder;
impl ChildEnv for tokio::process::Command;

impl PaneEnv {
    /// Unchanged variables. `pane` is the pane's own ID for a pane, and the
    /// selected pane, or unset, for a run (FR-009).
    fn apply(&self, child: &mut impl ChildEnv, pane: Option<IdOf<Pane>>);

    /// A `server.run` command with argv, directory and this environment, plus
    /// `SHIP_ATTACHMENT_ID` when given.
    pub(crate) fn run_command(&self, command: &[String], cwd: &Path, pane: Option<IdOf<Pane>>, attachment: Option<IdOf<Attachment>>) -> tokio::process::Command;
}
```

#### Docs (slices 1 and 2)

- `docs/site/configuration.md`: `client.run` (slice 1) and `server.run` (slice 2), with an example each, including `["sh", "-c", ...]` for when a shell is wanted. It covers where output goes, how failures show, and that Ship never stops a run, so anything that should end with Ship belongs in a pane.
- `docs/site/keys.md`: no change. `alt-e` keeps its row.

#### Spec deltas (slice per behavior)

- `openspec/changes/run-actions/specs/keymap/spec.md`: the `alt-e` default sentence gains `name = "scrollback"` (slice 1); new requirements for `client.run` (slice 1) and `server.run` (slice 2).

### Moved

None.

### Replaced

None. `PaneEnv::apply` changes its parameter type but keeps its behavior for panes.

### Untouched

- The CLI: `ship pane create -- COMMAND` stays argv, and there's no `ship run`.
- Pane spawning in `pane::spawn`, apart from calling `apply` through `ChildEnv`.
- Pane teardown, `GRACE` and `stop_panes`: runs aren't part of shutdown.
- The attach stream and replica: runs publish no state and no events.
- `pane::Create.command`, which stays argv as it is.
- `pane_label`: a `["sh", "-c", ...]` pane still shows `sh` unless named.

## Build order

### 1. Runner, `client.run` and the scrollback name

Files: `ship-core/Cargo.toml`, `ship-core/src/lib.rs`, `run.rs`, `command.rs` (`Run`), `defaults.toml`, `runlog.rs`, `diagnostics.rs`, `main.rs` (client), `ship-client/Cargo.toml`, `keymap.rs`, `ui/mod.rs`, `ui/run.rs`, docs, spec delta.

Input to output: a key bound to `client.run` starts the command on the client without blocking. A failure to start, or a non-zero exit, shows in the status line. Output lands in the client log, and the command keeps running after the client quits. `alt-e` opens a pane labeled `scrollback`.

Checks:
- The `client.run` scenarios in P1, P2 and P3, and P4.
- `ship config check` on configs with `client.run = { command = [] }` and with `command = "ls"`, which must both fail with a message naming the field.
- `ship config defaults` showing the new `alt-e`.
- SC-003 for the client: bind `client.run = { command = ["sh", "-c", "while true; do echo tick; sleep 1; done"] }`, quit the client, and after 5 seconds confirm with `pgrep -f "echo tick"` that it's running and the client log is still growing. Repeat by closing the client's terminal window instead of quitting.
- `cargo fmt`, clippy with `-D warnings`, and a workspace build.

### 2. `server.run`

Files: `command.rs` (`Command::Run`), `protocol.rs`, `keymap.rs` (label), `execute.rs`, `api.rs`, `routes.rs`, `lib.rs`, `state.rs`, `pane.rs`, `main.rs` (server), `ship-server/Cargo.toml`, docs, spec delta.

Input to output: a key bound to `server.run` starts the command on the server, in the selected pane's directory, with the pane environment. A failure to start shows in the status line. A non-zero exit and all output go to the server log only, and the command keeps running after the server stops.

Checks:
- The `server.run` scenarios in P1, P2 and P3.
- SC-001: type straight after a `["sleep", "30"]` run.
- SC-003 for the server: the same `tick` loop, then `ship server stop`, then `pgrep` after 5 seconds. Repeat with a foreground `ship server` stopped by Ctrl-C.
- `ship --help` lists no `run` command, and `ship server status` reports `protocol_version` 21.
- A pane's environment is unchanged: compare `env` in a new pane before and after.
- fmt, clippy and build.

Both slices are verified on macOS, where this work happens. Linux is reported as unverified unless Cyan checks it there.

## Deviation log

- Slice 1 follow-up, approved by Cyan in chat: `Run.command` and `ui::run::start` use the existing `mitsein::vec1::Vec1<String>` instead of `Vec<String>` plus a custom deserializer. The config stays an argv array and rejects empty commands with line/column diagnostics; process construction uses `split_first()` without indexing. `ui::run` also replaces its pre-existing `open_first: bool` with `InitialSelection::{None, FirstPane}`. Live checks confirmed unchanged startup selection for a newly launched server, an existing named server and an explicit server URL. The complete slice 1 probe and debug/release builds passed again on macOS.

- Slice 1: the live environment probe found that `Client.url` is the internal HTTP base (`http://localhost/`), not a usable `SHIP_SERVER_URL`. `Client::new` now takes and retains the existing `Endpoint` and builds the internal HTTP base itself. `ui/run.rs` exports `client.endpoint.url()`. This changes constructor plumbing, not the locked environment behavior, and lets a run call Ship back over the same socket.
