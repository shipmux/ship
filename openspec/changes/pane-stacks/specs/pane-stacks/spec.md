## ADDED Requirements

### Requirement: Slots hold stacks
Every leaf of a tab's layout SHALL be a slot holding one or more panes in order. The first pane SHALL be the front, and only the front SHALL be drawn. A slot SHALL never be empty. The front of a slot SHALL be the same for every client.

#### Scenario: Two clients see one front
- **WHEN** two clients view the same tab and one cycles a stack
- **THEN** both show the same front pane

### Requirement: Slot sizes
A slot's size SHALL come from the layout alone. The front pane's terminal SHALL be sized to the slot. Panes behind the front SHALL keep the size they last had and SHALL be resized to the slot when they come to the front.

#### Scenario: Pane comes forward after a resize
- **WHEN** a slot holds `[X, A]`, the user resizes the slot, and then cycles
- **THEN** `A` is shown at the slot's new size

### Requirement: Slot layout shape
`ship tab get`, `GET /api/v0/tabs` and every other layout output SHALL show each leaf as `{"kind": "slot", "panes": [...]}`, front first.

#### Scenario: Read a stack back
- **WHEN** a slot holds `[X, A, B]` and a user runs `ship tab get`
- **THEN** its layout has a `"kind": "slot"` leaf whose `panes` lists `X`, `A`, `B` in that order

### Requirement: Stacking create
`ship pane create --place stack` and `server.pane.create = { place = "stack" }` SHALL put the new pane in front of the anchor pane's slot, at the slot's size, without splitting. With only a tab target, `stack` SHALL stack onto the pane a split would have split, or create the first pane in an empty tab. The anchor's program SHALL keep running behind the new pane.

#### Scenario: Stack onto the selected pane
- **WHEN** pane `X` is selected and the user presses `alt-s`
- **THEN** a new shell pane is in front of `X`'s slot, at the slot's size, and selected, and `X`'s program keeps running

#### Scenario: Stack with a tab target
- **WHEN** a user runs `ship pane create --tab <t> --place stack` in a tab with differently sized panes
- **THEN** the new pane is stacked in front of the largest pane's slot

### Requirement: Cycling a stack
`ship pane cycle --pane X --direction next|prev`, `server.pane.cycle = { direction = "next" | "prev" }` and `POST /api/v0/panes/{id}/cycle` with `{"direction": "next" | "prev"}` SHALL rotate `X`'s slot one step and answer with the tab. `direction` SHALL be required. `next` SHALL turn `[X, A, B]` into `[A, B, X]` and `prev` SHALL turn it into `[B, X, A]`. Cycling a slot with one pane SHALL change nothing and succeed. The pane SHALL default like other pane commands.

#### Scenario: Next and previous
- **WHEN** a slot holds `[X, A, B]` and a user runs `ship pane cycle --pane X --direction next`, then `ship pane cycle --pane A --direction prev`
- **THEN** the slot holds `[A, B, X]`, then `[X, A, B]`

#### Scenario: Missing direction
- **WHEN** a user runs `ship pane cycle --pane X`
- **THEN** the CLI rejects the arguments before any request, exits 2 and nothing changes

#### Scenario: One pane
- **WHEN** a user cycles a slot that holds only `X`
- **THEN** the command succeeds and the layout is unchanged

### Requirement: Selecting a pane behind the front
Selecting a pane behind its slot's front, by any route, SHALL rotate the slot until that pane is in front, keeping the order of the others, in the same edit as the selection.

#### Scenario: Select from a script
- **WHEN** a slot holds `[X, A, B]` and a script runs `ship view select --attachment <id> B`
- **THEN** the slot holds `[B, X, A]` and that client has `B` selected

### Requirement: Closing a stacked pane
Closing a pane SHALL remove it from its slot. If other panes remain, the next one SHALL become the front. If it was the last pane, the slot SHALL be removed as session-structure's Closing split panes describes.

#### Scenario: Close down a stack
- **WHEN** a slot holds `[X, A]`, `X` is selected and the user closes `X`, then closes `A`
- **THEN** `A` is shown and selected, then the slot is gone and its neighbor takes the space

### Requirement: Moving panes
`ship pane move --pane X --onto Y` SHALL take `X` out of its slot and put it in front of `Y`'s slot. `ship pane move --pane X --beside Y --direction right|down` SHALL take `X` out of its slot and split `Y`'s slot, with `X` alone in the new slot on that side. `Y` MAY be in any tab. A move SHALL be one edit that keeps `X`'s program, terminal contents, ID and selections. An emptied old slot SHALL be removed as in Closing a stacked pane.

#### Scenario: Onto a stack in another tab
- **WHEN** `X` runs `top` alone in its slot and a user runs `ship pane move --pane X --onto Y` for a pane `Y` in another tab
- **THEN** `X` is in front of `Y`'s slot with the same ID and `top` still running, and `X`'s old slot is gone

#### Scenario: Out of a stack beside a pane
- **WHEN** `X` is in a stack and a user runs `ship pane move --pane X --beside Y --direction right`
- **THEN** `Y`'s slot is split and `X` sits alone in the new slot to its right

#### Scenario: Onto itself
- **WHEN** a user runs `ship pane move --pane X --onto X`
- **THEN** it fails with 422 `invalid_structure`, exits 1, and `ship tab get` shows the layout unchanged

### Requirement: Keeping moved selections visible
If any client has the moved pane selected and the destination tab's zoom would hide it, the move SHALL unzoom the destination in that same edit, including a move within one tab. Otherwise moving onto a stack SHALL preserve zoom under the existing slot rules.

#### Scenario: Selected pane moves out of a surviving zoomed stack
- **WHEN** X is selected in zoomed stack `[X, A]` and moves onto Y in another slot of the same tab
- **THEN** the stacks hold `[A]` and `[X, Y]`, the tab unzooms and X stays selected

#### Scenario: Selected pane enters a tab zoomed on another slot
- **WHEN** selected X moves onto Y in another tab zoomed on a different slot Z
- **THEN** X stays selected in front of Y, and the destination tab unzooms

#### Scenario: Unselected move preserves zoom
- **WHEN** no client has X selected and X moves onto Y in a tab zoomed on another surviving slot Z
- **THEN** Z remains zoomed and X is in front of Y's slot

### Requirement: Move requests
`POST /api/v0/panes/{id}/move` SHALL take `{"at": <pane>, "place": "stack" | {"split": "right" | "down"}}`, with no default for `place`, and answer with the destination tab. The CLI SHALL require exactly one of `--onto` and `--beside`, and `--direction` only with `--beside`.

#### Scenario: Move over HTTP
- **WHEN** a caller sends `POST /api/v0/panes/<x>/move` with `{"at": "<y>", "place": {"split": "down"}}`
- **THEN** `<x>` sits alone in a new slot below `<y>`'s slot, and the response is `<y>`'s tab

#### Scenario: Both destinations
- **WHEN** a user runs `ship pane move --pane X --onto Y --beside Z --direction down`
- **THEN** the CLI rejects the arguments before any request, exits 2 and nothing changes
