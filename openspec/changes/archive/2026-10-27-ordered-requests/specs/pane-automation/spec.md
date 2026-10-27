# Spec Delta

## MODIFIED Requirements

### Requirement: Input without an attachment
`POST /api/v0/attach/input` SHALL accept a request without the `x-ship-attachment-id` header and route its key and paste frames as it does with one. A body holding a mouse frame SHALL get 422, and none of its frames SHALL be routed.

#### Scenario: Headerless keys and paste
- **WHEN** a caller posts a paste frame and a key frame for a pane without the attachment header
- **THEN** the pane receives the paste, then the key, and the response is 204

#### Scenario: Headerless mouse frame
- **WHEN** a caller posts a key frame and a mouse frame without the attachment header
- **THEN** the response is 422 and the pane receives neither

### Requirement: Input with an attachment
With the `x-ship-attachment-id` header, an unknown attachment SHALL get 404 with nothing routed, and mouse frames SHALL be accepted. A malformed header SHALL get 400.

#### Scenario: Full-screen client's mouse copy
- **WHEN** a user drags a selection in a full-screen client while a second client views the same tab
- **THEN** the copy reaches only the first client's clipboard

#### Scenario: Malformed header
- **WHEN** a caller posts input with `x-ship-attachment-id: nonsense`
- **THEN** the response is 400 and nothing is routed

## REMOVED Requirements

### Requirement: Input stream lines
**Reason**: Input is one JSON body instead of a stream of lines. Input body replaces this requirement.
**Migration**: Send `{"frames": [...]}` as `application/json`, with the frames that were lines in order.

## ADDED Requirements

### Requirement: Input body
`POST /api/v0/attach/input` SHALL take an `application/json` body `{"frames": [...]}`, holding input frames in the order they should reach their panes. It SHALL answer 204 once every frame is routed. Unknown fields SHALL be ignored. A frame for a removed pane SHALL be dropped, and the frames around it SHALL still be routed.

#### Scenario: Several frames in one body
- **WHEN** a caller posts a paste frame and two key frames for a pane in one body
- **THEN** the pane receives the paste, then both keys in order, and the response is 204

#### Scenario: Unknown field in a frame
- **WHEN** a caller posts a key frame for a pane and a key frame with an extra `colour` field
- **THEN** the pane receives both keys, ignores `colour`, and the response is 204

#### Scenario: Large paste
- **WHEN** `ship pane send -` reads 1 MB of text from stdin
- **THEN** the pane receives it as one paste

### Requirement: Refused input body
The input body SHALL be decoded as a request body is, as session-structure's Request body validation describes. A body with a bad known value, invalid JSON or more than 2 MB SHALL get 400 `validation`, and none of its frames SHALL be routed.

#### Scenario: Bad value after a valid frame
- **WHEN** a caller posts a valid paste frame, then a paste frame whose `text` is a number
- **THEN** the response is 400 `validation` with `data.errors`, and the pane receives neither paste

#### Scenario: A body that isn't JSON
- **WHEN** a caller posts a body that isn't valid JSON
- **THEN** the response is 400 `validation` with one entry carrying the syntax message, and nothing is routed

#### Scenario: Oversized body
- **WHEN** a caller posts a 3 MB paste frame
- **THEN** the response is 400 `validation` and nothing is routed
