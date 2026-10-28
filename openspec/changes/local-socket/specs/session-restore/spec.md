## REMOVED Requirements

### Requirement: Saved file location
**Reason**: Servers are told apart by name instead of port.
**Migration**: Replaced by "Saved file per server name" below. Rename `tabs/43179.json` to `tabs/default.json` by hand to keep existing tabs.

## ADDED Requirements

### Requirement: Saved file per server name
Each server name SHALL have its own saved file at `$XDG_STATE_HOME/ship/tabs/<name>.json`, defaulting to `~/.local/state/ship/tabs/<name>.json` on Linux and macOS; other platforms' location is best effort. A server SHALL NOT read or write another name's file, and a second server with a running name SHALL fail to take the name before it reads the file.

#### Scenario: Two names
- **WHEN** servers named `default` and `scratch` run with different tabs and both restart
- **THEN** each restores its own saved tabs from its own file

#### Scenario: Second server with the same name
- **WHEN** a server named `scratch` is running and another `ship server --name scratch` starts
- **THEN** the second exits saying a server is already running, without reading or writing `tabs/scratch.json`

## MODIFIED Requirements

### Requirement: Saved file format
The saved file SHALL hold pretty-printed JSON in the shape `ship tab list` prints, with each pane's `cwd` set to its saved directory, `command` empty, `status` running, and `dir`, `title` and tab `geometry` absent. Loading SHALL ignore `command`, `status`, `dir`, `title` and `geometry`, so `ship tab list` output is a valid saved file.

#### Scenario: Hand-written file
- **WHEN** a user saves `ship tab list` output as the server name's saved file and starts the server
- **THEN** the server restores those tabs, each pane running a shell in that pane's `cwd`

### Requirement: Kept previous copy
Before a server's first save replaces an existing saved file, the server SHALL copy that file's contents, readable or not, to `<name>.json.prev` beside it, replacing any older copy. Later saves in the same server run SHALL NOT change the copy. The server SHALL NOT restore from the copy on its own.

#### Scenario: Recover after starting fresh
- **WHEN** a user runs `ship server --no-restore`, makes an edit, stops the server, moves `<name>.json.prev` over `<name>.json` and starts the server
- **THEN** the server restores the tabs from before `--no-restore`

#### Scenario: Later saves leave the copy alone
- **WHEN** a server saves three times in one run
- **THEN** `<name>.json.prev` holds the file as it was before the first of those saves

### Requirement: Starting without restoring
`ship server --no-restore` SHALL start without reading the saved file. Its help SHALL say that the saved tabs are replaced on the first save and kept as `<name>.json.prev`. Every server launched automatically SHALL restore.

#### Scenario: Starting fresh
- **WHEN** the server's name has saved tabs and a user runs `ship server --no-restore`
- **THEN** `ship tab list` prints `{}` and the saved file is unchanged until the first save
