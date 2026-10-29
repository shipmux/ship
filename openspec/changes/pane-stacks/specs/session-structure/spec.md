## MODIFIED Requirements

### Requirement: Layout order and proportions
A tab's pane order SHALL be its binary split tree traversed first child before second, with first left in a horizontal split and above in a vertical split, and each slot's panes front first. Splits SHALL retain their stored proportions when the available area changes, subject to whole-cell rounding. Shrinking SHALL fit the existing tree into the available area without removing panes or changing stored proportions.

#### Scenario: Nested tree order
- **WHEN** a tab's layout is a left subtree containing A above B beside right pane C
- **THEN** inspection, pane cycling and the first-pane-derived tab label use A, B, C, not geometric row scanning

#### Scenario: Stack order
- **WHEN** a tab's layout is a slot holding `[A, X]` beside a slot holding C
- **THEN** inspection lists A, X, C

#### Scenario: Shrink and restore
- **WHEN** a viewing terminal shrinks until some panes have no visible content and later grows again
- **THEN** all pane IDs, programs, tree structure and stored proportions survive, and enlarged panes recover their proportions subject to rounding

### Requirement: Split placement
Pane creation SHALL take a place: split the anchor's slot right or down, or stack in front of it, defaulting to right. The CLI SHALL take `--place right|down|stack`, with `--direction` as another name for it. With only a tab target it SHALL use that tab's largest pane by content area, breaking ties by layout order, or create the first pane in an empty tab. A split that would leave either half with less than 1 column and 1 row of content SHALL be refused with "no space for new pane", without starting a program or changing the layout. A split SHALL put the new pane alone in its own slot.

#### Scenario: Explicit anchor
- **WHEN** a user runs `ship pane create --pane <id> --place down` with enough room for the split
- **THEN** the original pane's slot is above the new pane's slot in the same tab and existing unrelated splits remain

#### Scenario: Old flag name
- **WHEN** a user runs `ship pane create --pane <id> --direction down`
- **THEN** the pane is created as with `--place down`

#### Scenario: Largest pane
- **WHEN** a user creates a pane with only `--tab` in a viewed tab with differently sized panes
- **THEN** the largest content rectangle is split, with equal areas resolved by layout order

#### Scenario: Empty tab
- **WHEN** a user creates a pane in an empty tab
- **THEN** it becomes the tab's only pane

#### Scenario: No space for the split
- **WHEN** a user splits a pane right whose content is 2 columns wide
- **THEN** the command fails with "no space for new pane", no program starts, and the layout is unchanged

### Requirement: Closing split panes
Closing the last pane of a slot SHALL replace the slot's parent split with the sibling subtree, which takes its space; closing the last pane of the only slot SHALL leave the tab empty. Closing a pane with others in its slot SHALL leave the layout's shape unchanged, as pane-stacks' Closing a stacked pane describes.

#### Scenario: Close a nested split
- **WHEN** a user closes A in the layout `[A | [B / C]]`, with A alone in its slot
- **THEN** B above C fills the former tab area, A's program ends under the existing program-lifetime contract, and B and C keep running

### Requirement: Explicit pane swaps
The server swap operation SHALL take two explicit pane IDs in the same tab and exchange their places, slot and position in the stack, in one edit, preserving their IDs, programs, screens and the layout's shape and sizes. Adjacency SHALL NOT be required. The server SHALL NOT resolve direction or accept selection recency. Missing or cross-tab targets SHALL be rejected without a partial edit.

#### Scenario: Nonadjacent pair
- **WHEN** a caller requests a swap of two nonadjacent panes in the same tab
- **THEN** those panes exchange places while their programs and screens remain associated with their IDs

#### Scenario: Swap with a pane behind
- **WHEN** a caller swaps C, alone in its slot, with A, behind X in a slot holding `[X, A]`
- **THEN** the first slot holds `[X, C]` and A is alone in the other slot

#### Scenario: Stale geometry
- **WHEN** a caller resolves A and B from geometry, the layout changes, and the server then receives the pair while both remain in the same tab
- **THEN** the server exchanges A and B without re-evaluating their former direction

#### Scenario: Invalid pair
- **WHEN** either target is missing or the targets belong to different tabs
- **THEN** the request fails and neither tab's layout or zoom state changes

### Requirement: Create request bodies
`POST /api/v0/panes` SHALL take `{ "at", "place"?, "select"?, "pane"? }` and `POST /api/v0/tabs` SHALL take `{ "at"?, "starter"?, "select"?, "tab"? }`. `place` SHALL be `{"split": "right"}`, `{"split": "down"}` or `"stack"`, defaulting to `{"split": "right"}`; the body SHALL NOT take `direction`. The request's own fields SHALL sit at the top level, and the new entity's fields SHALL sit under `pane` or `tab`, using the entity's names. An absent `at` on `POST /api/v0/tabs` SHALL mean the end of the top level.

#### Scenario: Create a pane with settings
- **WHEN** a caller sends `POST /api/v0/panes` with `{"at": {"tab": "<t>"}, "pane": {"cwd": "/tmp", "exit_behavior": "keep"}}`
- **THEN** a pane is created in `<t>` starting in `/tmp` with `exit_behavior: keep`

#### Scenario: Stack a pane over HTTP
- **WHEN** a caller sends `POST /api/v0/panes` with `{"at": {"pane": "<p>"}, "place": "stack"}`
- **THEN** the new pane is in front of `<p>`'s slot

#### Scenario: Entity field at the top level
- **WHEN** a caller sends `POST /api/v0/panes` with `{"at": {"tab": "<t>"}, "cwd": "/tmp"}`
- **THEN** the top-level `cwd` is ignored and the pane is created using the normal directory default; only `pane.cwd` sets its starting directory

#### Scenario: Create a named tab
- **WHEN** a caller sends `POST /api/v0/tabs` with `{"tab": {"name": "notes"}}`
- **THEN** a top-level tab named `notes` is appended

### Requirement: Tab zoom
A tab's `zoomed` attribute SHALL be the front pane of the slot filling it, or `null`. Setting it to a pane in that tab SHALL bring that pane to its slot's front and zoom the slot, and `null` SHALL unzoom. A pane in another tab SHALL get 422 `invalid_structure`, and a pane that doesn't exist SHALL get 404 `not_found`, changing nothing. `POST /api/v0/panes/{id}/zoom` and `ship pane zoom` SHALL NOT exist.

#### Scenario: Zoom and unzoom over HTTP
- **WHEN** a caller sends `PATCH /api/v0/tabs/{t}` with `{"zoomed": "<p>"}` for a pane `<p>` in `<t>`, then `{"zoomed": null}`
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

### Requirement: Request body validation
A body that doesn't decode SHALL get 400 `validation`, with `data.errors` listing every problem found, each with `detail` and, where it has a location, a JSON Pointer `pointer`. A request that fails validation SHALL change nothing. A body sent without `Content-Type: application/json` SHALL get 400 `validation` with one entry.

#### Scenario: Four mistakes at once
- **WHEN** a caller sends `POST /api/v0/panes` with `{"place": "up", "pane": {"exit_behavior": "linger", "scrollback_bytes": "lots", "exitBehavior": "keep"}}`
- **THEN** one 400 `validation` response lists the missing `at`, the bad `place`, `linger` and `lots`, each with its pointer, ignoring `exitBehavior`, and no pane is created

#### Scenario: Not JSON
- **WHEN** a caller sends a body that isn't valid JSON
- **THEN** it receives 400 `validation` with one entry carrying the syntax message

#### Scenario: No content type
- **WHEN** a caller sends `PATCH /api/v0/panes/{id}` with a JSON body and no `Content-Type` header
- **THEN** it receives 400 `validation` with one entry, not 415, and the pane is unchanged
