# Entity and request shapes

Status: Locked on 2026-10-25. Written from a design conversation with Cyan on 2026-10-25, which settled scope (one change with request validation), compatibility (a clean break), null handling, names, the CLI flags and the zoom move. Revised the same day at Cyan's direction ("sounds good to me") so query strings share the body error shape and reject unknown parameters, then approved in Plannotator review ("LGTM"). Revised again the same day at Cyan's direction ("i think i like this the best, give that a go") so `client.pane.zoom` takes a `zoomed` switch that states its toggle, approved in chat without a further review.

## Goal

The API spells the same thing several ways. A pane's `right_click_passthrough` is a bool when you `get` it, a bool or `"toggle"` when you set it, and a field mixed in with `at` and `direction` when you create one. Zoom is a pane ID on the tab but a `true`/`false` on a pane route. A blank name clears in one place, and `null` is ignored in another. Scripts and agents learn each route on its own, and typos in create bodies are silently dropped (#75).

This change gives every entity one shape, the one `get` returns, and makes every input a subset of it. Every request body then rejects unknown fields and lists every problem in one error. It serves anyone driving Ship from scripts, the CLI or keybindings, including Cyan's agents.

```text
today                                            after
POST /panes {at, direction, name, cwd, select}   POST /panes {at, direction, select, pane: {name, cwd}}
POST /tabs  {at, name, starter, select}          POST /tabs  {at, starter, select, tab: {name}}
POST /tabs/{id}/move {"after": "tab:…"}          POST /tabs/{id}/move {"at": {"after": "tab:…"}}
POST /panes/{id}/zoom {"zoomed": true}           PATCH /tabs/{id} {"zoomed": "pane:…"}
PATCH {"name": ""} clears, {"name": null} no-op  PATCH {"name": null} clears, {"name": ""} is a name
```

## User stories

### P1: A script learns a field once

- **Given** a pane from `GET /panes/{id}`, **when** a script sends any of its settable fields back in `PATCH /panes/{id}`, in `POST /panes`'s `pane` object, or in `[server.defaults.pane]`, **then** the field has the same name, path and type in each.
- **Given** a create body, **then** the request's own fields (`at`, `direction`, `starter`, `select`) sit at the top level, and everything describing the new entity sits under `pane` or `tab`.
- **Given** `POST /tabs` and `POST /tabs/{id}/move`, **then** both place the tab with the same `at` field and type.

Tested independently by diffing the OpenAPI document: every input field that names an entity field has that field's type, at that field's path.

### P2: A typo is reported, not dropped

- **Given** `POST /panes {"at": {"tab": "tab:…"}, "pane": {"exitBehavior": "keep"}}`, **when** it's sent, **then** the server answers 400 `validation`, and no pane is created.
- **Given** a body with three mistakes, such as an unknown field, a bad enum value and a wrong type, **then** one error lists all three, each with a JSON Pointer to where it is.
- **Given** `GET /panes/{id}/text?line=50`, **then** the server answers 400 `validation` naming `line`, instead of returning the whole screen.

Tested independently by sending bad bodies and queries to every route that takes one.

### P3: PATCH means what Merge Patch means

- **Given** `PATCH /panes/{id} {}`, **then** nothing changes.
- **Given** `{"name": null}`, **then** the name is cleared. **Given** `{"name": ""}` or `{"name": "  x "}`, **then** the name is stored exactly as sent.
- **Given** `{"right_click_passthrough": null}`, or `null` on any field that can't be empty, **then** the server answers 400 `validation`.
- **Given** `{"right_click_passthrough": "toggle"}`, **then** it flips, as today.

Tested independently with PATCH bodies against a running server.

### P4: Zoom is a tab field

- **Given** a split tab, **when** a script sends `PATCH /tabs/{id} {"zoomed": "pane:…"}`, **then** that pane fills the tab, and `{"zoomed": null}` unzooms it. A pane in another tab gets 422 `invalid_structure`, and a pane that doesn't exist gets 404 `not_found`.
- **Given** `ship tab set --tab T --zoomed P`, **then** the tab zooms P. **Given** a bare `--zoomed`, **then** it unzooms.
- **Given** a user presses `alt-f`, **then** the selected pane zooms, and pressing it again unzooms, as today. The default binding says so: `client.pane.zoom = { zoomed = "toggle" }`.
- **Given** a binding with `zoomed = true` or `zoomed = false`, **then** it only zooms the selected pane or only unzooms its tab, however often it is pressed.
- **Given** two clients press `alt-f` on the same unzoomed tab at once, **then** the tab ends up zoomed rather than flipping twice.

Tested independently by zooming from the CLI and from two attached clients.

### P5: Clearing a name from the CLI

- **Given** `ship pane set --pane P --name`, with no value, **then** the name is cleared. **Given** `--name ""`, **then** the name is set to the empty string.
- The same holds for `ship tab set --name`.

Tested independently by running both forms and reading the entity back with `get`.

### P6: The rules are written where the next change will find them

- **Given** someone adding a field or route, **then** `docs/agents/code.md` states the shape rules, and the keymap spec says when an action is `server.` and when it is `client.`.

Tested independently by reading the two documents.

## Non-goals

- Listing every problem in a query string. Queries report one problem, in the same shape as bodies.
- Compatibility with protocol 16. Old clients and scripts break, and the protocol version goes up.
- The saved tabs file. It's persistence, not an input, and keeps its own fields, including `cwd` for the current directory.
- Swap, resize and scroll. They stay verb routes with their own bodies.
- Clearing a name from a keybinding. TOML has no `null`, so `server.pane.set` can't clear one; the CLI and API can.
- Rejecting blank names, or showing them differently in the sidebar. An empty name shows as an empty row.
- A `"toggle"` value for the tab's `zoomed` field. The toggle lives in the `client.pane.zoom` action's argument (FR-011), which the client resolves before sending a plain pane ID or `null`.
- Config's `shell`. It stays a path, the one exception to "same name, same type", because it's the default for `command`, an argv.
- An ADR. The rules go in `docs/agents/code.md`.

## Functional requirements

### Shapes

- **FR-001** Every field of an entity MUST have one name, one path and one type everywhere it appears: `get` output, PATCH bodies, create bodies and `[server.defaults.*]` config. A request field that reuses an entity field's name MUST have that field's type. Config's `shell` is the one exception.
- **FR-002** Every input that describes an entity MUST be a subset of the entity's `get` shape. PATCH bodies and create's `pane` and `tab` objects MUST use the same paths.
- **FR-003** A create body MUST put its request fields (`at`, `direction`, `starter`, `select`) at the top level, and the new entity's fields under `pane` (for `POST /panes`) or `tab` (for `POST /tabs`).
- **FR-004** `POST /tabs/{id}/move` MUST take its placement as `at`, with the same type as `POST /tabs`'s `at`.
- **FR-005** Create bodies and config defaults MUST take plain values only. Only PATCH bodies, `server.` action arguments and `client.` action arguments MAY also take `"toggle"`, and only on bool fields or arguments.

### PATCH

- **FR-006** PATCH bodies MUST follow JSON Merge Patch (RFC 7386): an absent field is left alone, and `null` clears it.
- **FR-007** `null` MUST be accepted only on fields the entity can leave empty (`name`, `zoomed`). `null` on any other field MUST get 400 `validation`.
- **FR-008** Names MUST be stored exactly as sent, including `""` and leading or trailing spaces. Only `null` clears a name.

### Zoom

- **FR-009** A tab's zoom MUST be set through `PATCH /tabs/{id}` with `zoomed`: a pane ID in that tab zooms it, `null` unzooms. A pane in another tab MUST get 422 `invalid_structure`, matching how moves into a descendant are refused, and an unknown pane MUST get 404 `not_found`.
- **FR-010** `POST /panes/{id}/zoom` and `ship pane zoom` MUST be removed. `ship tab set` MUST take `--zoomed <pane>`, and a bare `--zoomed` MUST unzoom.
- **FR-011** The zoom keybinding and menu item MUST be `client.pane.zoom`, which MUST take a required `zoomed` argument of the same type as `right_click_passthrough` in PATCH (`true`, `false` or `"toggle"`). `true` zooms the selected pane, `false` unzooms its tab, and `"toggle"` zooms it if its tab isn't zoomed on it and unzooms otherwise, deciding from what the client shows. The default binding and menu item MUST pass `"toggle"`. `server.pane.zoom` MUST no longer exist.
- **FR-012** Moving focus out of a zoomed pane MUST still unzoom the tab first, through `PATCH /tabs/{id} {"zoomed": null}`.

### Actions

- **FR-013** A `server.` action's request MUST depend only on its arguments and the caller's context: its attachment, its selected tab or pane, and its current directory. An action that reads other shared state to decide what to send MUST be a `client.` action. The keymap spec MUST state this.

### CLI

- **FR-014** Every CLI flag for a field the entity can leave empty MUST distinguish three cases: flag absent leaves it alone, a bare flag clears it, and a flag with a value (including `""`) sets it. This applies to `--name` on `ship tab set` and `ship pane set`, and to `--zoomed`.

### Validation

- **FR-015** Every JSON request body MUST reject unknown fields.
- **FR-016** An invalid body MUST get 400 `validation`, and the error's data MUST list every problem found, each with a JSON Pointer to its location and a description.
- **FR-017** The OpenAPI document MUST describe each body as the server accepts it, including which fields allow `null`. Field descriptions MUST state the real defaults. For example, an absent `cwd` on a split uses the split pane's directory, not the home directory.
- **FR-018** An invalid query string MUST get 400 `validation` in the same `AppError` shape as a body, and MUST reject unknown parameters. It MAY report only the first problem.

### Release

- **FR-019** The protocol version MUST go up. No route keeps accepting the old shapes.
- **FR-020** `docs/agents/code.md` MUST gain an "Entity and request shapes" subsection stating FR-001 to FR-008, FR-013 and FR-014's rule. The user docs that show the old shapes, such as `ship pane zoom` in `docs/site/panes.md`, MUST be updated.

## Success criteria

- **SC-001** In the OpenAPI document, every input field whose name matches an entity field has that field's type, the only exception being config's `shell`.
- **SC-002** Every route with a JSON body or query string answers 400 `validation` to an unknown field or parameter, and no request with an unknown field changes state.
- **SC-003** A body with at least three independent mistakes gets one response listing all of them.
- **SC-004** `alt-f` zooms and unzooms the selected pane as it does today, and a script can zoom and unzoom a pane in a tab it isn't attached to.
- **SC-005** Every `server.` action in the default keymap passes FR-013's test.
- **SC-006** No existing valid request, rewritten in its new shape, is rejected.

## Open unknowns

- **U-001** Whether the OpenAPI document can mark exactly which fields take `null` (FR-007, FR-017) without hand-written schemas. If it can't, the paper must choose between hand-written schema overrides for those fields and a looser document. This is a feasibility question for the architecture paper, but its answer could change FR-017.
- **U-002** Whether `"toggle"` needs a spelling in the OpenAPI document that's distinguishable from a string value, so that FR-001's "same type" check (SC-001) treats `bool | "toggle"` as matching `bool`. Recommended: yes, the check allows that one widening for PATCH and actions.
