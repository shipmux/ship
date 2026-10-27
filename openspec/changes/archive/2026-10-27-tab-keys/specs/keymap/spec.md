# Spec Delta

## MODIFIED Requirements

### Requirement: Client actions
A `client.` action SHALL change only that client's view and SHALL be reachable only from keys and the mouse, except `client.pane.swap`, which SHALL resolve direction locally and submit the explicit server swap pair, `client.pane.zoom`, which SHALL resolve its switch locally and submit the tab's zoomed pane, and `client.tab.move`, which SHALL resolve its direction locally and submit one explicit tab move. None of them SHALL have a CLI form.

#### Scenario: Local resolution submits shared swap
- **WHEN** a user invokes `client.pane.swap` right in an unzoomed tab
- **THEN** that client resolves the neighbor using published geometry and local recency, while all viewers observe the resulting server-owned layout edit

#### Scenario: Local resolution submits shared zoom
- **WHEN** a user invokes `client.pane.zoom = { zoomed = "toggle" }` on a pane in an unzoomed tab
- **THEN** that client decides to zoom from the tab it shows and sends the pane as the tab's zoomed pane, and all viewers see the pane fill the tab

#### Scenario: Local resolution submits a shared tab move
- **WHEN** a user invokes `client.tab.move.direction = "up"` on a tab with a previous sibling
- **THEN** that client sends one `server.tab.move` placing the tab before that sibling, and all viewers see the new order

## ADDED Requirements

### Requirement: Relative tab moves
`client.tab.move` SHALL take a required `direction` of `"up"`, `"down"`, `"in"` or `"out"`. It SHALL move the selected tab, or the selected pane's tab, with its subtree and panes, in one tab move: `up` to just before its previous sibling, `down` to just after its next sibling, `in` to the last child of its previous sibling, and `out` to just after its parent, leaving its siblings in place. Folded siblings SHALL count. The moved tab, or the pane selected in it, SHALL stay selected.

#### Scenario: Move up
- **WHEN** the top level is `api`, `web`, `notes`, `notes` is selected, and the user presses `shift-k` in `tabs` mode
- **THEN** the top level is `api`, `notes`, `web`, and `notes` stays selected

#### Scenario: Move in
- **WHEN** `api` has children `server` and `tests`, `tests` is selected, and the user presses `shift-l` in `tabs` mode
- **THEN** `tests` becomes `server`'s last child with its panes, and its programs keep running

#### Scenario: Move into a folded sibling
- **WHEN** `web` is folded, `notes` follows it and is selected, and the user presses `shift-l` in `tabs` mode
- **THEN** `notes` becomes `web`'s last child, `web` unfolds in that client, and `notes`'s row is visible and selected

#### Scenario: Move out
- **WHEN** `api` has children `server` and `tests`, `server` is selected, and the user presses `shift-h` in `tabs` mode
- **THEN** `server` sits at the top level right after `api`, and `tests` stays under `api`

#### Scenario: Move from a selected pane
- **WHEN** a pane in `tests` is selected and the user presses `shift-k` in `tabs` mode
- **THEN** `tests` moves above `server`, and the same pane stays selected

#### Scenario: Folded siblings still count
- **WHEN** `web` is folded with children, `notes` follows it and is selected, and the user presses `shift-k`
- **THEN** `notes` moves to just before `web`, stepping over `web`'s hidden rows as one sibling

### Requirement: Tab moves at their turn
`client.tab.move` SHALL resolve its direction against the tree at its turn in the client's input order, then SHALL be handled exactly as a mouse drop to the same destination, including unfolding a folded new parent in that client. With no selection, or `up` on a first sibling, `down` on a last sibling, `in` without a previous sibling, or `out` on a top-level tab, it SHALL send no request and show no error.

#### Scenario: Repeated presses
- **WHEN** a tab with three previous siblings is selected and the user presses `shift-k` three times quickly
- **THEN** the tab moves up three places

#### Scenario: Nothing happens at an edge
- **WHEN** a top-level first tab is selected and the user presses `shift-k`, `shift-h` or `shift-l`, or a last sibling is selected and the user presses `shift-j`
- **THEN** no move request is sent, `ship tab list` output is unchanged, and no error is shown

#### Scenario: Same as a drop
- **WHEN** one tab is moved `in` from the keyboard, and the same tab in an identical tree is dropped on the same new parent with the mouse
- **THEN** both produce the same tree

### Requirement: Default tabs keys
Default `tabs` mode SHALL bind `j`/`k` to `client.tab.next`/`prev`, `h`/`l` to `client.tab.collapse`/`expand`, `esc` to normal, and `shift-k`, `shift-j`, `shift-l` and `shift-h` to `client.tab.move` `up`, `down`, `in` and `out`, with the labels `move up`, `move down`, `move in` and `move out`.

#### Scenario: Labels in tabs mode
- **WHEN** a user with no config presses `alt-g`
- **THEN** the status line shows `move up`, `move down`, `move in` and `move out` for the four shifted keys
