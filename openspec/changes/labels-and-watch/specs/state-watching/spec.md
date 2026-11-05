# Spec Delta

## Purpose

Let a script or plugin follow a server's state without polling, by printing the same state stream every client receives as JSON lines, so plugins can be ordinary clients with no event system of their own.

## ADDED Requirements

### Requirement: Watch output
`ship watch` SHALL print a first JSON line holding the server's `protocol_version`, then one JSON line per published state, each holding that state unchanged from what attached clients receive. Each line SHALL be written as soon as its state arrives, so a reader on a pipe sees it without waiting for more output.

#### Scenario: Follow a rename
- **WHEN** a user runs `ship watch | jq -c .` and renames a tab from another terminal
- **THEN** the first line holds `protocol_version`, and a later line holds a state in which the tab has its new name

#### Scenario: Tags in the stream
- **WHEN** a script sets a tag on a pane while `ship watch` runs
- **THEN** the next line's state shows the pane with that tag

### Requirement: Watch feeds
`ship watch` SHALL accept `--feed latest` and `--feed every`, defaulting to `latest`. With `latest`, each line SHALL hold the newest state at the time it is sent, and states in between may be skipped. With `every`, it SHALL print every state after the first in revision order.

#### Scenario: Slow reader on the default feed
- **WHEN** a reader pauses while ten tabs are renamed, then resumes
- **THEN** the next line it reads holds a state with all ten names, and lines may skip revisions

#### Scenario: Every revision
- **WHEN** `ship watch --feed every` runs and a script renames a tab ten times
- **THEN** ten lines follow with consecutive revisions, one per rename

### Requirement: Watch leaves out screens and clipboard
`ship watch` SHALL NOT print screen contents or clipboard text. Watching SHALL NOT select, size or resize any tab or pane, and SHALL NOT change any client's view.

#### Scenario: Busy panes
- **WHEN** panes are drawing output while `ship watch` runs
- **THEN** no line holds screen rows or clipboard text, and every client's view and pane sizes stay as they were

### Requirement: Watch ending
When its stream ends, because the server shuts down or an `every` feed falls too far behind, `ship watch` SHALL report why and exit with a non-zero status.

#### Scenario: Server stops
- **WHEN** `ship watch` runs and the user runs `ship server stop`
- **THEN** `ship watch` reports that the server shut down and exits non-zero

#### Scenario: Falling behind on every
- **WHEN** `ship watch --feed every` falls behind by more than the server keeps
- **THEN** it reports that it fell behind and exits non-zero
