# Spec Delta

## MODIFIED Requirements

### Requirement: Slot layout shape
`ship tab get`, `GET /api/tabs` and every other layout output SHALL show each leaf as `{"kind": "slot", "panes": [...]}`, front first.

#### Scenario: Read a stack back
- **WHEN** a slot holds `[X, A, B]` and a user runs `ship tab get`
- **THEN** its layout has a `"kind": "slot"` leaf whose `panes` lists `X`, `A`, `B` in that order

### Requirement: Cycling a stack
`ship pane cycle --pane X --direction next|prev`, `server.pane.cycle = { direction = "next" | "prev" }` and `POST /api/panes/{id}/cycle` with `{"direction": "next" | "prev"}` SHALL rotate `X`'s slot one step and answer with the tab. `direction` SHALL be required. `next` SHALL turn `[X, A, B]` into `[A, B, X]` and `prev` SHALL turn it into `[B, X, A]`. Cycling a slot with one pane SHALL change nothing and succeed. The pane SHALL default like other pane commands.

#### Scenario: Next and previous
- **WHEN** a slot holds `[X, A, B]` and a user runs `ship pane cycle --pane X --direction next`, then `ship pane cycle --pane A --direction prev`
- **THEN** the slot holds `[A, B, X]`, then `[X, A, B]`

#### Scenario: Missing direction
- **WHEN** a user runs `ship pane cycle --pane X`
- **THEN** the CLI rejects the arguments before any request, exits 2 and nothing changes

#### Scenario: One pane
- **WHEN** a user cycles a slot that holds only `X`
- **THEN** the command succeeds and the layout is unchanged

### Requirement: Move requests
`POST /api/panes/{id}/move` SHALL take `{"at": <pane>, "place": "stack" | {"split": "right" | "down"}}`, with no default for `place`, and answer with the destination tab. The CLI SHALL require exactly one of `--onto` and `--beside`, and `--direction` only with `--beside`.

#### Scenario: Move over HTTP
- **WHEN** a caller sends `POST /api/panes/<x>/move` with `{"at": "<y>", "place": {"split": "down"}}`
- **THEN** `<x>` sits alone in a new slot below `<y>`'s slot, and the response is `<y>`'s tab

#### Scenario: Both destinations
- **WHEN** a user runs `ship pane move --pane X --onto Y --beside Z --direction down`
- **THEN** the CLI rejects the arguments before any request, exits 2 and nothing changes
