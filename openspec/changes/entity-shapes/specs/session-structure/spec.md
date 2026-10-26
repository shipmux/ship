# Spec Delta

## MODIFIED Requirements

### Requirement: Optional names
Tab and pane names SHALL be optional. A name SHALL be stored exactly as sent, including `""` and leading or trailing spaces. Only JSON `null`, or a bare `--name` on the CLI, SHALL clear a name. Duplicate tab and pane names SHALL be allowed anywhere, including among top-level tabs.

#### Scenario: Duplicate tab names
- **WHEN** a user creates two top-level tabs with the same name
- **THEN** both tabs exist with distinct IDs

#### Scenario: Unnamed tabs and panes
- **WHEN** a user runs `ship tab create` and `ship pane create --tab <tab>` without names
- **THEN** both entities are created and their JSON shows `"name": null`

#### Scenario: Blank names are names
- **WHEN** a user runs `ship tab create --name "  x "` and `ship pane create --tab <tab> --name ""`
- **THEN** their JSON shows `"name": "  x "` and `"name": ""`

#### Scenario: Clear a name
- **WHEN** a user runs `ship tab set --tab <id>` or `ship pane set --pane <id>` with a bare `--name`
- **THEN** the entity's name becomes `null`

### Requirement: HTTP error statuses
Errors SHALL use HTTP 400 for malformed input, 404 for a missing target, 422 for an invalid structural relationship, and 503 when server machinery is unavailable. Malformed input means a malformed path ID, or a request body or query string that doesn't decode, as Request body validation and Query string validation describe. A well-formed request that would break the structure, such as moving a tab into its own descendant, SHALL be rejected with 422.

#### Scenario: Empty move body
- **WHEN** a client sends `POST /api/v0/tabs/{id}/move` with `{}`
- **THEN** it receives 400 `validation` naming the missing `at` and nothing moves, while `{"at": {"parent": null}}` moves the tab to the top level

#### Scenario: Wrong kind of ID in a body
- **WHEN** a client sends `POST /api/v0/tabs/{id}/move` with `{"at": {"after": "pane:…"}}`
- **THEN** it receives 400 `validation` pointing at `/at/after` and nothing moves

### Requirement: Set attributes
`server.tab.set` and `ship tab set`, and `server.pane.set` and `ship pane set`, SHALL change a tab's or a pane's attributes in one edit. Each attribute SHALL be an optional field, and a field left out SHALL stay as it is. A bare `--name` SHALL clear the name. A `set` with no field, or one that changes nothing, SHALL succeed and SHALL NOT change the shared state's revision. `rename` SHALL NOT exist as a command or action.

#### Scenario: Two attributes in one edit
- **WHEN** a script runs `ship pane set --pane <id> --name build --right-click-passthrough true`
- **THEN** both change in one edit, and `ship pane get` shows them

#### Scenario: Nothing to set
- **WHEN** a script runs `ship pane set --pane <id>` with no attribute
- **THEN** nothing changes, the revision stays, and it exits 0

#### Scenario: Old command
- **WHEN** a script runs `ship tab rename --tab <id> notes`
- **THEN** the CLI reports an unknown subcommand, exits 2 and nothing changes

### Requirement: Set attribute fields
A tab's attributes SHALL be `name` (`--name`) and `zoomed` (`--zoomed`). A pane's attributes SHALL be `name` (`--name`), `right_click_passthrough` (`--right-click-passthrough`), `exit_behavior` (`--exit-behavior`) and `scrollback_bytes` (`--scrollback-bytes`).

#### Scenario: Exit behavior and history in one edit
- **WHEN** a script runs `ship pane set --pane <id> --exit-behavior keep_failed --scrollback-bytes 50000000`
- **THEN** both change in one edit, and `ship pane get` shows `exit_behavior: keep_failed` and `scrollback_bytes: 50000000`

#### Scenario: Name and zoom in one edit
- **WHEN** a script runs `ship tab set --tab <t> --name build --zoomed <p>` for a pane `<p>` in `<t>`
- **THEN** both change in one edit, and `ship tab get` shows them

### Requirement: HTTP set fields
Over HTTP, `PATCH /api/v0/tabs/{id}` SHALL take `{ "name"?, "zoomed"? }` and `PATCH /api/v0/panes/{id}` SHALL take `{ "name"?, "right_click_passthrough"?, "exit_behavior"?, "scrollback_bytes"? }`, following JSON Merge Patch (RFC 7386). An absent field SHALL leave its attribute unchanged, and JSON `null` SHALL clear it. `null` SHALL be accepted only on `name` and `zoomed`; on any other field it SHALL get 400 `validation`.

#### Scenario: Omitted and blank HTTP fields
- **WHEN** a caller patches a pane with `{}`, then `{"name": "  x "}`, then `{"name": ""}`, then `{"name": null}`
- **THEN** the first changes nothing, the next two store `"  x "` and `""` exactly, and the last clears the name

#### Scenario: Null on a field that can't be empty
- **WHEN** a caller patches a pane with `{"right_click_passthrough": null}`
- **THEN** it receives 400 `validation` pointing at `/right_click_passthrough` and the pane is unchanged

#### Scenario: Toggle over HTTP
- **WHEN** a caller patches a pane with `{"right_click_passthrough": "toggle"}`
- **THEN** the setting flips

### Requirement: HTTP set results
Both HTTP set routes SHALL return the entity. Unknown fields SHALL be ignored; a PATCH with only unknown fields SHALL change nothing and leave the revision unchanged.

#### Scenario: HTTP set response
- **WHEN** a caller patches a tab or pane
- **THEN** the route returns the entity, applies known fields and ignores unknown fields

## ADDED Requirements

### Requirement: One shape per entity
Every field of a tab or pane SHALL have one name, one path and one type in `get` output, set bodies, create bodies and `[server.defaults.*]` config. A request field that reuses an entity field's name SHALL have that field's type. PATCH bodies and set actions MAY also take `"toggle"` on a bool field. Config's `shell`, a path that defaults `command`, SHALL be the one exception.

#### Scenario: Send a field back
- **WHEN** a script reads `exit_behavior` from `GET /api/v0/panes/{id}` and sends it in `PATCH /api/v0/panes/{id}`, in `POST /api/v0/panes`'s `pane` object, and in `[server.defaults.pane]`
- **THEN** each accepts it under the same name with the same value

### Requirement: Create request bodies
`POST /api/v0/panes` SHALL take `{ "at", "direction"?, "select"?, "pane"? }` and `POST /api/v0/tabs` SHALL take `{ "at"?, "starter"?, "select"?, "tab"? }`. The request's own fields SHALL sit at the top level, and the new entity's fields SHALL sit under `pane` or `tab`, using the entity's names. An absent `at` on `POST /api/v0/tabs` SHALL mean the end of the top level.

#### Scenario: Create a pane with settings
- **WHEN** a caller sends `POST /api/v0/panes` with `{"at": {"tab": "<t>"}, "pane": {"cwd": "/tmp", "exit_behavior": "keep"}}`
- **THEN** a pane is created in `<t>` starting in `/tmp` with `exit_behavior: keep`

#### Scenario: Entity field at the top level
- **WHEN** a caller sends `POST /api/v0/panes` with `{"at": {"tab": "<t>"}, "cwd": "/tmp"}`
- **THEN** the top-level `cwd` is ignored and the pane is created using the normal directory default; only `pane.cwd` sets its starting directory

#### Scenario: Create a named tab
- **WHEN** a caller sends `POST /api/v0/tabs` with `{"tab": {"name": "notes"}}`
- **THEN** a top-level tab named `notes` is appended

### Requirement: Plain values on create
Create bodies and config defaults SHALL take plain values only. A create body SHALL reject `"toggle"` on every field, and `null` on every field but `name`, with 400 `validation`.

#### Scenario: Toggle on create
- **WHEN** a caller sends `POST /api/v0/panes` with `{"at": {"tab": "<t>"}, "pane": {"right_click_passthrough": "toggle"}}`
- **THEN** it receives 400 `validation` pointing at `/pane/right_click_passthrough`, and no pane is created

#### Scenario: Null on create
- **WHEN** a caller sends `POST /api/v0/panes` with `{"at": {"tab": "<t>"}, "pane": {"cwd": null}}`
- **THEN** it receives 400 `validation` pointing at `/pane/cwd`, and no pane is created

### Requirement: Tab move body
`POST /api/v0/tabs/{id}/move` SHALL take its destination as `{ "at" }`, with the same type as `POST /api/v0/tabs`'s `at`: `{"parent": <tab or null>}`, `{"before": <tab>}` or `{"after": <tab>}`.

#### Scenario: Move after a sibling
- **WHEN** a caller sends `POST /api/v0/tabs/{id}/move` with `{"at": {"after": "<sibling>"}}`
- **THEN** the tab sits right after `<sibling>`, under the same parent

#### Scenario: Old move body
- **WHEN** a caller sends `POST /api/v0/tabs/{id}/move` with `{"after": "<sibling>"}`
- **THEN** it receives 400 `validation` and nothing moves

### Requirement: Tab zoom
A tab's `zoomed` attribute SHALL be the pane filling it, or `null`. Setting it to a pane in that tab SHALL zoom that pane, and `null` SHALL unzoom. A pane in another tab SHALL get 422 `invalid_structure`, and a pane that doesn't exist SHALL get 404 `not_found`, changing nothing. `POST /api/v0/panes/{id}/zoom` and `ship pane zoom` SHALL NOT exist.

#### Scenario: Zoom and unzoom over HTTP
- **WHEN** a caller sends `PATCH /api/v0/tabs/{t}` with `{"zoomed": "<p>"}` for a pane `<p>` in `<t>`, then `{"zoomed": null}`
- **THEN** `<p>` fills the tab, then the tab shows every pane again

#### Scenario: Pane from another tab
- **WHEN** a caller patches `<t>` with `{"zoomed": "<q>"}` for a pane `<q>` in a different tab
- **THEN** it receives 422 `invalid_structure` and neither tab's zoom changes

#### Scenario: Unknown pane
- **WHEN** a caller patches `<t>` with `{"zoomed": "<ended>"}` naming a pane that doesn't exist
- **THEN** it receives 404 `not_found` and the tab's zoom is unchanged

#### Scenario: Zoom from the CLI
- **WHEN** a script runs `ship tab set --tab <t> --zoomed <p>`, then `ship tab set --tab <t> --zoomed`
- **THEN** `<p>` fills the tab, then the tab is unzoomed

#### Scenario: Old zoom command
- **WHEN** a script runs `ship pane zoom --pane <p>`
- **THEN** the CLI reports an unknown subcommand, exits 2 and nothing changes

### Requirement: Clearable CLI flags
Every CLI flag for an attribute that can be empty SHALL distinguish three cases: an absent flag SHALL leave the attribute alone, a bare flag SHALL clear it, and a flag with a value, including `""`, SHALL set it. This SHALL apply to `--name` on `ship tab set` and `ship pane set`, and to `--zoomed` on `ship tab set`.

#### Scenario: Bare flag and empty value
- **WHEN** a script runs `ship pane set --pane <id> --name`, then `ship pane set --pane <id> --name ""`
- **THEN** `ship pane get` shows `"name": null`, then `"name": ""`

### Requirement: Unknown request fields
JSON request structs SHALL ignore unknown fields. Tagged enums SHALL retain serde's tag structure. Known fields SHALL still use their declared types, required values and nullability.

#### Scenario: Extra field in a request struct
- **WHEN** a caller sends `PATCH /api/v0/panes/{id}` with only an extra `colour` field
- **THEN** the field is ignored, the pane is unchanged and the revision stays the same

#### Scenario: Externally tagged enum structure
- **WHEN** a caller sends `PUT /api/v0/server/log` with `{"set": "info", "extra": true}`
- **THEN** it receives 400 `validation`, because an externally tagged enum's object must have exactly one entry

### Requirement: Request body validation
A body that doesn't decode SHALL get 400 `validation`, with `data.errors` listing every problem found, each with `detail` and, where it has a location, a JSON Pointer `pointer`. A request that fails validation SHALL change nothing. A body sent without `Content-Type: application/json` SHALL get 400 `validation` with one entry.

#### Scenario: Four mistakes at once
- **WHEN** a caller sends `POST /api/v0/panes` with `{"direction": "up", "pane": {"exit_behavior": "linger", "scrollback_bytes": "lots", "exitBehavior": "keep"}}`
- **THEN** one 400 `validation` response lists the missing `at`, the bad `direction`, `linger` and `lots`, each with its pointer, ignoring `exitBehavior`, and no pane is created

#### Scenario: Not JSON
- **WHEN** a caller sends a body that isn't valid JSON
- **THEN** it receives 400 `validation` with one entry carrying the syntax message

#### Scenario: No content type
- **WHEN** a caller sends `PATCH /api/v0/panes/{id}` with a JSON body and no `Content-Type` header
- **THEN** it receives 400 `validation` with one entry, not 415, and the pane is unchanged

### Requirement: Query string validation
A query string SHALL ignore unknown parameters. One that doesn't decode SHALL get 400 `validation` in the same error shape as a body, and MAY list only the first problem. A malformed path ID SHALL get 400 `validation` in the same shape, with one entry. A request that fails validation SHALL return nothing else.

#### Scenario: Unknown parameter
- **WHEN** a caller sends `GET /api/v0/panes/{id}/text?line=50`
- **THEN** `line` is ignored and the normal default text result is returned

#### Scenario: Invalid known parameter
- **WHEN** a caller sends `GET /api/v0/panes/{id}/text?lines=nope`
- **THEN** it receives 400 `validation` with one entry naming `lines`

#### Scenario: Malformed path ID
- **WHEN** a caller sends `GET /api/v0/tabs/notes`
- **THEN** it receives 400 `validation` with one entry, as JSON rather than plain text

### Requirement: Described request bodies
The API description SHALL describe each request body as the server accepts it, including which fields accept `null`. Field descriptions SHALL state the real defaults, such as an absent `cwd` on a split taking the split pane's current directory.

#### Scenario: Nullable fields in the description
- **WHEN** a developer constructs and serializes the API description and reads the bodies of `PATCH /api/v0/tabs/{id}` and `PATCH /api/v0/panes/{id}`
- **THEN** `name` and `zoomed` are marked as accepting `null`, and `right_click_passthrough`, `exit_behavior` and `scrollback_bytes` are not
