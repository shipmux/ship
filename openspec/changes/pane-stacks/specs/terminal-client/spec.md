## MODIFIED Requirements

### Requirement: Pane frames and screens
A tab with two or more slots SHALL draw labeled borders around its visible splits with a visually distinct selected-pane border and shared adjoining edges. A slot's top border SHALL list the labels of all its panes, front first, separated by ` · `, with the front's label marked. A lone or zoomed slot SHALL fill its area without a border, and SHALL show a one-row title row listing its panes the same way only while it holds more than one pane. Each visible pane SHALL display its own faithful screen, with the selected pane's cursor placed inside its content rectangle. A border held by a drag SHALL be drawn in the selected-border color until release.

#### Scenario: Editor beside shell and agent
- **WHEN** a user runs Neovim, a shell and Pi in three splits, types into each and closes one
- **THEN** each pane keeps its own screen, remaining programs redraw into the reclaimed area, and input and the visible cursor follow the selection

#### Scenario: Alternate screen in one split
- **WHEN** one split enters and exits an alternate-screen program
- **THEN** that split switches cleanly and its neighbors' screens are not disturbed

#### Scenario: Borderless lone pane
- **WHEN** a tab has one slot holding one pane, or is zoomed onto a slot holding one pane
- **THEN** the visible pane has no border and no title row

#### Scenario: Stack titles in a border
- **WHEN** a slot beside another holds `[X, A, B]`
- **THEN** its top border reads `X · A · B` with X marked, and only X's screen is drawn

#### Scenario: Lone stacked slot
- **WHEN** a tab's only slot holds `[X, A]`, and later A closes
- **THEN** a title row reading `X · A` sits above X's screen, then the title row is gone and X fills the area

#### Scenario: Held border
- **WHEN** a user presses and holds a border between two unselected panes
- **THEN** that border is drawn in the selected-border color until release
