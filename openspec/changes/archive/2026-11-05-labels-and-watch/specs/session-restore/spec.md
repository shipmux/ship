# Spec Delta

## MODIFIED Requirements

### Requirement: Nothing live is saved
The saved tabs SHALL NOT hold a pane's program or arguments, its title, its exit status, its status records, its geometry, or its screen and scrollback, nor any tab's or pane's tags.

#### Scenario: Program arguments stay off disk
- **WHEN** a pane runs `sleep 1234` and the saved tabs are written
- **THEN** the saved file does not contain `1234`

#### Scenario: Status records stay off disk
- **WHEN** a pane's program has reported `working` and the saved tabs are written
- **THEN** the saved file has no `program` field, and after a restart the restored pane has no records

#### Scenario: Tags stay off disk
- **WHEN** a tab and a pane have tags and the saved tabs are written
- **THEN** the saved file has no `tags` field, and after a restart neither has tags

### Requirement: Saved file format
The saved file SHALL hold pretty-printed JSON in the shape `ship tab list` prints, with each pane's `cwd` set to its saved directory, `command` empty, `status` running, and `dir`, `title`, `tags` and tab `geometry` absent. Loading SHALL ignore `command`, `status`, `dir`, `title`, `tags` and `geometry`, so `ship tab list` output is a valid saved file.

#### Scenario: Hand-written file
- **WHEN** a user saves `ship tab list` output as the server name's saved file and starts the server
- **THEN** the server restores those tabs, each pane running a shell in that pane's `cwd`

#### Scenario: Tags in a hand-written file
- **WHEN** the saved file was written from `ship tab list` output in which a tab and a pane have tags
- **THEN** the restored tab and pane have no tags
