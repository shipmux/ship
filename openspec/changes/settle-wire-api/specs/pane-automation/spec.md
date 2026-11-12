# Spec Delta

## MODIFIED Requirements

### Requirement: Input body
`POST /api/attach/input` SHALL take an `application/json` body `{"frames": [...]}`, holding input frames in the order they should reach their panes. It SHALL answer 204 once every frame is routed. A body with an unknown field, in any frame, SHALL be rejected as session-structure's Unknown request fields describes, and no frame SHALL be routed. A frame for a removed pane SHALL be dropped, and the frames around it SHALL still be routed.

#### Scenario: Several frames in one body
- **WHEN** a caller posts a paste frame and two key frames for a pane in one body
- **THEN** the pane receives the paste, then both keys in order, and the response is 204

#### Scenario: Unknown field in a frame
- **WHEN** a caller posts a key frame for a pane and a key frame with an extra `colour` field
- **THEN** it receives 400 `validation` with an entry whose pointer is `/frames/1/colour`, and the pane receives neither key

#### Scenario: Large paste
- **WHEN** `ship pane send -` reads 1 MB of text from stdin
- **THEN** the pane receives it as one paste

### Requirement: Input without an attachment
`POST /api/attach/input` SHALL accept a request without the `x-ship-attachment-id` header and route its key and paste frames as it does with one. A body holding a mouse frame SHALL get 422, and none of its frames SHALL be routed.

#### Scenario: Headerless keys and paste
- **WHEN** a caller posts a paste frame and a key frame for a pane without the attachment header
- **THEN** the pane receives the paste, then the key, and the response is 204

#### Scenario: Headerless mouse frame
- **WHEN** a caller posts a key frame and a mouse frame without the attachment header
- **THEN** the response is 422 and the pane receives neither
