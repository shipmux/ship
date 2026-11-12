# Spec Delta

## MODIFIED Requirements

### Requirement: HTTP API prefix
The tab, pane and attachment HTTP routes SHALL be served under `/api` and SHALL address entities by ID only. The path SHALL carry no version: `protocol_version` in `/health` is the one version a client checks. The existing `/health` route SHALL remain at `/health`.

#### Scenario: Prefixed route
- **WHEN** a client sends `GET /api/tabs` to a running server
- **THEN** it receives the ordered top-level tabs with their descendants, while `GET /health` still returns the health response

#### Scenario: Old prefix
- **WHEN** a client sends `GET /api/v0/tabs`
- **THEN** it receives 404

### Requirement: Unknown request fields
A JSON request body with a field the server doesn't know, at any depth, SHALL get 400 `validation` with an entry whose `pointer` names that field, and SHALL change nothing. Tagged enums SHALL retain serde's tag structure. Known fields SHALL still use their declared types, required values and nullability. Query strings follow Query string validation instead.

#### Scenario: Extra field in a request struct
- **WHEN** a caller sends `PATCH /api/panes/{id}` with only an extra `colour` field
- **THEN** it receives 400 `validation` with an entry whose pointer is `/colour`, the pane is unchanged and the revision stays the same

#### Scenario: Extra field in a nested object
- **WHEN** a caller sends `POST /api/panes` with `{"at": {"tab": "<t>"}, "pane": {"cwd": "/tmp", "colour": "red"}}`
- **THEN** it receives 400 `validation` with an entry whose pointer is `/pane/colour`, and no pane is created

#### Scenario: Externally tagged enum structure
- **WHEN** a caller sends `PUT /api/server/log` with `{"set": "info", "extra": true}`
- **THEN** it receives 400 `validation`, because an externally tagged enum's object must have exactly one entry

### Requirement: JSON names
Every JSON object key and string value Ship reads or writes SHALL be snake case: HTTP request and response bodies, attach stream messages, CLI stdout, error lines and the saved tabs file. A name that the config file or a key-binding action shares with JSON SHALL be spelled the same in both. CLI flags SHALL spell the same words in kebab case. Ship SHALL NOT read camel case keys: one is an unknown field, rejected as Unknown request fields describes.

#### Scenario: One spelling for a setting
- **WHEN** the config sets `[server.defaults.pane] right_click_passthrough = true` and a script runs `ship pane create` and then `ship pane get`
- **THEN** the pane JSON has `right_click_passthrough: true`, and the matching flag is `--right-click-passthrough`

#### Scenario: Camel case key over HTTP
- **WHEN** a caller sends `PATCH /api/panes/{id}` with `{ "exitBehavior": "keep" }`
- **THEN** it receives 400 `validation` with an entry whose pointer is `/exitBehavior`, the pane keeps its exit behavior and the revision stays the same

#### Scenario: Multi-word values
- **WHEN** a server shuts down while a client is attached, and a pane shows struck-through text
- **THEN** the stream ends with `"reason": "server_shutdown"`, and the text's run lists `crossed_out` in `attrs`

### Requirement: HTTP set results
Both HTTP set routes SHALL return the entity. A body with an unknown field SHALL be rejected as Unknown request fields describes, and SHALL change nothing.

#### Scenario: HTTP set response
- **WHEN** a caller patches a tab or pane with known fields
- **THEN** the route applies them and returns the entity

### Requirement: Create request bodies
`POST /api/panes` SHALL take `{ "at", "place"?, "select"?, "pane"? }` and `POST /api/tabs` SHALL take `{ "at"?, "starter"?, "select"?, "tab"? }`. The request's own fields SHALL sit at the top level, and the new entity's fields SHALL sit under `pane` or `tab`, using the entity's names. An absent `at` on `POST /api/tabs` SHALL mean the end of the top level.

#### Scenario: Create a pane with settings
- **WHEN** a caller sends `POST /api/panes` with `{"at": {"tab": "<t>"}, "pane": {"cwd": "/tmp", "exit_behavior": "keep"}}`
- **THEN** a pane is created in `<t>` starting in `/tmp` with `exit_behavior: keep`

#### Scenario: Entity field at the top level
- **WHEN** a caller sends `POST /api/panes` with `{"at": {"tab": "<t>"}, "cwd": "/tmp"}`
- **THEN** it receives 400 `validation` with an entry whose pointer is `/cwd`, and no pane is created; only `pane.cwd` sets a starting directory

#### Scenario: Create a named tab
- **WHEN** a caller sends `POST /api/tabs` with `{"tab": {"name": "notes"}}`
- **THEN** a top-level tab named `notes` is appended

### Requirement: Request body validation
A body that doesn't decode SHALL get 400 `validation`, with `data.errors` listing every problem found, each with `detail` and, where it has a location, a JSON Pointer `pointer`. A request that fails validation SHALL change nothing. A body sent without `Content-Type: application/json` SHALL get 400 `validation` with one entry.

#### Scenario: Four mistakes at once
- **WHEN** a caller sends `POST /api/panes` with `{"place": "up", "pane": {"exit_behavior": "linger", "scrollback_bytes": "lots"}}`
- **THEN** one 400 `validation` response lists the missing `at`, the bad `place`, `linger` and `lots`, each with its pointer, and no pane is created

#### Scenario: Not JSON
- **WHEN** a caller sends a body that isn't valid JSON
- **THEN** it receives 400 `validation` with one entry carrying the syntax message

#### Scenario: No content type
- **WHEN** a caller sends `PATCH /api/panes/{id}` with a JSON body and no `Content-Type` header
- **THEN** it receives 400 `validation` with one entry, not 415, and the pane is unchanged

### Requirement: HTTP error statuses
Errors SHALL use HTTP 400 for malformed input, 404 for a missing target, 422 for an invalid structural relationship, and 503 when server machinery is unavailable. Malformed input means a malformed path ID, or a request body or query string that doesn't decode, as Request body validation and Query string validation describe. A well-formed request that would break the structure, such as moving a tab into its own descendant, SHALL be rejected with 422.

#### Scenario: Empty move body
- **WHEN** a client sends `POST /api/tabs/{id}/move` with `{}`
- **THEN** it receives 400 `validation` naming the missing `at` and nothing moves, while `{"at": {"parent": null}}` moves the tab to the top level

#### Scenario: Wrong kind of ID in a body
- **WHEN** a client sends `POST /api/tabs/{id}/move` with `{"at": {"after": "pane:…"}}`
- **THEN** it receives 400 `validation` pointing at `/at/after` and nothing moves

### Requirement: HTTP set fields
Over HTTP, `PATCH /api/tabs/{id}` SHALL take `{ "name"?, "zoomed"?, "tags"? }` and `PATCH /api/panes/{id}` SHALL take `{ "name"?, "right_click_passthrough"?, "exit_behavior"?, "scrollback_bytes"?, "tags"? }`, as JSON Merge Patch (RFC 7396). An absent field SHALL leave its attribute unchanged, and JSON `null` SHALL clear it. `null` SHALL be accepted only on `name`, `zoomed` and `tags`; on any other field it SHALL get 400 `validation`. `tags` follows the `tags` capability's merge rules.

#### Scenario: Omitted and blank HTTP fields
- **WHEN** a caller patches a pane with `{}`, then `{"name": "  x "}`, then `{"name": ""}`, then `{"name": null}`
- **THEN** the first changes nothing, the next two store `"  x "` and `""` exactly, and the last clears the name

#### Scenario: Null on a field that can't be empty
- **WHEN** a caller patches a pane with `{"right_click_passthrough": null}`
- **THEN** it receives 400 `validation` pointing at `/right_click_passthrough` and the pane is unchanged

#### Scenario: Toggle over HTTP
- **WHEN** a caller patches a pane with `{"right_click_passthrough": "toggle"}`
- **THEN** the setting flips

#### Scenario: Null tags
- **WHEN** a caller patches a pane that has tags with `{"tags": null}`
- **THEN** the pane has no tags

### Requirement: One shape per entity
Every field of a tab or pane SHALL have one name, one path and one type in `get` output, set bodies, create bodies and `[server.defaults.*]` config. A request field that reuses an entity field's name SHALL have that field's type. PATCH bodies and set actions MAY also take `"toggle"` on a bool field. Config's `shell`, a path that defaults `command`, SHALL be the one exception.

#### Scenario: Send a field back
- **WHEN** a script reads `exit_behavior` from `GET /api/panes/{id}` and sends it in `PATCH /api/panes/{id}`, in `POST /api/panes`'s `pane` object, and in `[server.defaults.pane]`
- **THEN** each accepts it under the same name with the same value

### Requirement: HTTP pane creation placement
`place` SHALL be `{"split": "right"}`, `{"split": "down"}` or `"stack"`, defaulting to `{"split": "right"}`; the body SHALL NOT take `direction`.

#### Scenario: Stack a pane over HTTP
- **WHEN** a caller sends `POST /api/panes` with `{"at": {"pane": "<p>"}, "place": "stack"}`
- **THEN** the new pane is in front of `<p>`'s slot

### Requirement: Plain values on create
Create bodies and config defaults SHALL take plain values only. A create body SHALL reject `"toggle"` on every field, and `null` on every field but `name`, with 400 `validation`.

#### Scenario: Toggle on create
- **WHEN** a caller sends `POST /api/panes` with `{"at": {"tab": "<t>"}, "pane": {"right_click_passthrough": "toggle"}}`
- **THEN** it receives 400 `validation` pointing at `/pane/right_click_passthrough`, and no pane is created

#### Scenario: Null on create
- **WHEN** a caller sends `POST /api/panes` with `{"at": {"tab": "<t>"}, "pane": {"cwd": null}}`
- **THEN** it receives 400 `validation` pointing at `/pane/cwd`, and no pane is created

### Requirement: Tab move body
`POST /api/tabs/{id}/move` SHALL take its destination as `{ "at" }`, with the same type as `POST /api/tabs`'s `at`: `{"parent": <tab or null>}`, `{"before": <tab>}` or `{"after": <tab>}`.

#### Scenario: Move after a sibling
- **WHEN** a caller sends `POST /api/tabs/{id}/move` with `{"at": {"after": "<sibling>"}}`
- **THEN** the tab sits right after `<sibling>`, under the same parent

#### Scenario: Old move body
- **WHEN** a caller sends `POST /api/tabs/{id}/move` with `{"after": "<sibling>"}`
- **THEN** it receives 400 `validation` and nothing moves

### Requirement: Tab zoom
A tab's `zoomed` attribute SHALL be the front pane of the slot filling it, or `null`. Setting it to a pane in that tab SHALL bring that pane to its slot's front and zoom the slot, and `null` SHALL unzoom. A pane in another tab SHALL get 422 `invalid_structure`, and a pane that doesn't exist SHALL get 404 `not_found`, changing nothing. `POST /api/panes/{id}/zoom` and `ship pane zoom` SHALL NOT exist.

#### Scenario: Zoom and unzoom over HTTP
- **WHEN** a caller sends `PATCH /api/tabs/{t}` with `{"zoomed": "<p>"}` for a pane `<p>` in `<t>`, then `{"zoomed": null}`
- **THEN** `<p>` fills the tab, then the tab shows every slot again

#### Scenario: Zoom a pane behind
- **WHEN** a slot holds `[X, A]` and a script runs `ship tab set --tab <t> --zoomed A`
- **THEN** the slot holds `[A, X]`, it fills the tab, and `zoomed` is `A`

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

### Requirement: Query string validation
A query string SHALL ignore unknown parameters. One that doesn't decode SHALL get 400 `validation` in the same error shape as a body, and MAY list only the first problem. A malformed path ID SHALL get 400 `validation` in the same shape, with one entry. A request that fails validation SHALL return nothing else.

#### Scenario: Unknown parameter
- **WHEN** a caller sends `GET /api/panes/{id}/text?line=50`
- **THEN** `line` is ignored and the normal default text result is returned

#### Scenario: Invalid known parameter
- **WHEN** a caller sends `GET /api/panes/{id}/text?lines=nope`
- **THEN** it receives 400 `validation` with one entry naming `lines`

#### Scenario: Malformed path ID
- **WHEN** a caller sends `GET /api/tabs/notes`
- **THEN** it receives 400 `validation` with one entry, as JSON rather than plain text

### Requirement: Described request bodies
The API description SHALL describe each request body as the server accepts it, including which fields accept `null`. Field descriptions SHALL state the real defaults, such as an absent `cwd` on a split taking the split pane's current directory.

#### Scenario: Nullable fields in the description
- **WHEN** a developer constructs and serializes the API description and reads the bodies of `PATCH /api/tabs/{id}` and `PATCH /api/panes/{id}`
- **THEN** `name` and `zoomed` are marked as accepting `null`, and `right_click_passthrough`, `exit_behavior` and `scrollback_bytes` are not

## RENAMED Requirements

- FROM: `### Requirement: Versioned HTTP API`
- TO: `### Requirement: HTTP API prefix`
