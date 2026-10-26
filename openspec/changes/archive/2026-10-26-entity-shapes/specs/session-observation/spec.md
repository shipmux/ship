# Spec Delta

## MODIFIED Requirements

### Requirement: Shared zoom
A tab's zoomed pane, set as session-structure's Tab zoom describes, SHALL fill the tab for all viewers while other programs keep running. Pane focus or next from the zoomed pane SHALL unzoom before moving, and a directional client swap SHALL unzoom before resolving its neighbor, even when none is found. Creating, closing, swapping or resizing a pane SHALL unzoom that tab before the edit. Selecting another tab SHALL leave zoom unchanged.

#### Scenario: Two viewers zoom together
- **WHEN** one client zooms its selected pane
- **THEN** both viewers see that pane alone and hidden selected panes repair to it

#### Scenario: Move out of zoom
- **WHEN** two clients concurrently request directional focus out of a zoomed pane
- **THEN** the tab becomes unzoomed, does not toggle back into zoom, and each client moves using unzoomed geometry

#### Scenario: Layout edit ends zoom
- **WHEN** a valid create, close, swap or resize operation is performed in a zoomed tab
- **THEN** the tab unzooms and all viewers see the edited layout

#### Scenario: Visit another tab
- **WHEN** a client leaves a zoomed tab to view a different tab
- **THEN** the original tab stays zoomed for its other viewers and for a later return

#### Scenario: Zoom from a script
- **WHEN** a script runs `ship tab set --tab <t> --zoomed <p>` for a tab no client is attached to, then attaches a client to it
- **THEN** the client shows `<p>` filling the tab
