# Spec Delta

## MODIFIED Requirements

### Requirement: Watch output
`ship watch` SHALL print a first JSON line holding the server's `protocol_version`, then every event of its attach stream as one JSON line, exactly as the server sent it, including event types `watch` doesn't know: `attached` with the first state at `data.replica`, then `state` with each later state at `data`, then `ended`. Each line SHALL be written as soon as its event arrives, so a reader on a pipe sees it without waiting for more output.

#### Scenario: Follow a rename
- **WHEN** a user runs `ship watch | jq -c .` and renames a tab from another terminal
- **THEN** the first line holds `protocol_version`, the second has `type` `attached`, and a later line has `type` `state` and a state in which the tab has its new name

#### Scenario: Tags in the stream
- **WHEN** a script sets a tag on a pane while `ship watch` runs
- **THEN** the next `state` line shows the pane with that tag

#### Scenario: Event type from a newer server
- **WHEN** a server sends an event whose type this `ship watch` doesn't know
- **THEN** `watch` prints that line byte for byte and keeps running

### Requirement: Watch ending
When its stream ends, `ship watch` SHALL exit with a non-zero status. The server sends `ended`, which holds the reason, as the last line before it closes the stream, and `watch` prints it like any other line. A stream that stops without `ended` SHALL also exit non-zero. A reader closing `watch`'s output SHALL end it with a zero status.

#### Scenario: Server stops
- **WHEN** `ship watch` runs and the user runs `ship server stop`
- **THEN** its last line is `{"type":"ended","data":{"reason":"server_shutdown"}}` and it exits non-zero

#### Scenario: Falling behind on every
- **WHEN** `ship watch --feed every` falls behind by more than the server keeps
- **THEN** its last line is `{"type":"ended","data":{"reason":"lost"}}` and it exits non-zero

#### Scenario: Cut stream
- **WHEN** the server is killed while `ship watch` runs
- **THEN** it exits non-zero

#### Scenario: Reader stops early
- **WHEN** a user runs `ship watch | head -1`
- **THEN** `ship watch` exits zero
