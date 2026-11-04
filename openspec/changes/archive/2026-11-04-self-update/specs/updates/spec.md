# Spec Delta

## ADDED Requirements

### Requirement: Scheduled checks
A full-screen client SHALL check for the latest release when it starts and then every hour while it runs. A check SHALL NOT delay startup or input. CLI commands other than `ship update` SHALL NOT check. `[client.updates] check = false` SHALL stop the schedule, while the panel's `[check]` and `ship update --check` still work.

#### Scenario: Startup check
- **WHEN** a newer release exists and a user attaches a client with checks on
- **THEN** the client checks without slowing the attach, and the sidebar shows the newer release once the check returns

#### Scenario: CLI commands
- **WHEN** a user runs `ship tab create` or `ship server status`
- **THEN** no update request is made

#### Scenario: Checks off
- **WHEN** a client runs with `check = false`
- **THEN** it never asks for the latest release on its own

### Requirement: One check an hour per machine
Each check's result SHALL be shared by every client of the same user on the machine. A scheduled check SHALL reuse a result less than an hour old instead of asking again. `[check]` and `ship update --check` SHALL always ask. A shared result that is missing or can't be read SHALL count as no result.

#### Scenario: Fourth client
- **WHEN** three clients are attached, one checked 10 minutes ago, and a fourth starts
- **THEN** the fourth shows that result and sends no request

#### Scenario: Unreadable result
- **WHEN** the shared result is deleted or corrupted and a client starts
- **THEN** the client checks as if no client had checked

### Requirement: Failed checks
A failed check, for example offline or rate-limited, SHALL change nothing on screen except the panel's status, and the previous result SHALL still stand. The next scheduled check SHALL retry an hour later.

#### Scenario: Offline
- **WHEN** a client's check fails because the network is down
- **THEN** the sidebar and mode bar don't change, and the panel shows that the check failed and why

### Requirement: Where the latest version comes from
The latest version SHALL come from the first of: `[client.updates] latest_command`, the build-time `SHIP_LATEST_COMMAND`, or the newest release of `shipmux/ship` on GitHub that isn't a draft or prerelease. A command SHALL be a shell line whose output holds the version. A build-time override MAY name another repository; release builds don't set it.

#### Scenario: Latest command
- **WHEN** a user sets `latest_command = "echo 9.9.9"` and runs `ship update --check` on version 0.1.0
- **THEN** it prints `0.1.0 → 9.9.9` without asking GitHub for the version

#### Scenario: No releases yet
- **WHEN** the repository has no releases and a user runs `ship update --check`
- **THEN** it prints `no releases yet`

### Requirement: Release notes source
Release notes SHALL come from the GitHub release whose tag matches the latest version's `major.minor.patch`, whatever reported that version, so a packager's `0.2.0-1` shows `v0.2.0`'s notes. With no matching release, the panel SHALL show versions without notes.

#### Scenario: Packager suffix
- **WHEN** `latest_command` prints `0.2.0-1` and GitHub has a `v0.2.0` release
- **THEN** the panel shows `v0.2.0`'s notes

### Requirement: Sidebar version row
The sidebar's bottom row SHALL show the client's version, dimmed when up to date, or as `installed → latest` in the accent color when a newer release exists.

#### Scenario: Up to date
- **WHEN** no newer release exists and a client shows its sidebar
- **THEN** the bottom row shows `v0.1.0`, dimmed

#### Scenario: Newer release
- **WHEN** a check finds `v0.2.0` on a `v0.1.0` client
- **THEN** the bottom row shows `v0.1.0 → v0.2.0` in the accent color

### Requirement: One-time note
When a check first finds a release newer than any one-time note has shown before on the machine, the status line SHALL show `ship v<latest> is available · ≡ updates`. Each release SHALL be noted once across every client on the machine.

#### Scenario: Note once
- **WHEN** a check finds `v0.2.0`, then the user restarts the client and attaches a second one
- **THEN** the note showed once, before the restart, and neither later client shows it

#### Scenario: Hidden sidebar
- **WHEN** the sidebar is hidden and a check first finds `v0.2.0`
- **THEN** the note still shows on the status line

### Requirement: Opening the panel
Clicking the sidebar's version row, picking `updates` from the global menu, or running the `client.updates.open` action SHALL open the updates panel. Esc and a click outside it SHALL close it, like a menu.

#### Scenario: From the corner
- **WHEN** a user clicks the sidebar's bottom row
- **THEN** the updates panel opens

#### Scenario: Close
- **WHEN** the panel is open and the user presses Esc
- **THEN** it closes and the pane under it is unchanged

### Requirement: Panel contents
The panel SHALL show the client's version, the server's version, the latest version, when the last check ran, a `[check]` button and an `[update]` button. Below them it SHALL show release notes for every release newer than the client, newest first, rendered from markdown, each with its date and its GitHub URL. When up to date it SHALL show the client's own release notes. The notes SHALL scroll.

#### Scenario: Two newer releases
- **WHEN** the client is `v0.1.0` and `v0.2.0` and `v0.3.0` are released
- **THEN** the panel shows `v0.3.0`'s notes, then `v0.2.0`'s, each with its date and URL

### Requirement: Panel status
The panel's status SHALL say which of these holds: up to date, checking, update available, updating in tab `update`, installed with whether the server must restart, update failed, check failed, checks off, or no update method. With no method, `[update]` SHALL be disabled and the panel SHALL say how to update. With checks off, `[check]` SHALL still work.

#### Scenario: No method
- **WHEN** a source build with no `command` set opens the panel
- **THEN** `[update]` is disabled and the panel says `update with your package manager`

#### Scenario: Checks off
- **WHEN** `check = false` and the user opens the panel
- **THEN** it says `update checks are off`, and `[check]` shows the result and the time of the check

### Requirement: Sync guidance in the panel
Before an update, the panel SHALL say the server restarts afterwards to stay in sync. When the client's and server's versions differ, the server row SHALL say to restart the server to match.

#### Scenario: Versions differ
- **WHEN** a `v0.2.0` client is attached to a `v0.1.0` server and the user opens the panel
- **THEN** the server row says to restart the server to match

### Requirement: Update from the panel
`[update]` SHALL run `ship update` in a new selected pane that stays after its program exits. The pane SHALL go in the tab that held the last run's pane while that tab exists, and otherwise in a new top-level tab named `update`. The panel SHALL follow that pane: updating while it runs, installed with the restart answer after exit 0, update failed otherwise, and back to its earlier status if the tab closes before the run ends.

#### Scenario: Success
- **WHEN** a release build with a newer release clicks `[update]` and the program exits 0
- **THEN** a tab `update` shows the run, and the panel shows `installed v0.2.0` and whether the server must restart, which stays after the tab is closed

#### Scenario: Failure
- **WHEN** the update's program exits 1
- **THEN** the panel shows `update failed · see tab "update"`

#### Scenario: Retry after a failure
- **WHEN** a failed run's tab is still open and the user clicks `[update]` again
- **THEN** the new pane opens in that tab, and no second `update` tab appears

### Requirement: Update method
`ship update` SHALL use the first available method: `[client.updates] command`, the build-time `SHIP_UPDATE_COMMAND`, self-replace in builds from Ship's release workflow, or none. A command SHALL run as a shell line. Self-replace SHALL download this platform's archive from the latest release and verify it against its published `.sha256` before replacing anything.

#### Scenario: Configured command
- **WHEN** a user sets `command = "brew upgrade ship"` and runs `ship update`
- **THEN** Ship runs that command and doesn't replace its own binary

#### Scenario: Bad checksum
- **WHEN** a release's `.sha256` doesn't match its archive and a release build runs `ship update`
- **THEN** it fails, says why and leaves the binary unchanged

### Requirement: No update method
With no method, `ship update` SHALL print that this install can't update itself and to update it the way it was installed, and exit with an error.

#### Scenario: Source build
- **WHEN** a user runs `ship update` in a `cargo build` binary with no `command` set
- **THEN** it prints that this install can't update itself and exits 1

### Requirement: Sync after an update
After updating, `ship update` SHALL ask the installed binary for its version and protocol: the replaced binary after self-replace, or the `ship` on `PATH` after a command. When a server is running, it SHALL say whether it must restart, with `ship server stop && ship` and that this ends running programs while tabs are restored. If the probe fails, it SHALL say the server may need a restart. `ship update` SHALL NOT start a server.

#### Scenario: New protocol
- **WHEN** `ship update` installs a version whose protocol differs from the running server's
- **THEN** it prints the new version, the server's version, and `restart the server: ship server stop && ship (ends running programs; tabs are restored)`

#### Scenario: Same protocol
- **WHEN** the installed version speaks the running server's protocol
- **THEN** it says the server is compatible and can restart when convenient

### Requirement: Check from the CLI
`ship update --check` SHALL always ask, then print `installed → latest` or `up to date`, and change nothing but the shared result.

#### Scenario: Newer release
- **WHEN** `v0.2.0` is released and a `v0.1.0` user runs `ship update --check`
- **THEN** it prints `0.1.0 → 0.2.0`, exits 0 and leaves the binary unchanged
