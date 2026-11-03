# Proposal

Status: Locked through its design papers. `design/product.md`, `design/architecture.md` and `design/program.md` are locked with Cyan's recorded approval, and they are the contract. This proposal and the deltas under `specs/` are views assembled from them.

## Why

Ship can't tell you a newer version exists, and it can't update itself ([#36](https://github.com/shipmux/ship/issues/36)). That has to change before v0.1 ships, or v0.1 users never hear about v0.2. First installs also mean building from source today, since there is no install script.

## What Changes

- **Background checks.** The full-screen client asks for the latest release at startup and every hour. Every client on the machine shares one cached result, so a machine sends at most one automatic request an hour. CLI commands never check. `[client.updates] check = false` turns the schedule off.
- **A notice in the sidebar's corner.** The sidebar's bottom row shows the installed version, dimmed, or `v0.1.0 → v0.2.0` in the accent color when a newer release exists. The first time a check finds a given release, the mode bar shows a one-time note: `ship v0.2.0 is available · ≡ updates`.
- **An updates panel**, opened by clicking that row, from `≡ → updates`, or with the new `client.updates.open` action, which has no default key. It shows the client, server and latest versions, when the last check ran, `[check]`, `[update]`, and rendered release notes for every newer release. `[update]` runs `ship update` in a new tab named `update` and follows it to success or failure.
- **`ship update` and `ship update --check`.** The update method is the first of: `[client.updates] command`, the packager's build-time `SHIP_UPDATE_COMMAND`, or self-replace (only in binaries built by Ship's release workflow, with the checksum verified). With none of these, Ship says how to update and exits with an error. Afterwards it says whether the running server must restart to stay in sync, with the command and its cost.
- **Version and protocol in messages.** `ship --version` prints `ship 0.1.0 (protocol 21)`. A client that attaches to a server speaking another protocol gets an error naming both versions and protocols and the restart command.
- **An install script and release targets.** `install.sh` installs the latest release to `~/.local/bin` after checking its checksum. Release builds turn on self-replace. Intel macOS is added only if it cross-compiles and runs under Rosetta.

Not in this change: installing without being asked, remote servers ([#88](https://github.com/shipmux/ship/issues/88)), restarting the server from the client ([#89](https://github.com/shipmux/ship/issues/89)), protocol compatibility across versions, a settings screen or tabbed menu, a default key for the panel, a what's-new popup, skipping a version, self-replace for builds from outside the release workflow, and musl or Windows binaries.

## Capabilities

### New Capabilities

- `updates`: checking for releases, the sidebar notice and one-time note, the updates panel, `ship update`, and the sync message after an update.
- `installation`: the install script and the platforms releases publish.

### Modified Capabilities

- `health-exchange`: the protocol-mismatch error names both versions and protocols and gives the restart command.
- `local-foundation`: `ship --version` prints the protocol, help lists `ship update`, and the README gives the install command.
- `config-file`: the new `[client.updates]` table and how it reloads.
- `keymap`: the `client.updates.*` actions, where `client.updates.update` is a client action that creates a tab.
- `context-menus`: the default global menu gains `updates`.
- `mouse-input`: clicking the sidebar's version row opens the panel.
- `terminal-client`: the sidebar holds a version row under its sections, and the status line can show the one-time note.

## Impact

- `ship-client`: a new `updates/` module (settings, checking and its cache file, and probing a binary for its version), a new `ui/updates/` module (the tracker and the panel), and changes to `ui/mod.rs`, `ui/sidebar/mod.rs`, `ui/mouse.rs`, `keymap.rs` and `lib.rs`. New dependencies are `semver`, which is already in the lockfile, and `tui-markdown` 0.3 with its default features off.
- `ship` binary: a new `update.rs`, and changes to `cli.rs` and `main.rs`. A `self-update` cargo feature adds `self_update` 1.3, and only the release workflow enables it.
- `ship-core`: `defaults.toml`. The server, the protocol and `PROTOCOL_VERSION` don't change.
- Release and docs: a new `install.sh`, `.github/workflows/release.yml`, `README.md`, `docs/site/configuration.md` and `docs/site/building-from-source.md`. The website's download count (`website/src/landing/`) ships in the same pull request, listed under Details.
- Network: a running client sends `GET api.github.com/repos/shipmux/ship/releases` at most once an hour per machine, unless checks are off.
- Size: roughly 1,000 to 1,200 implementation lines, estimated, not measured. No tests are planned.
