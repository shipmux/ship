# Update notices, an updates panel and `ship update`: program

Status: Locked on 2026-11-03. Built on the locked [product.md](product.md) (with its approved amendment) and the locked [architecture.md](architecture.md). Drafting it re-opened the architecture's version probe, approved again with this paper. A first review asked for `Picked` over a new `Choice` type and for the skeleton to follow the codebase's conventions; the revision was approved in Plannotator review ("LGTM").

## Summary

Six slices. The first three need no full-screen UI work: clearer version and mismatch messages, then `ship update --check` with the config table and the cache, then `ship update` itself. Slice 4 puts the notice in the client: the sidebar row and the one-time note. Slice 5 adds the panel. Slice 6 is the install script, release targets and docs, with the website's download count folded in.

Two choices in here aren't in the papers and need a look:

- **Commands are shell strings.** `command`, `latest_command` and the build-time `SHIP_UPDATE_COMMAND`/`SHIP_LATEST_COMMAND` are one string each, run with `sh -c`, the way the product paper's example writes them (`command = "brew upgrade ship"`). Packagers and users write shell lines, and an argv array like `server.pane.create`'s `command` would make them quote by hand. On Windows these run with `cmd /C`, best effort.
- **"full notes" is a plain URL.** The panel prints each release's GitHub URL on a dim line, and the terminal's own link detection makes it clickable. Opening a browser would need platform code for `open`/`xdg-open`/`start`.

This is a plan written in the document only. None of the code below exists in the working tree, and none of it has been compiled.

## Rationale

Shape A puts all update logic in one new client module, `ship-client/src/updates/`, which the binary also calls, so the CLI and the full-screen client share one checker and one probe:

- `updates/mod.rs` owns settings, versions and the method choice.
- `updates/check.rs` owns finding the latest release and its notes, and the cache file that doubles as the one-hour stamp.
- `updates/probe.rs` owns asking an installed binary for its version and protocol, and the sync message built from that.

The full-screen client adds `ui/updates/mod.rs`, the tracker that lives in `Local` for the whole session (it follows the update pane even while the panel is closed), and `ui/updates/panel.rs`, the overlay. Both follow the loop's existing idioms: the tracker schedules like `Attention` (`due()` polled with `until`) and keeps pending requests the way the lane does (polled with `finish`), and the panel returns `Picked` like `Menu` and `Rename`, with its buttons as ordinary client actions. The binary adds `crates/ship/src/update.rs` for `ship update`, the only place `self_update` is compiled, behind the `self-update` feature.

The server and `ship-core`'s protocol don't change. The update tab uses the existing `create_tab` and `create_pane` calls, and the tracker reads the pane's published `status`.

## Skeleton map

### Added

#### `crates/ship-client/src/updates/mod.rs` (slice 2)

```rust
//! Update settings, versions and how this build updates. Shared by
//! `ship update` and the full-screen client.

mod check;
mod probe;

pub use check::{Checked, Checker, Found, Freshness, Release};
pub use probe::{Probed, Sync, mismatch, probe};

use std::path::PathBuf;

use semver::Version;

/// `[client.updates]` with defaults applied. Replaced whole on reload.
#[derive(Clone)]
pub struct UpdateSettings {
    /// Automatic checks (FR-004).
    pub check: bool,
    /// Overrides SHIP_UPDATE_COMMAND; a shell line.
    pub command: Option<String>,
    /// Overrides SHIP_LATEST_COMMAND; a shell line printing the latest version.
    pub latest_command: Option<String>,
}

/// What the binary knows and the library can't: where the cache lives and
/// whether this build has the `self-update` feature.
pub struct UpdateContext {
    pub cache: PathBuf,
    pub self_replace: bool,
}

/// Build-time values (FR-005, FR-014, product OU-4).
pub const UPDATE_COMMAND: Option<&str> = option_env!("SHIP_UPDATE_COMMAND");
pub const LATEST_COMMAND: Option<&str> = option_env!("SHIP_LATEST_COMMAND");
/// `owner/name` on GitHub; release builds leave it at shipmux/ship.
pub const RELEASE_REPO: &str = match option_env!("SHIP_RELEASE_REPO") {
    Some(repo) => repo,
    None => "shipmux/ship",
};

/// How an update is applied, first match wins (FR-014).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Method {
    /// The config's `command`, else SHIP_UPDATE_COMMAND.
    Command(String),
    SelfReplace,
    /// Neither, and no `self-update` feature: notify only (FR-016).
    Unavailable,
}

impl UpdateSettings {
    /// Computed each time; never stored beside the settings it comes from.
    pub fn method(&self, context: &UpdateContext) -> Method;
    /// The config's `latest_command`, else SHIP_LATEST_COMMAND.
    pub fn latest_command(&self) -> Option<&str>;
}

/// The `major.minor.patch` in `0.2.0`, `v0.2.0`, `0.2.0-1` or
/// `ship 0.2.0 (protocol 22)`; packager suffixes are dropped (notes amendment).
pub fn core_version(text: &str) -> Option<Version>;

/// This binary's version.
pub fn installed() -> Version;

/// A shell line as a child process: `sh -c` on Unix, `cmd /C` elsewhere.
#[cfg(unix)]
fn shell(line: &str) -> tokio::process::Command;
#[cfg(not(unix))]
fn shell(line: &str) -> tokio::process::Command;
```

#### `crates/ship-client/src/updates/check.rs` (slice 2)

```rust
//! Finding the latest release and its notes, through a cache file shared by
//! every client on the machine (FR-001 to FR-005).

use std::{path::PathBuf, time::{Duration, SystemTime}};

use semver::Version;
use serde::{Deserialize, Serialize};
use ship_core::prelude::*;

/// How long a check's result stands for every client (FR-002).
pub const FRESH_FOR: Duration = Duration::from_secs(60 * 60);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Freshness {
    /// Reuse a result under FRESH_FOR old: the schedule.
    Cached,
    /// Ask now: `[check]` and `ship update --check`.
    Force,
}

/// One published release, as GitHub lists it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Release {
    pub version: Version,
    pub published: Option<String>,
    pub url: String,
    /// release-plz's changelog entry, as markdown.
    pub notes: String,
}

/// The cache file's contents.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Found {
    pub checked_at: SystemTime,
    /// None until something is released.
    pub latest: Option<Version>,
    /// Every non-draft, non-prerelease on the first page, newest first.
    /// Readers pick those newer than their own version, so clients of
    /// different versions share one file.
    pub releases: Vec<Release>,
    /// The newest version a one-time note was shown for (product OU-1).
    pub announced: Option<Version>,
}

/// A check's result, saying whether this caller shows the one-time note.
pub struct Checked {
    pub found: Found,
    /// `latest` is newer than `announced` was; the file now records it.
    pub announce: bool,
}

/// Cheap to clone: reqwest's client is shared.
#[derive(Clone)]
pub struct Checker {
    http: reqwest::Client,
    cache: PathBuf,
}

impl Checker {
    pub fn new(cache: PathBuf) -> Result<Self>;
    /// The file, if readable; anything unreadable is no cache.
    pub fn cached(&self) -> Option<Found>;
    /// Reuses a fresh file for `Cached`, else runs `latest_command` if set
    /// and asks GitHub, then writes the file whole by temp file and rename.
    /// Errors go to the caller (FR-003).
    pub async fn check(&self, freshness: Freshness, latest_command: Option<&str>) -> Result<Checked>;
}

impl Found {
    /// Releases newer than `than`, newest first (FR-010).
    pub fn newer(&self, than: &Version) -> impl Iterator<Item = &Release>;
    /// The release whose tag matches `version`'s major.minor.patch.
    pub fn release(&self, version: &Version) -> Option<&Release>;
}
```

GitHub request: `GET https://api.github.com/repos/{RELEASE_REPO}/releases?per_page=30` with a `User-Agent`.

#### `crates/ship-client/src/updates/probe.rs` (slice 1)

```rust
//! Ask an installed `ship` for its version and protocol, and say what that
//! means for the running server (FR-017, FR-019, FR-020, product OU-2).

use std::ffi::OsStr;

use semver::Version;
use ship_core::{HealthResponse, prelude::*};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Probed {
    pub version: Version,
    pub protocol: u32,
}

/// What an installed binary means for the running server.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Sync {
    /// Same protocol: restart when convenient.
    Compatible(Probed),
    /// Another protocol: the server must restart.
    Restart(Probed),
    /// The probe failed: the server may need a restart.
    Unknown,
}

/// Runs `<program> --version` and parses `ship 0.2.0 (protocol 22)`.
/// `program` is `current_exe()` after self-replace, or `"ship"` to search
/// PATH after a command.
pub async fn probe(program: &OsStr) -> Result<Probed>;

impl Sync {
    pub fn of(probed: Result<Probed>, server: &HealthResponse) -> Self;
    /// The lines `ship update` prints and the panel shows, with the restart
    /// command and its cost.
    pub fn describe(&self, server: &HealthResponse) -> String;
}

/// The attach-time mismatch error, naming both sides and the restart (FR-020).
pub fn mismatch(server: &HealthResponse) -> String;
```

#### `crates/ship/src/update.rs` (slice 2 for `--check`, slice 3 for the rest)

```rust
//! `ship update` and `ship update --check`.

use std::process::ExitCode;

use ship_client::updates::{Checker, UpdateContext, UpdateSettings};
use ship_core::{HealthResponse, prelude::*};

/// Prints `0.1.0 → 0.2.0`, `up to date` or `no releases yet`; changes nothing (FR-018).
pub async fn check(checker: &Checker, settings: &UpdateSettings) -> Result<ExitCode>;

/// Applies the method, then prints `Sync::describe` when a server is running.
/// `Method::Unavailable` prints how to update and fails (FR-016). `server`
/// comes from `checked_target`, which never starts one.
pub async fn update(
    checker: &Checker,
    settings: &UpdateSettings,
    context: &UpdateContext,
    server: Option<HealthResponse>,
) -> Result<ExitCode>;

/// self_update's GitHub backend for RELEASE_REPO and this target's
/// `.tar.gz`, checksum verified, in `spawn_blocking` (FR-015).
#[cfg(feature = "self-update")]
async fn self_replace() -> Result<()>;
```

#### `crates/ship-client/src/ui/updates/mod.rs` (slice 4)

The tracker follows `Attention` and the lane: it says when it's next due, and keeps each pending request as an `Option<Request>` that the loop polls with `finish`, the way it polls `lane.finished()`. No channels.

```rust
//! What the client knows about updates for the session: read by the
//! sidebar row, the one-time note and the panel.

mod panel;

pub(super) use panel::Panel;

use std::time::Instant;

use semver::Version;
use ship_core::{HealthResponse, id::IdOf, model::Pane, tree::Tabs};

use super::Request;
use crate::{
    Client,
    updates::{Checked, Checker, Found, Freshness, Sync, UpdateContext, UpdateSettings},
};

/// The panel's update once its pane exists (FR-011, FR-013). "Starting"
/// isn't a state: it's `Tracker::starting` being Some.
pub(super) enum Run {
    Idle,
    Running(IdOf<Pane>),
    Installed(Sync),
    Failed,
}

pub(super) struct Tracker<'a> {
    checker: Checker,
    context: UpdateContext,
    settings: UpdateSettings,
    /// From `/health` at startup; a restarted server ends this client.
    server: HealthResponse,
    found: Option<Found>,
    /// Polled with `finish`, like the lane's running request.
    checking: Option<Request<'a, Checked>>,
    /// Why the last check failed; the previous `found` still stands.
    check_failed: Option<String>,
    /// Creating the `update` tab and its pane.
    starting: Option<Request<'a, IdOf<Pane>>>,
    run: Run,
    /// When the schedule asks next; None while checks are off.
    due: Option<Instant>,
}

/// What the sidebar's bottom row draws (FR-006).
pub(super) enum VersionRow<'a> {
    Current(&'a Version),
    Available { installed: &'a Version, latest: &'a Version },
}

impl<'a> Tracker<'a> {
    /// Reads the cache, and is due now when checks are on.
    pub fn new(context: UpdateContext, settings: UpdateSettings, server: HealthResponse) -> Result<Self>;
    pub fn due(&self) -> Option<Instant>;
    /// Starts a check unless one is pending; the schedule passes `Cached`,
    /// `client.updates.check` passes `Force`. Moves `due` an hour on.
    pub fn check(&mut self, freshness: Freshness);
    /// The pending check, for `finish` in the loop.
    pub fn pending_check(&mut self) -> &mut Option<Request<'a, Checked>>;
    /// Stores a finished check; the one-time note when it announces.
    pub fn checked(&mut self, result: Result<Checked>) -> Option<String>;
    /// `client.updates.update`: starts creating the update tab.
    pub fn update(&mut self, client: &'a Client);
    /// The pending tab creation, for `finish` in the loop.
    pub fn pending_start(&mut self) -> &mut Option<Request<'a, IdOf<Pane>>>;
    pub fn started(&mut self, result: Result<IdOf<Pane>>);
    /// After each publication, like `Sidebar::tabs.reconcile`: an exited
    /// update pane becomes Installed or Failed, a closed one Idle.
    pub fn reconcile(&mut self, tabs: &Tabs);
    pub fn row(&self) -> VersionRow<'_>;
    /// On config reload; turning checks on makes it due now.
    pub fn configure(&mut self, settings: UpdateSettings);
}
```

#### `crates/ship-client/src/ui/updates/panel.rs` (slice 5)

The panel returns `Picked`, like `Menu` and `Rename`. `[check]` and `[update]` are the actions `client.updates.check` and `client.updates.update`, so the loop runs them through `act` like any picked menu item, and keys or menus can bind them.

```rust
//! The updates overlay (FR-009 to FR-013).

use crossterm::event::Event;
use ratatui::{Frame, layout::Rect};

use super::Tracker;
use crate::ui::menu::Picked;

pub(super) struct Panel {
    scroll: u16,
    /// Where each button was drawn, for clicks.
    buttons: Buttons,
}

impl Panel {
    pub fn open() -> Self;
    /// Esc and clicks outside close; buttons pick their action; scrolling
    /// moves the notes.
    pub fn handle(&mut self, event: &Event, tracker: &Tracker) -> Picked;
    /// Status row from the tracker, notes through tui-markdown.
    pub fn draw(&mut self, frame: &mut Frame, screen: Rect, tracker: &Tracker);
}
```

#### `install.sh` (slice 6)

POSIX `sh`. Maps `uname -s`/`uname -m` to `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `aarch64-apple-darwin`, and `x86_64-apple-darwin` if slice 6 publishes it. Downloads `releases/latest/download/ship-<target>.tar.gz` and its `.sha256`, verifies with `sha256sum` or `shasum -a 256`, and installs to `~/.local/bin`. Exits non-zero with a reason on an unknown platform, a missing asset or a mismatch, and prints a PATH hint when needed (FR-021).

### Replaced

- **`crates/ship/src/main.rs` `health()`** (slice 1): the error text becomes `updates::mismatch(&response)`. The check itself is unchanged.
- **`crates/ship/src/cli.rs` `#[command(version)]`** (slice 1): `version = concat!(env!("CARGO_PKG_VERSION"), " (protocol ", <PROTOCOL_VERSION>, ")")`. `PROTOCOL_VERSION` is a `u32` const, so it needs a const string beside it in `ship-core` or a `build.rs`-free formatting trick; the exact form is a slice 1 detail.
- **`crates/ship/src/cli.rs` `Command`** (slices 2 and 3): gains `Update(UpdateArgs)` with `--check`.
- **`crates/ship/src/main.rs` `dispatch`** (slices 2 to 4): routes `Command::Update`, builds the `UpdateContext` from `runlog::state_dir()` and `cfg!(feature = "self-update")`, and passes it to `ui::run`.
- **`crates/ship-client/src/ui/mod.rs`** (slices 4 and 5): `run` takes `UpdateContext` and reads `/health` once before `drive`. `Local` gains `updates: Tracker`. `drive`'s `select!` gains three branches in the existing style: `() = until(local.updates.due())` starts a scheduled check, `result = finish(local.updates.pending_check())` stores it and puts any one-time note in `local.message`, and the pending tab creation is polled the same way. `reconcile` calls `updates.reconcile(&replica.tabs)`. `reload` calls `updates.configure`. `Overlay` gains `Updates(Panel)`, read through the same `Picked` match as the others. `act` handles `ClientAction::Updates`: `Open` sets the overlay, `Check` and `Update` call the tracker. The update creates tab `update` with `create_tab`, then its first pane with `create_pane` (`command = ["sh", "-c", "exec \"$SHIP_BIN\" update"]`, `exit_behavior = "keep"`, `select = true`), the way the pane menu's `edit scrollback` runs `$SHIP_BIN`.
- **`crates/ship-client/src/ui/sidebar/mod.rs`** (slices 4 and 5): `draw` takes `version: VersionRow`, reserves the bottom row, and draws it dimmed or in the accent color. `hit` returns a new `Hit::Version` there.
- **`crates/ship-client/src/ui/mouse.rs`** (slice 5): a left click on `Hit::Version` opens the panel, like `Hit::Button` opens the global menu.
- **`crates/ship-client/src/keymap.rs`** (slices 2 and 5): `ClientSettings` gains `updates: UpdateSettings`, parsed from a new `UpdatesFile` in `Client` with `deny_unknown_fields`. `ClientAction` gains `Updates(ClientUpdates)`, with `Open {}`, `Check {}` and `Update {}` spelled `client.updates.open`, `client.updates.check` and `client.updates.update`, like `ConfigAction::Reload {}`.
- **`crates/ship-client/src/lib.rs`** (slice 1): `pub mod updates;`.

### Proposed settings

`crates/ship-core/src/defaults.toml` (slices 2 and 5):

```toml
[client.updates]
check = true                         # bool; check at startup and hourly
# command = "brew upgrade ship"      # what ship update runs instead of the build's method
# latest_command = "..."             # prints the latest version instead of asking GitHub

# in [client.menus] global, before "reload config":
  { label = "updates", action = { client.updates.open = {} } },
```

Manifests (slices 1, 3 and 5):

```toml
# crates/ship-client/Cargo.toml
semver = "1"
tui-markdown = { version = "0.3", default-features = false }

# crates/ship/Cargo.toml
[features]
self-update = ["dep:self_update"]

[dependencies]
self_update = { version = "1.3", optional = true, default-features = false, features = ["reqwest", "rustls", "github", "compression-tar-gz", "checksums"] }
```

`semver` is already in `Cargo.lock` through other crates. If `self_update`'s `checksums` feature doesn't read the `.sha256` files `upload-rust-binary-action` writes, `update.rs` verifies them itself with `sha2` (architecture risk).

`.github/workflows/release.yml` (slices 3 and 6): `features: self-update` on the upload step, and the `x86_64-apple-darwin` row on `macos-15` if it cross-compiles (OU-3).

### Untouched

- `crates/ship-server`: no change. The update pane is an ordinary pane.
- `crates/ship-core/src/protocol.rs` and `command.rs`: no new messages, fields or protocol bump.
- `vendor/ratatui-ghostty`, `ship-macros`.
- `release-plz.toml`: it already creates the GitHub release with the changelog as its body.

### Blocking unknowns

- **OU-3** (Intel macOS) blocks only slice 6's extra release row; it's settled by trying it.

## Build order

Checks run on the machine doing the work. A check named for a platform that machine isn't is reported unverified there, per `AGENTS.md`.

### 1. Versions and the mismatch error

Files: `ship-client/src/lib.rs`, `ship-client/src/updates/mod.rs` (only `core_version`, `installed`), `updates/probe.rs`, `ship/src/cli.rs`, `ship/src/main.rs`, `ship-client/Cargo.toml` (`semver`).

Delivers: `ship --version` prints `ship 0.1.0 (protocol 21)`. A client attaching to a server with another protocol fails with both versions, both protocols and the restart command.

Checks: run `ship --version`. Start a server from `main`, bump `PROTOCOL_VERSION` locally, run bare `ship` against it, and read the error. `cargo clippy` and `cargo test` pass.

### 2. `ship update --check`, config and cache

Files: `updates/mod.rs` (settings, `method`, build-time consts), `updates/check.rs`, `ship-client/src/keymap.rs` (settings only), `ship-core/src/defaults.toml` (`[client.updates]`), `ship/src/update.rs` (`check`), `ship/src/cli.rs`, `ship/src/main.rs`.

Delivers: `ship update --check` prints `no releases yet` against `shipmux/ship` today, and `0.1.0 → 9.9.9` with `latest_command = "echo 9.9.9"`. It writes `<state dir>/update.json`. `ship config check` accepts `[client.updates]` and rejects unknown keys in it.

Checks: those three runs; delete and corrupt the cache file and confirm both act as "no cache". Linux and macOS.

### 3. `ship update`

Files: `ship/src/update.rs` (`update`, `self_replace`), `ship/Cargo.toml` (feature), `ship/src/main.rs`, `release.yml` (`features: self-update`).

Delivers: with `command = "echo updating"`, `ship update` runs it and prints the sync message from the `PATH` probe. A plain `cargo build` prints "this install can't update itself" and exits non-zero. A `--features self-update` build with `SHIP_RELEASE_REPO` pointed at a fork's two releases replaces itself, and `ship --version` shows the new version.

Checks: the three runs above on Linux and macOS, plus a deliberately wrong `.sha256` on the fork's release that leaves the binary unchanged. `cargo build` without the feature doesn't compile `self_update` (`cargo tree -p ship -e features`).

### 4. The notice in the client

Files: `ui/updates/mod.rs`, `ui/mod.rs` (tracker in `Local`, `select!` branches, `run` signature), `ui/sidebar/mod.rs` (row), `ship/src/main.rs` (`UpdateContext`).

Delivers: the sidebar's bottom row shows `v0.1.0` dimmed. With `latest_command = "echo 9.9.9"`, it changes to `v0.1.0 → v9.9.9` in the accent color soon after startup, and the mode bar shows `ship v9.9.9 is available · ≡ updates` once. Restarting the client doesn't show the note again. `check = false` keeps the row dimmed and sends no request. CLI commands never check.

Checks: those runs; startup feel with checks on versus off; a second client within the hour reuses the cache (no second request in the client log); config reload toggles `check` live.

### 5. The panel

Files: `ui/updates/panel.rs`, `ui/mod.rs` (`Overlay::Updates`, `act`, the update request turn, `reconcile`), `ui/mouse.rs` (`Hit::Version`), `keymap.rs` (`client.updates.open`), `defaults.toml` (menu item), `ship-client/Cargo.toml` (`tui-markdown`).

Delivers: `≡ → updates` and clicking the sidebar row open the panel, with versions, check time, `[check]`, `[update]` and rendered notes. `[update]` opens tab `update` running `ship update`. The panel follows it to installed (with the restart answer) or failed, and back to available if the tab is closed. With method `None`, `[update]` is disabled with the reason. With `check = false`, it says checks are off and `[check]` still works.

Checks: walk every status in FR-011 by hand using `latest_command`, `command = "false"`, `command = "sleep 5"` and a plain build. Notes render from a real release body on the fork. Esc and clicks outside close it like the menu.

### 6. Install, releases and docs

Files: `install.sh`, `release.yml` (Intel row if OU-3 passes), `README.md` (install command), `docs/site/configuration.md` (`[client.updates]`), `docs/site/building-from-source.md` (updates paragraph), and the website's download count (`website/src/landing/`, under Details in the PR).

Delivers: the README's `curl -fsSL https://raw.githubusercontent.com/shipmux/ship/main/install.sh | sh` installs on Linux and macOS arm64, and refuses on other platforms with a link to building from source. The Intel macOS archive is published if it cross-compiles and runs under Rosetta.

Checks: run the script against the fork's release on this Mac and a clean Linux user account. On an unsupported `uname -m` (faked), it refuses. Try the `x86_64-apple-darwin` build and run it under Rosetta (`arch -x86_64 ./ship --version`). Docs follow `docs/agents/docs.md`. The release workflow's dry run passes.

## Deviation log

### Slice 1: versions and mismatch guidance

- `cli.rs` formats clap's static version string once with `std::sync::LazyLock<String>`, using the existing numeric `PROTOCOL_VERSION`. This is the build-script-free formatting choice left to slice 1; no string constant or other change was added to `ship-core`.
- Verification on macOS: `ship --version` printed exactly `ship 0.1.0 (protocol 21)` and created no server or runtime files. `cargo tree -d` was unchanged. A temporary Rust executable checked release tags, packager suffixes, malformed versions, a real binary probe, failed probes, all three sync states and restart guidance; no permanent tests were added.
- An owned protocol-21 server used isolated temporary XDG directories. After a temporary protocol-22 build, bare `ship` (with `SHIP_SERVER_NAME`), explicit name/URL targets and `server stop` each exited 1 with both labeled versions/protocols and the restart command and cost. The original server remained healthy. The constant was restored to 21, the binary rebuilt, and the owned server stopped with its matching binary.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` and `openspec validate self-update --type change --strict` passed. The health-exchange delta and local-foundation version requirements match the implementation. Linux remains unverified.
