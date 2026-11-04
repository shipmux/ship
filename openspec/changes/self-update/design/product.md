# Update notices, an updates panel and `ship update`

Status: Locked on 2026-11-02. Written from design conversations with Cyan on 2026-11-01 and 2026-11-02 for [#36](https://github.com/shipmux/ship/issues/36), then approved in Plannotator review ("LGTM"). The open unknowns below stay open until Cyan decides them.

## Approved amendment: open unknowns settled

On 2026-11-02, Cyan approved these in chat ("those defaults sound good to me"). They settle OU-1, OU-2 and OU-4 and refine FR-010. OU-3 stays open and is settled during the build.

- **OU-1.** When a check first finds a release newer than any it has noted before, the mode bar shows a one-time note: `ship v0.2.0 is available · ≡ updates`. It shows once per release across every client on the machine. There is no dot on `≡`; the sidebar row carries the notice while the sidebar is shown.
- **OU-2.** After a config or packager command succeeds, Ship runs `ship --version` from `PATH` and compares its protocol with the running server's. If that fails, it prints the generic message: the server may need a restart to stay in sync, with the command and its cost.
- **OU-4.** A build-time override names the GitHub repository Ship checks and updates from, defaulting to `shipmux/ship`. Self-replace is tested against a fork's releases. Release builds never set it.
- **Notes source (refines FR-010).** Notes always come from the GitHub release whose tag matches the latest version's `major.minor.patch`, whatever reported that version, so a packager's `0.2.0-1` reads `v0.2.0`'s notes. With no matching release, the panel shows versions only. There is no notes override.

## Goal

Ship can't tell you a newer version exists, and it can't update itself. That has to change before v0.1 ships, or v0.1 users never hear about v0.2. This change adds a background check, a version in the sidebar's corner, an updates panel with release notes, `ship update`, and an install script for first installs. The product brief puts self-update in the later multi-server direction. This change brings forward the local part of it and leaves remote servers ([#88](https://github.com/shipmux/ship/issues/88)) and restarting from the client ([#89](https://github.com/shipmux/ship/issues/89)) where they are.

What you see when an update is waiting:

```text
┌──────────────────┬──────────────────────────────────────┐
│ tabs           ≡ │ ~/web $ npm run dev                  │
│▌web              │   ready on :3000                     │
│ api              │                                      │
│                  │                                      │
│ v0.1.0 → v0.2.0  │                                      │  ← accent color; click opens the panel
└──────────────────┴──────────────────────────────────────┘
```

The panel, opened from that corner or from `≡ → updates`:

```text
┌ updates ──────────────────────────────────────────── esc ┐
│ client   v0.1.0                                          │
│ server   v0.1.0                                          │
│ latest   v0.3.0                   checked 2h ago [check] │
│          the server restarts afterwards to stay in sync  │
│                                                [update]  │
│──────────────────────────────────────────────────────────│
│ v0.3.0 · 2026-11-20                         full notes ↗ │
│   Features                                               │
│   - floating panes                                       │
│ v0.2.0 · 2026-11-09                         full notes ↗ │
│   Bug Fixes                                              │
│   - tab switch flicker                                   │
└──────────────────────────────────────────────────────────┘
```

How `ship update` picks its method, first match wins:

```text
1. [client.updates] command     your override
2. SHIP_UPDATE_COMMAND          set by a packager at build time
3. self-replace                 only binaries built by Ship's release workflow
4. none                         says so, and how to update the way you installed
```

The conventions behind this:

- Every tool we compared (Ghostty, Zed, WezTerm, gh, uv, rustup, deno, bun, Herdr, Claude Code) checks on its own and has a switch to turn that off. None installs silently. Herdr tried silent installs and backed out.
- `ship update` and `ship update --check` follow `uv self update`, `rustup self update`, `herdr update` and `claude update`. Claude Code sends package-manager installs to the package manager, the way method 4 does.
- Only release-workflow builds replace their own binary. A Nix, distro or `cargo install` binary that overwrites itself fights whatever installed it.

## User stories

### P1: Notice that an update exists

You keep a Ship client open for days. A new release comes out, and within an hour the sidebar's corner shows `v0.1.0 → v0.2.0` without you doing anything.

- Given a TUI client is attached and a newer release exists, when the next check runs, then the sidebar's bottom row shows the installed and latest versions in the accent color.
- Given no newer release exists, when you look at the sidebar, then its bottom row shows the installed version, dimmed.
- Given the check fails (offline, rate-limited), when it fails, then nothing on screen changes, and the next check retries an hour later.
- Given three clients are attached and one checked 10 minutes ago, when a fourth starts, then it reuses that result and sends no request.
- Given you run `ship tab create` or any other CLI command, when it runs, then it makes no update request.

Tested independently: point `[client.updates] latest_command` at a script printing a higher version, start the client, and watch the corner change.

### P2: Update from the panel or the CLI

You see the corner change, open the panel, read what's new, and click `[update]`.

- Given a release build and a newer release, when you click `[update]`, then a tab named `update` opens running `ship update`, and the panel shows `updating in tab "update"…`.
- Given that tab's program exits successfully, when you look at the panel, then it shows `installed v0.2.0` and whether the server must restart.
- Given it fails, when you look at the panel, then it shows `update failed · see tab "update"`.
- Given you set `[client.updates] command = "brew upgrade ship"`, when you run `ship update`, then Ship runs that command and does not replace its own binary.
- Given a source build with no command configured, when you open the panel, then `[update]` is disabled with `update with your package manager`, and `ship update` prints the same and exits with an error.
- Given you run `ship update --check`, when it finishes, then it prints `v0.1.0 → v0.2.0` or `up to date` and changes nothing.

Tested independently: in a release build pointed at a test release, run `ship update` and compare the binary's version before and after.

### P3: Keep the client and server in sync

The client and server are the same binary, but an update only replaces the file. The running server keeps running the old code.

- Given `ship update` installed a version whose protocol differs from the running server's, when it finishes, then it prints the new version, the server's version, and `restart the server: ship server stop && ship (ends running programs; tabs are restored)`.
- Given the protocols match, when it finishes, then it says the server is compatible and can restart when convenient.
- Given an updated client attaches to an old server, when the health check fails, then the error names both versions and protocols and gives the same restart command.
- Given the client and server versions differ, when you open the panel, then the server row says to restart the server to match.

Tested independently: start a server from one build, attach with a build that has a bumped protocol, and read the error.

### P4: Install Ship for the first time

- Given Linux x86_64, Linux arm64 or macOS arm64 with curl, when you run the install command from the README, then the latest release is downloaded, its checksum is verified, and `ship` is installed to `~/.local/bin`.
- Given `~/.local/bin` isn't on `PATH`, when the install finishes, then the script says how to add it.
- Given a platform without a prebuilt binary, when you run it, then it says so and links to building from source, and installs nothing.
- Given the checksum doesn't match, when it runs, then it installs nothing and says why.

Tested independently: run the script on a clean Linux user account and on this Mac.

### P5: Turn checks off or check by hand

- Given `[client.updates] check = false`, when a client runs, then it never contacts GitHub on its own, and the panel says `update checks are off` and still offers `[check]`.
- Given you click `[check]`, when it finishes, then the panel shows the result and the time of the check.

Tested independently: set `check = false`, attach, and confirm the client makes no request.

## Non-goals

- Installing updates without being asked.
- Remote servers and updating a server on another machine ([#88](https://github.com/shipmux/ship/issues/88)).
- Restarting the server from the client ([#89](https://github.com/shipmux/ship/issues/89)). The panel shows the command instead.
- Protocol compatibility across versions, compatibility floors, and keeping programs alive across an upgrade.
- A settings screen or a tabbed menu. The updates panel is a standalone overlay.
- A default key for the panel or for `≡`. `alt-m` stays unbound until the tabbed-menu question is settled.
- A "what's new" popup or note after an update.
- Skipping a version.
- Publishing the protocol with each release so the panel knows ahead of time whether a restart is needed.
- Self-replace for binaries not built by Ship's release workflow.
- Prebuilt binaries for musl Linux and Windows.
- The website's download count. It ships in the same pull request under Details but isn't part of this paper.

## Functional requirements

### Checking

- FR-001: The TUI client MUST check for the latest release when it starts and every hour while it runs. CLI commands MUST NOT check.
- FR-002: A check MUST be skipped when any client on the machine checked within the last hour; the client then uses that check's result.
- FR-003: A check MUST NOT delay startup or input, and a failed check MUST NOT show anything; the next scheduled check retries.
- FR-004: `[client.updates] check = false` MUST stop all automatic checks. `[check]` in the panel and `ship update --check` MUST still work.
- FR-005: The latest version MUST come from the first of: `[client.updates] latest_command`, `SHIP_LATEST_COMMAND` set at build time, the newest non-draft, non-prerelease GitHub release of `shipmux/ship`.

### Notice

- FR-006: The sidebar's bottom row MUST show the client's version, dimmed when up to date and as `installed → latest` in the accent color when a newer release exists.
- FR-007: Clicking that row MUST open the updates panel.
- FR-008: The global menu (`≡`) MUST have an `updates` item that opens the panel. Opening the panel MUST be an action that keys and menus can bind; no key is bound by default.

### Panel

- FR-009: The panel MUST show the client version, the server version, the latest version, when the last check ran, `[check]`, and `[update]`.
- FR-010: The panel MUST show release notes for every release newer than the client, newest first, rendered from markdown, each with a link to its GitHub page. When up to date, it MUST show the client's own release notes.
- FR-011: The panel's status MUST cover: up to date, checking, available, updating, installed (with whether a restart is needed), failed, check failed, checks off, and no update method (with `[update]` disabled and the reason).
- FR-012: Before an update, the panel MUST say the server restarts afterwards to stay in sync. When the client and server versions differ, the server row MUST say to restart the server to match.
- FR-013: `[update]` MUST open a tab named `update` running `ship update` and follow that program to success or failure.

### `ship update`

- FR-014: `ship update` MUST use the first available method: `[client.updates] command`, `SHIP_UPDATE_COMMAND`, self-replace in release-workflow builds, or none.
- FR-015: Self-replace MUST verify the downloaded archive against its published `.sha256` before replacing anything.
- FR-016: With no method, `ship update` MUST print that this install can't update itself and how to update it, and exit with an error.
- FR-017: After updating, `ship update` MUST state the installed version and whether the running server must restart, with the restart command and its cost. (See OU-2 for the command method.)
- FR-018: `ship update --check` MUST print `installed → latest` or `up to date` and change nothing.
- FR-019: `ship --version` MUST print the protocol version alongside the release version.

### Sync

- FR-020: When the client's health check finds a different protocol, the error MUST name both versions and both protocols and give the restart command and its cost.

### Install and releases

- FR-021: An install script MUST download the latest release for the current platform, verify its checksum, and install `ship` to `~/.local/bin`. It MUST refuse to install on a checksum mismatch or an unsupported platform, and say why.
- FR-022: The script MUST be served from `raw.githubusercontent.com/shipmux/ship/main/install.sh` until [#64](https://github.com/shipmux/ship/issues/64) picks a domain.
- FR-023: Releases MUST include Linux x86_64, Linux arm64 and macOS arm64. Intel macOS is added only if it cross-compiles and runs under Rosetta without real fixes (OU-3).

### Docs

- FR-024: The README MUST give the install command, and the configuration page MUST document `[client.updates]`.

## Success criteria

- SC-001: With a newer release published, an attached TUI client shows it in the sidebar within an hour and no sooner than its next scheduled check.
- SC-002: Attach time with checks on is within noise of attach time with `check = false`.
- SC-003: Across any number of clients and CLI calls, a machine sends at most one automatic update request per hour.
- SC-004: A release build updates itself from the panel, and afterwards `ship --version` reports the new version, on Linux and macOS.
- SC-005: The install script works on a clean Linux account and on macOS arm64.
- SC-006: An updated client attaching to an old server gets an error that names the restart command.

## Open unknowns

- OU-1: **The notice when the sidebar is hidden.** `≡` sits in the sidebar, so both the corner and a dot on `≡` vanish with `alt-b`. That leaves no notice and no way to open the panel from the UI. Recommendation: also show a one-time mode-bar note when a check first finds a new release, as approved in round 1 (Q2), and drop the dot, since the corner already carries the notice while the sidebar is shown. Blocks FR-006 to FR-008.
- OU-2: **Restart guidance after a packager or config command.** Ship doesn't install the binary itself, so it may not know which binary the command installed. Recommendation: run `ship --version` on the `ship` found on `PATH` afterwards and compare protocols; if that fails, print the generic sync message. Blocks FR-017 for methods 1 and 2.
- OU-3: **Intel macOS.** Whether the Zig-built Ghostty dependency cross-compiles from the arm64 runner hasn't been tried. Settled during the build by trying it. Blocks FR-023's Intel row only.
- OU-4: **Testing self-replace before a second release exists.** v0.1 is the first release, so there is nothing newer to update to. Recommendation: a prerelease or a fork's release, via a build-time repo override, so FR-005's "non-prerelease" rule still holds for real users. Blocks SC-004.
