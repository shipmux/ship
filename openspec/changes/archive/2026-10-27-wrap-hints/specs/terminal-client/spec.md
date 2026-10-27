## MODIFIED Requirements

### Requirement: Status line
The status line SHALL be an overlay drawn over the bottom rows of the pane area, the area right of the sidebar, never over the sidebar. It SHALL take one row unless its hints need more, as Hint layout says, and never more rows than the pane area has. It SHALL NOT change the reported area or any pane's geometry, and SHALL hide the pane cells beneath it only while it is shown.

#### Scenario: Overlay leaves sizes alone
- **WHEN** a pane runs `stty size` in normal mode, and again after the user presses `alt-r`
- **THEN** both report the same size, and the status line covers only the bottom of the pane area, not the sidebar

### Requirement: Mode key hints
Outside normal mode the status line SHALL show the active mode's name and key hints for that mode: each bound chord in the config file's order, labeled by its binding's `label` when set, else by a word from its action: the mode's name, the direction, the key sent, else the action's name. A hint SHALL NOT be split across rows or cut partway.

#### Scenario: Active mode
- **WHEN** the user presses `alt-r`
- **THEN** the status line appears showing `resize` and the hints `h left`, `j down`, `k up`, `l right`, `shift-h sidebar left`, `shift-l sidebar right`, `shift-j split down`, `shift-k split up` and `esc normal` in that order, and disappears after `esc`

#### Scenario: Custom binding hint
- **WHEN** the file binds `"f" = { action = { client.pane.zoom = { zoomed = "toggle" } } }` in `resize` mode and the user enters it
- **THEN** the hints include `f zoom`

#### Scenario: Set label
- **WHEN** the file binds `"f" = { action = { client.pane.zoom = { zoomed = "toggle" } }, label = "fill" }` in `resize` mode and the user enters it
- **THEN** the hints include `f fill` and not `f zoom`

## ADDED Requirements

### Requirement: Hint layout
The status line SHALL use one row when the mode's name, messages and all hints fit. Otherwise its last row SHALL hold the mode's name and messages, with hints above it in aligned columns as wide as the widest hint, as many as fit, filled left to right in config order. Hints wider than the pane area SHALL be omitted whole before layout. The overlay SHALL grow upward. When the pane area is too short, hints SHALL be cut whole from the end, and the last row SHALL stay.

#### Scenario: One row when it fits
- **WHEN** the pane area is wide enough for `resize` and all its hints on one row and the user presses `alt-r`
- **THEN** the status line is one row, as before this change

#### Scenario: Columns when it doesn't
- **WHEN** a 120-column client with the default 26-column sidebar enters `tabs` mode
- **THEN** the last row shows `TABS`, the rows above it show every hint whole with chords aligned in columns, and `esc normal` is visible

#### Scenario: Hint wider than the pane area
- **WHEN** a hint's display width exceeds the pane area's width
- **THEN** that hint is omitted whole, and the remaining hints use aligned columns sized to their widest hint and stay in config order

#### Scenario: Short pane area
- **WHEN** the pane area is 3 rows tall and the hints need more
- **THEN** the status line takes 3 rows, its last row still shows the mode's name, and the hints that don't fit are left off whole from the end
