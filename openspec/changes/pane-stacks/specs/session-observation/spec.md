## MODIFIED Requirements

### Requirement: Selection repair
When a client's selected pane is removed, its selection SHALL move to the next pane in its slot if any remain, else into the sibling subtree that takes the slot's space, choosing the leaf toward the closed side on same-axis splits and the first child otherwise. With no own panes left it SHALL fall back to the tab. Removed tabs or ancestors SHALL repair to the nearest surviving ancestor, or nothing. Moved surviving selections SHALL stay selected. A selection on a pane behind its slot's front SHALL repair to that front. A selection hidden by shared zoom SHALL repair to the zoomed pane.

#### Scenario: Selected pane removed
- **WHEN** A is selected in `[A | [B / C]]`, alone in its slot, and A is removed
- **THEN** the client's selection moves to B in the subtree taking A's space

#### Scenario: Selected pane removed from a stack
- **WHEN** X is selected in a slot holding `[X, A]` and X is removed
- **THEN** the client's selection moves to A

#### Scenario: Selected pane pushed behind
- **WHEN** one client has X selected, X is the front of its slot, and another client stacks a new pane N onto X's slot
- **THEN** the first client's selection moves to N

#### Scenario: Last pane in a tab removed
- **WHEN** a client's selected pane is the only pane in its tab and is removed
- **THEN** its selection becomes the tab and the client shows the no-pane hint

#### Scenario: Selected tab's ancestor removed
- **WHEN** an ancestor of a client's selected tab is removed
- **THEN** its selection becomes the nearest surviving ancestor, or nothing if none survives

#### Scenario: Selected tab moved
- **WHEN** a client's selected pane's tab moves to the top level
- **THEN** the client keeps that pane selected

#### Scenario: Shared zoom hides selection
- **WHEN** one client zooms A while another viewing that tab has B selected
- **THEN** the other client's selection repairs to A

### Requirement: Published pane geometry
The server SHALL publish each viewed tab's size and visible pane frame/content rectangles in layout order, relative to the tab area, as optional `geometry` on that tab recursively. Only each slot's front pane SHALL have an entry. A borderless slot holding more than one pane SHALL keep its top row out of the content rectangle for its title row. Unviewed tabs SHALL omit geometry; a viewed empty tab SHALL include its size and an empty pane map. `Replica` SHALL NOT contain a parallel geometry map.

#### Scenario: Viewed child under unviewed ancestors
- **WHEN** a viewer selects a pane in the grandchild of an unviewed tab
- **THEN** the nested grandchild carries its own geometry in the same replica as its layout and viewer record, and its unviewed ancestors omit geometry

#### Scenario: Panes behind have no geometry
- **WHEN** a viewed tab's only slot holds `[X, A]`
- **THEN** its geometry has an entry for X only, and X's content rectangle starts one row below its frame

#### Scenario: Last viewer leaves a tab
- **WHEN** the last viewer of a tab selects another tab or detaches
- **THEN** subsequent publications and tab inspection omit that tab's geometry while its programs retain their last terminal sizes

### Requirement: Shared zoom
A tab's zoomed slot, set as session-structure's Tab zoom describes, SHALL fill the tab for all viewers, showing its front, while other programs keep running. Pane focus or next from the zoomed pane SHALL unzoom before moving, and a directional client swap SHALL unzoom before resolving its neighbor, even when none is found. Splitting, swapping or resizing in a tab SHALL unzoom it before the edit, as SHALL removing the zoomed slot. Cycling, stacking onto the zoomed slot, and closing or moving out a pane SHALL keep the zoom, with `zoomed` naming the zoomed slot's new front. A move SHALL instead unzoom its destination if that zoom would hide the moved pane and any client has that pane selected, including a move within one tab. Selecting another tab SHALL leave zoom unchanged.

#### Scenario: Two viewers zoom together
- **WHEN** one client zooms its selected pane
- **THEN** both viewers see that pane alone and hidden selected panes repair to it

#### Scenario: Move out of zoom
- **WHEN** two clients concurrently request directional focus out of a zoomed pane
- **THEN** the tab becomes unzoomed, does not toggle back into zoom, and each client moves using unzoomed geometry

#### Scenario: Layout edit ends zoom
- **WHEN** a valid split create, swap or resize operation is performed in a zoomed tab
- **THEN** the tab unzooms and all viewers see the edited layout

#### Scenario: Stack edits keep zoom
- **WHEN** X's slot is zoomed and the user cycles it, stacks a new pane onto it, or opens edit-scrollback on X
- **THEN** the tab stays zoomed and shows the slot's new front

#### Scenario: Front closes in a zoomed stack
- **WHEN** a slot holding `[X, A]` is zoomed and X closes
- **THEN** the tab stays zoomed on A

#### Scenario: Another slot closes
- **WHEN** X's slot is zoomed and a pane in another slot of the same tab closes
- **THEN** the tab stays zoomed

#### Scenario: Zoomed slot removed
- **WHEN** X is alone in its zoomed slot and X closes
- **THEN** the tab unzooms

#### Scenario: Visit another tab
- **WHEN** a client leaves a zoomed tab to view a different tab
- **THEN** the original tab stays zoomed for its other viewers and for a later return

#### Scenario: Zoom from a script
- **WHEN** a script runs `ship tab set --tab <t> --zoomed <p>` for a tab no client is attached to, then attaches a client to it
- **THEN** the client shows `<p>` filling the tab

### Requirement: Select behind a zoom
Selecting a pane in another slot of a zoomed tab, through the views route or `ship view select`, SHALL unzoom that tab for every viewer and select the pane, in one edit. Selecting a pane in the zoomed slot SHALL bring it to the front and keep the zoom.

#### Scenario: Script selects a hidden pane
- **WHEN** A is zoomed in a tab with B in another slot, two clients view the tab, and a script selects B for one of them
- **THEN** both clients see the tab unzoomed, and the script's client has B selected

#### Scenario: Select within the zoomed slot
- **WHEN** a zoomed slot holds `[X, A]` and a script selects A
- **THEN** the tab stays zoomed, now on A, and the script's client has A selected
