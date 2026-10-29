## ADDED Requirements

### Requirement: Stack title clicks
A left click on a pane's title in a slot's title row SHALL select that pane for that client, bringing it to the front as pane-stacks' Selecting a pane behind the front describes, and SHALL NOT reach any program.

#### Scenario: Click a title behind the front
- **WHEN** a slot holds `[X, A, B]` and the user clicks `B`'s title in its border
- **THEN** `B` is shown and selected, and the slot holds `[B, X, A]`

## MODIFIED Requirements

### Requirement: Border drags
A press on a cell of one border between panes SHALL grab that border, unless the cell holds the title of a pane behind its slot's front, where the press SHALL be a title click. A press on a cell where borders meet SHALL grab every border through that cell, each moving in its own axis. A left press on a border SHALL NOT change which pane is selected and SHALL NOT reach any program, including while earlier input is still waiting. A lone or zoomed slot has no border, so there SHALL be nothing to grab.

#### Scenario: Selection stays
- **WHEN** the right pane is selected, a delayed create is still waiting, and the user presses on the border and drags
- **THEN** the right pane stays selected and no program receives the press

#### Scenario: Junction
- **WHEN** a user presses on the center junction of a 2x2 grid and drags diagonally
- **THEN** both the vertical and the horizontal edge move

#### Scenario: Title on a border
- **WHEN** a slot's top border is also a border between two slots and the user presses on the title of a pane behind its front
- **THEN** that pane is selected and brought forward, and no border is grabbed
