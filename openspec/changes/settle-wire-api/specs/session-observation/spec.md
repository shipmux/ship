# Spec Delta

## ADDED Requirements

### Requirement: Additions from a newer server
Every reader of an attach stream in Ship, the full client, `ship pane wait` and `ship pane send --wait`, SHALL skip an event whose `type` it doesn't know and keep reading, and SHALL ignore fields it doesn't know. It SHALL treat an `ended` reason it doesn't know as it treats `lost`, and a status record it can't read as absent. An event of a known type that doesn't decode SHALL still be an error.

#### Scenario: Unknown event type
- **WHEN** a server sends an event of a type the client doesn't know between two `state` events
- **THEN** the client keeps its screens, applies the second state and stays attached

#### Scenario: Unknown end reason
- **WHEN** a server ends an attached client's stream with a reason the client doesn't know
- **THEN** the full client reconnects as it does after `lost`

#### Scenario: Unknown reported state
- **WHEN** a pane's root status record has a state the client doesn't know
- **THEN** the client draws the pane like one without a report, and `ship pane wait` on it keeps waiting

## MODIFIED Requirements

### Requirement: Falling behind an every feed
The server SHALL keep at least the last 256 revisions for `every` streams. A stream that falls further behind SHALL end with `{"type": "ended", "data": {"reason": "lost"}}` instead of skipping revisions, and its attachment SHALL detach.

#### Scenario: Stalled reader
- **WHEN** an `every` stream's reader stops reading while more revisions than the server keeps are published, and then resumes
- **THEN** when the server detects the lag, it ends the stream with reason `lost` without draining the retained revisions, and `ship view list` no longer shows the attachment

### Requirement: Attach feed
`POST /api/attach` SHALL accept `"feed": "latest"` or `"feed": "every"`, defaulting to `latest`. A `latest` stream SHALL send only the newest replica, so it may skip revisions in between. An `every` stream SHALL send each replica newer than its `attached` seed, once and in revision order. Screen and clipboard events SHALL NOT depend on the feed.

#### Scenario: Every revision in order
- **WHEN** a caller attaches with `{"feed": "every"}` and a script renames a pane 20 times
- **THEN** the caller receives 20 `state` events with consecutive revisions

#### Scenario: Default feed
- **WHEN** a caller attaches with `{}`
- **THEN** it receives the newest state after each change and may skip revisions in between, as before

### Requirement: Area-only view updates
`PUT /api/attach/view` SHALL carry only the attachment's terminal `area`, with the attachment named by `x-ship-attachment-id`. Updating the area SHALL NOT replace its selection; selection changes SHALL use the selection route or a selecting create, apart from attachment initialization and server repair.

#### Scenario: Resize after an external selection
- **WHEN** a script selects a pane for an attached client and that client resizes its terminal
- **THEN** the reported area changes and the scripted selection remains selected

### Requirement: Views routes
`GET /api/views` SHALL return every attachment's viewing record as a JSON object keyed by attachment ID, in attach order, with the same records the observation stream publishes under `viewers`. `GET /api/views/{attachment}` SHALL return one record. `PUT /api/views/{attachment}/selection` with `{"selection": <tab or pane ID>}` or `{"selection": null}` SHALL set that attachment's selection and return the record as stored. An unknown attachment or node SHALL get 404 and change nothing.

#### Scenario: Read every view
- **WHEN** two clients are attached and a caller sends `GET /api/views`
- **THEN** it receives an object with both attachment IDs, each holding that client's selection and area, when it has them

#### Scenario: Read an observer
- **WHEN** a full-screen client and an observer are attached and a caller sends `GET /api/views`
- **THEN** the observer's record holds neither a selection nor an area

#### Scenario: Select nothing
- **WHEN** a caller sends `PUT /api/views/<id>/selection` with `{"selection": null}`
- **THEN** it receives the record without a selection, and that client shows the nothing-selected hint

#### Scenario: Unknown node
- **WHEN** a caller selects a pane ID that doesn't exist for an attached client
- **THEN** it receives 404 and the client's selection is unchanged

#### Scenario: Unknown attachment
- **WHEN** a caller sends `GET /api/views/<id>` for an attachment that has ended
- **THEN** it receives 404

### Requirement: Observer attachments
`POST /api/attach` SHALL accept a request without an `area`. An attachment without an area SHALL be left out of every tab's size, as one without a selection is, and its viewing record SHALL omit `area`. An **observer** is an attachment with neither an area nor a selection: it SHALL receive the `attached` replica and later `state` replicas as its feed sends them, and no `screen` events. A later `PUT /api/attach/view` SHALL give the attachment an area like any other.

#### Scenario: Observer leaves geometry alone
- **WHEN** a full-screen client views a tab and a caller attaches with the body `{}`
- **THEN** the tab's published `geometry` is unchanged, the caller receives `attached` and `state` events and no `screen` events, and `ship view list` shows its record without an area

#### Scenario: Observer on the every feed
- **WHEN** a caller attaches with `{"feed": "every"}`
- **THEN** it receives a `state` event for each later revision and no `screen` events

#### Scenario: Observer detaches
- **WHEN** the observer's stream is dropped
- **THEN** its record leaves `ship view list` and the full-screen client keeps drawing as before
