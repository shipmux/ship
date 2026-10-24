# Spec Delta

## ADDED Requirements

### Requirement: JSON names
Every JSON object key and string value Ship reads or writes SHALL be snake case: HTTP request and response bodies, attach stream messages, CLI stdout, error lines and the saved tabs file. A name that the config file or a key-binding action shares with JSON SHALL be spelled the same in both. CLI flags SHALL spell the same words in kebab case. Ship SHALL NOT accept camel case keys.

#### Scenario: One spelling for a setting
- **WHEN** the config sets `[server.defaults.pane] right_click_passthrough = true` and a script runs `ship pane create` and then `ship pane get`
- **THEN** the pane JSON has `right_click_passthrough: true`, and the matching flag is `--right-click-passthrough`

#### Scenario: Camel case key over HTTP
- **WHEN** a caller sends `PATCH /api/v0/panes/{id}` with `{ "exitBehavior": "keep" }`
- **THEN** the request is rejected as an unknown field and the pane is unchanged

#### Scenario: Multi-word values
- **WHEN** a server shuts down while a client is attached, and a pane shows struck-through text
- **THEN** the stream ends with `"reason": "server_shutdown"`, and the text's run lists `crossed_out` in `attrs`

## MODIFIED Requirements

### Requirement: Exit behavior setting
Every pane SHALL have an exit behavior of `close`, `keep` or `keep_failed`. It is shared state, the same for every client, shown as `exit_behavior` by `ship pane get` and in `ship tab list` output. `ship pane create --exit-behavior` and `server.pane.create` with `exit_behavior` SHALL set it, else it SHALL come from `[server.defaults.pane]`. A tab's starter pane SHALL take the default. It SHALL survive a server restart as part of the saved tabs.

#### Scenario: Create with an exit behavior
- **WHEN** a script runs `ship pane create --tab <t> --exit-behavior keep -- cargo test`
- **THEN** `ship pane get` shows `exit_behavior: keep` for the new pane

#### Scenario: Starter pane
- **WHEN** the config has no `exit_behavior` and a script runs `ship tab create --starter`
- **THEN** its starter pane shows `exit_behavior: close`

#### Scenario: Bad value
- **WHEN** a script runs `ship pane create --exit-behavior sometimes`
- **THEN** the CLI rejects the value, exits 2 and nothing is created

### Requirement: Setting the exit behavior of an exited pane
Setting the exit behavior of a pane whose program has already exited SHALL store the value and do nothing else.

#### Scenario: Exited pane stays
- **WHEN** a pane with exit behavior `keep` has exited and a script runs `ship pane set --pane <id> --exit-behavior close`
- **THEN** `ship pane get` shows `exit_behavior: close` and the pane remains

### Requirement: Set attribute fields
A tab's only attribute SHALL be `name` (`--name`). A pane's attributes SHALL be `name` (`--name`), `right_click_passthrough` (`--right-click-passthrough`), `exit_behavior` (`--exit-behavior`) and `scrollback_bytes` (`--scrollback-bytes`).

#### Scenario: Exit behavior and history in one edit
- **WHEN** a script runs `ship pane set --pane <id> --exit-behavior keep_failed --scrollback-bytes 50000000`
- **THEN** both change in one edit, and `ship pane get` shows `exit_behavior: keep_failed` and `scrollback_bytes: 50000000`

### Requirement: HTTP set fields
Over HTTP, `PATCH /api/v0/tabs/{id}` SHALL take `{ "name"? }` and `PATCH /api/v0/panes/{id}` SHALL take `{ "name"?, "right_click_passthrough"?, "exit_behavior"?, "scrollback_bytes"? }`. An absent or JSON `null` field SHALL leave its attribute unchanged; a present empty or whitespace-only `name` SHALL clear it.

#### Scenario: Omitted and blank HTTP fields
- **WHEN** a caller patches a tab or pane with an absent or JSON `null` attribute, or an empty or whitespace-only `name`
- **THEN** an absent or `null` field leaves its attribute unchanged, and a present blank `name` clears it

### Requirement: Right-click passthrough setting
Every pane SHALL have a `right_click_passthrough` setting, on or off. It is shared state: one value per pane, the same for every client, shown as `right_click_passthrough` by `ship pane get` and in `ship tab list` output. The setting SHALL survive a server restart as part of the saved tabs.

#### Scenario: Survives a restart
- **WHEN** a pane has `right_click_passthrough: true` and the server restarts
- **THEN** the restored pane shows `right_click_passthrough: true`

### Requirement: Initial right-click passthrough
`server.pane.create` SHALL accept `right_click_passthrough = true` or `false`, and `ship pane create` the switch `--right-click-passthrough` (on) or `--right-click-passthrough=false`. Otherwise a new pane, including a tab's starter pane, SHALL take `[server.defaults.pane] right_click_passthrough`, which defaults to off.

#### Scenario: Create with passthrough
- **WHEN** a script runs `ship pane create --tab <t> --right-click-passthrough -- nvim`
- **THEN** `ship pane get` shows `right_click_passthrough: true` for the new pane

#### Scenario: Create without passthrough over a default
- **WHEN** the config sets `right_click_passthrough = true` and a script runs `ship pane create --right-click-passthrough=false`
- **THEN** `ship pane get` shows `right_click_passthrough: false` for the new pane

#### Scenario: Starter pane
- **WHEN** the config has no `right_click_passthrough` and a script runs `ship tab create --starter`
- **THEN** its starter pane shows `right_click_passthrough: false`
