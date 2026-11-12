# Spec Delta

## MODIFIED Requirements

### Requirement: Saved file format
The saved file SHALL hold pretty-printed JSON in the shape `ship tab list` prints, with each pane's `cwd` set to its saved directory, `command` empty, `process` running, and `dir`, `title`, `tags` and tab `geometry` absent. Loading SHALL ignore `command`, `process`, `dir`, `title`, `tags` and `geometry`, so `ship tab list` output is a valid saved file. A file that names the process state `status`, as files saved before protocol 24 do, SHALL load the same way.

#### Scenario: Hand-written file
- **WHEN** a user saves `ship tab list` output as the server name's saved file and starts the server
- **THEN** the server restores those tabs, each pane running a shell in that pane's `cwd`

#### Scenario: Tags in a hand-written file
- **WHEN** the saved file was written from `ship tab list` output in which a tab and a pane have tags
- **THEN** the restored tab and pane have no tags
