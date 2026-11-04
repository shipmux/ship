# Spec Delta

## ADDED Requirements

### Requirement: Update settings
`[client.updates]` SHALL configure updates. `check` SHALL turn scheduled checks on or off, defaulting to `true`. `command` SHALL be a shell line `ship update` runs instead of the build's method, unset by default. `latest_command` SHALL be a shell line printing the latest version, used instead of asking GitHub, unset by default. An unknown key SHALL fail loading with its line and column. The configuration page SHALL document the table.

#### Scenario: Unknown key
- **WHEN** the file sets `[client.updates] chek = false` and the user runs `ship config check`
- **THEN** it reports `chek` with its line and column and exits 1

#### Scenario: No updates table
- **WHEN** the file has no `[client.updates]`
- **THEN** clients check hourly, and `ship update` uses the build's method

### Requirement: Update settings reload
The update settings SHALL reload with the rest of `[client]`. Turning `check` on SHALL make a check due at once, and turning it off SHALL stop the schedule without a restart. A `[client]` that fails to load SHALL leave the running update settings in place.

#### Scenario: Checks off on reload
- **WHEN** a user saves `check = false` while a client is attached
- **THEN** that client makes no further scheduled checks
