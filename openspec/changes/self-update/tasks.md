# Tasks

The six groups are `design/program.md`'s build slices, shipped in one pull request. The paper's skeleton map gives the types and signatures; record anything that differs in its deviation log. The delta specs under `specs/` were written ahead of the slices, so each slice checks that its deltas still match what it built and doesn't write them.

The paper puts all docs in slice 6. Here each slice updates the page its own behavior touches: `configuration.md` in slice 2, `building-from-source.md` in slice 3, the global menu table in slice 5, and the README in slice 6. The files and their content are the paper's.

Verification uses real commands and a running client, not permanent authored tests. Use an owned temporary server on its own socket, with `XDG_STATE_HOME`, `XDG_CONFIG_HOME` and `XDG_RUNTIME_DIR` pointed at temporary directories, and don't use any server that's already running. Most checks use `latest_command` and `command` so they need no network. Read-only requests to `api.github.com` are fine. Ask Cyan before creating a fork, publishing releases or pushing anywhere. Every code group ends with `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` and `openspec validate self-update --type change --strict`. Checks run on macOS; Linux is unverified unless it runs there.

## 1. Versions and the mismatch error

- [ ] 1.1 Add `semver` to `crates/ship-client/Cargo.toml`, `pub mod updates;` to `lib.rs`, and `updates/mod.rs` with only `core_version` and `installed`. Verify that `cargo tree -d` shows no new duplicate crates.
- [ ] 1.2 Add `crates/ship-client/src/updates/probe.rs` with `Probed`, `Sync`, `probe`, `Sync::of`, `Sync::describe` and `mismatch`.
- [ ] 1.3 In `crates/ship/src/cli.rs`, make the version string `ship 0.1.0 (protocol 21)`, choosing how to get `PROTOCOL_VERSION` into a const string (record the choice in the deviation log if it adds anything to `ship-core`). Verify that `ship --version` prints exactly that and starts no server.
- [ ] 1.4 In `crates/ship/src/main.rs` `health()`, report a protocol mismatch with `updates::mismatch`. Verify by starting an owned server from this build, bumping `PROTOCOL_VERSION` locally, and running bare `ship` against it: the error names both versions, both protocols and the restart command, exits 1, and leaves the server running.
- [ ] 1.5 Check that the `health-exchange` delta and the version part of the `local-foundation` delta match what was built.

## 2. `ship update --check`, config and cache

- [ ] 2.1 Finish `updates/mod.rs`: `UpdateSettings`, `UpdateContext`, the build-time consts, `Method`, `method`, `latest_command` and `shell`.
- [ ] 2.2 Add `updates/check.rs`: `Freshness`, `Release`, `Found`, `Checked` and `Checker`, using the workspace `reqwest` with a `User-Agent` and writing the cache by temp file and rename.
- [ ] 2.3 In `crates/ship-client/src/keymap.rs`, parse `[client.updates]` through `UpdatesFile` with `deny_unknown_fields` into `ClientSettings::updates`, and add the table to `crates/ship-core/src/defaults.toml`. Verify that `ship config check` accepts the defaults and reports `chek = false` with its line and column, and that `ship config defaults` prints the table with its comments.
- [ ] 2.4 Add `Command::Update(UpdateArgs)` with `--check` in `cli.rs`, `update::check` in `crates/ship/src/update.rs`, and its route in `dispatch`. Verify `no releases yet` against `shipmux/ship`, `0.1.0 → 9.9.9` with `latest_command = "echo 9.9.9"`, and `up to date` with `latest_command = "echo 0.1.0"`, and that `<state dir>/update.json` is written.
- [ ] 2.5 Delete the cache, then corrupt it. Verify that both runs behave as if there were no cache and rewrite a valid file.
- [ ] 2.6 Following `docs/agents/docs.md`, document `[client.updates]` in `docs/site/configuration.md`. Verify that every key and default matches `defaults.toml`.
- [ ] 2.7 Check that the `config-file` delta's Update settings and the `updates` delta's Where the latest version comes from and Check from the CLI match what was built.

## 3. `ship update`

- [ ] 3.1 Add the `self-update` feature and the optional `self_update` dependency to `crates/ship/Cargo.toml`. Verify with `cargo tree -p ship -e features` that a plain build doesn't compile `self_update` and a `--features self-update` build does.
- [ ] 3.2 Add `update::update` and `#[cfg(feature = "self-update")] self_replace` to `crates/ship/src/update.rs`, and route `ship update` in `dispatch`. Find out whether `self_update`'s `checksums` feature reads `upload-rust-binary-action`'s `.sha256` files; if it doesn't, verify with `sha2` in `update.rs` and record that in the deviation log.
- [ ] 3.3 Verify that with `command = "echo updating"`, `ship update` runs it and prints the sync message from the `PATH` probe, and that with no server running it prints no server line and starts none.
- [ ] 3.4 Verify that a plain `cargo build` binary prints that this install can't update itself and how to update it, and exits 1.
- [ ] 3.5 Add `features: self-update` to the upload step in `.github/workflows/release.yml`. With Cyan's go-ahead on a fork and two test releases, verify that a `--features self-update` build with `SHIP_RELEASE_REPO` set to the fork replaces itself and `ship --version` shows the newer version. Then verify that a deliberately wrong `.sha256` leaves the binary unchanged and says why.
- [ ] 3.6 Following `docs/agents/docs.md`, add the updates paragraph to `docs/site/building-from-source.md`: source builds don't replace themselves, and packagers set `SHIP_UPDATE_COMMAND` and `SHIP_LATEST_COMMAND`.
- [ ] 3.7 Check that the `updates` delta's Update method, No update method and Sync after an update match what was built.

## 4. The notice in the client

- [ ] 4.1 Add `crates/ship-client/src/ui/updates/mod.rs` with `Run`, `Tracker` and `VersionRow`, without the panel's half (`update`, `pending_start`, `started`).
- [ ] 4.2 In `crates/ship-client/src/ui/mod.rs`, have `run` take `UpdateContext` and read `/health` once, add `Local::updates`, add the `until(due)` and `finish(pending_check)` branches to `drive`, put an announced note in `local.message`, and call `updates.configure` on reload. In `crates/ship/src/main.rs`, build the `UpdateContext` and pass it in.
- [ ] 4.3 In `crates/ship-client/src/ui/sidebar/mod.rs`, have `draw` take `VersionRow` and draw it on the bottom row, dimmed or in the accent color.
- [ ] 4.4 Verify in a running client: the row shows `v0.1.0` dimmed; with `latest_command = "echo 9.9.9"` it changes to `v0.1.0 → v9.9.9` soon after startup, and the mode bar shows `ship v9.9.9 is available · ≡ updates` once; restarting the client and attaching a second one doesn't show the note again; with the sidebar hidden the note still shows.
- [ ] 4.5 Verify that `check = false` keeps the row dimmed and runs no check, that saving `check = true` makes one due at once, that a second client within the hour reuses the cache with no second request in the client log, and that `ship tab create` makes no request.
- [ ] 4.6 Compare attach time with checks on and with `check = false` (SC-002), with `latest_command` set to a command that sleeps, and record both.
- [ ] 4.7 Check that the `terminal-client` delta and the `updates` delta's checking, sidebar row and one-time note requirements match what was built.

## 5. The panel

- [ ] 5.1 Add `tui-markdown` to `crates/ship-client/Cargo.toml` with default features off. Verify with `cargo tree -d` that it adds no second ratatui or crossterm.
- [ ] 5.2 Add `crates/ship-client/src/ui/updates/panel.rs` and the tracker's `update`, `pending_start`, `started` and `reconcile`.
- [ ] 5.3 Wire the panel in: `ClientUpdates` (`open`, `check`, `update`) in `keymap.rs`; `Overlay::Updates`, the `act` arms, the `finish(pending_start)` branch and `updates.reconcile` in `ui/mod.rs`; `Hit::Version` in `ui/sidebar/mod.rs` and `ui/mouse.rs`; the `updates` global menu item in `defaults.toml`.
- [ ] 5.4 Verify that `≡ → updates`, clicking the version row and a custom `client.updates.open` binding each open the panel, and that Esc and a click outside close it like the menu. Verify that `alt-m` still reaches the program.
- [ ] 5.5 Walk every FR-011 status by hand: up to date and available with `latest_command`; checking with a slow `latest_command`; check failed with a failing one; checks off with `check = false`, then `[check]`; updating, installed and failed with `command = "sleep 5"`, `command = "true"` and `command = "false"`; back to available after closing the `update` tab mid-run; no method in a plain build, with `[update]` disabled and the reason shown.
- [ ] 5.6 Verify that the notes render from a real release body (the fork's from 3.5, or a read-only fetch of another repository's release in a scratch build), that they scroll, and that each URL is shown.
- [ ] 5.7 Following `docs/agents/docs.md`, add `updates` to the global menu row in `docs/site/scrolling-copying-and-the-mouse.md`.
- [ ] 5.8 Check that the `keymap`, `context-menus` and `mouse-input` deltas and the `updates` delta's panel requirements match what was built.

## 6. Install, releases and docs

- [ ] 6.1 Add `install.sh`. Verify with `shellcheck` and `sh -n`, then on this Mac against a test release: it installs to a temporary `HOME`'s `~/.local/bin` and prints the `PATH` hint. A faked unsupported `uname -m` refuses with a link to building from source, and a wrong checksum installs nothing. Report the clean Linux account run as unverified unless it was done there.
- [ ] 6.2 Try `x86_64-apple-darwin` on the arm64 runner's toolchain locally and run it with `arch -x86_64 ./ship --version` (OU-3). If it works without real fixes, add the row to `release.yml`; either way, record the outcome in the deviation log and settle OU-3 with Cyan.
- [ ] 6.3 Run the release workflow's dry run, or the closest local equivalent, and verify that the asset names match what `install.sh` and `self_replace` expect.
- [ ] 6.4 Following `docs/agents/docs.md`, add the install command to `README.md`, and add the download count to `website/src/landing/`. Verify with `pnpm --dir website build`.
- [ ] 6.5 Check that the `installation` delta and the README part of the `local-foundation` delta match what was built, and report implementation Rust and test line counts separately.

## Workflow follow-up

- Open one pull request from `.github/pull_request_template.md`, with the website download count under Details and a tracking issue for any decision left open.
- After merge, sync the deltas into `openspec/specs/` and archive the change.
