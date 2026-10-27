# Spec Delta

## MODIFIED Requirements

### Requirement: Local folding and selected-row visibility
Fold state SHALL be independent for each client. `client.tab.collapse` and `client.tab.expand` SHALL fold/unfold the selected tab's row in any mode where bound, and otherwise select as Parent and child hops describes. Selecting a descendant by any means SHALL expand its ancestors, so the selected row is visible. Folding SHALL NOT introduce a separate cursor, and folding or unfolding a row SHALL NOT change the selection.

#### Scenario: Two clients fold differently
- **WHEN** two clients view the same tree and one folds a tab
- **THEN** the other client's folds and selection are unchanged

#### Scenario: Open a folded descendant
- **WHEN** a client selects a tab beneath a folded ancestor
- **THEN** that ancestor expands and the selected tab's row becomes visible

#### Scenario: Tabs mode uses the selection
- **WHEN** a client enters default `tabs` mode on an open tab with children and presses `j`, `h`, `l` and `esc`
- **THEN** `j` moves the selection to the next visible row, `h` and `l` fold and unfold that row when it has children, and `esc` returns to normal on the same selection without Enter

## ADDED Requirements

### Requirement: Parent and child hops
On a folded row or a tab without children, `client.tab.collapse` SHALL select the tab's parent, and on such a top-level tab it SHALL do nothing. On an open row with children, `client.tab.expand` SHALL select the tab's first child, and on a tab without children it SHALL do nothing. A selected tab SHALL land as `client.tab.next` lands. With a pane selected, both SHALL act on its tab.

#### Scenario: Collapse from a child selects the parent
- **WHEN** `api` has children `server` and `tests`, `server` is selected, and the client invokes `client.tab.collapse`
- **THEN** `api` is selected where `client.tab.next` would land on it, and no row's fold changes

#### Scenario: Collapse on a folded row selects the parent
- **WHEN** a nested tab with children is folded and selected, and the client invokes `client.tab.collapse`
- **THEN** its parent is selected and the nested tab stays folded

#### Scenario: Collapse at the top level
- **WHEN** a top-level tab that is folded, or has no children, is selected and the client invokes `client.tab.collapse`
- **THEN** the selection and folds are unchanged

#### Scenario: Expand on an open row selects the first child
- **WHEN** `web` is open with children and selected, and the client invokes `client.tab.expand`
- **THEN** `web`'s first child is selected where `client.tab.next` would land on it

#### Scenario: Expand on a leaf
- **WHEN** a tab without children is selected and the client invokes `client.tab.expand`
- **THEN** the selection and folds are unchanged

#### Scenario: Hops from a selected pane
- **WHEN** a pane in `server` is selected and the client invokes `client.tab.collapse`
- **THEN** `api` is selected, as if `server` itself were selected
