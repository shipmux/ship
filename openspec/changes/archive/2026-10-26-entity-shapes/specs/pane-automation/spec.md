# Spec Delta

## MODIFIED Requirements

### Requirement: Input without an attachment
`POST /api/v0/attach/input` SHALL accept a request without the `x-ship-attachment-id` header and route its key and paste frames as it does with one. A mouse frame on such a request SHALL stop reading with 422. Without the header, the request SHALL end only when its body or the server ends.

#### Scenario: Headerless keys and paste
- **WHEN** a caller posts a paste frame and a key frame for a pane without the attachment header
- **THEN** the pane receives the paste, then the key, and the response is 204

#### Scenario: Headerless mouse frame
- **WHEN** a caller posts a mouse frame without the attachment header
- **THEN** the response is 422 and reading stops at that line

## ADDED Requirements

### Requirement: Input stream lines
Each line of `POST /api/v0/attach/input` SHALL be decoded as a request body is, as session-structure's Request body validation describes. Unknown fields SHALL be ignored. A line with a bad known value or invalid JSON SHALL stop reading with 400 `validation`, listing that line's problems in `data.errors`. Lines before it SHALL already have been delivered.

#### Scenario: Unknown field in a line
- **WHEN** a caller posts a key frame for a pane, then a key frame with an extra `colour` field
- **THEN** the pane receives both keys, ignores `colour`, and the response is 204

#### Scenario: Bad value after a valid line
- **WHEN** a caller posts a valid paste frame, then a paste frame whose `text` is a number
- **THEN** the pane receives the first paste, the response is 400 `validation` with `data.errors`, and reading stops at the second line

#### Scenario: A line that isn't JSON
- **WHEN** a caller posts a line that isn't valid JSON
- **THEN** the response is 400 `validation` with one entry carrying the syntax message, and reading stops at that line
