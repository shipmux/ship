# Spec Delta

## MODIFIED Requirements

### Requirement: Show the server's filter
`ship server log` SHALL print the running server's log filter as one JSON line with `active`, `startup` and `file`, the path of the part being written. `GET /api/server/log` SHALL return the same JSON. Like `ship server status`, it SHALL never start a server.

#### Scenario: Show
- **WHEN** a user runs `ship server log` against a server started with default settings
- **THEN** it prints JSON whose `active` and `startup` are the default filter and whose `file` is the run's current part

### Requirement: Invalid filter changes nothing
An invalid level or filter SHALL fail and leave the active filter unchanged. `PUT /api/server/log` SHALL take `{"set": "<filter>"}` or `"reset"`, return the resulting filter, and answer 422 with the filter's error when the filter is invalid.

#### Scenario: Bad filter
- **WHEN** a user runs `ship server log --filter 'ship=['`
- **THEN** the command fails with the filter's error, and `ship server log` shows the same `active` as before
