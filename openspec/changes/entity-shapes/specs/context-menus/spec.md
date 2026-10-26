# Spec Delta

## MODIFIED Requirements

### Requirement: Menus in the config
The client config SHALL accept `[client.menus]` with the targets `pane`, `tab` and `global`, each an ordered list of items. Each item SHALL be a table with exactly one action under `action`, written as in a key binding, and an optional `label`. An item without `label` SHALL take the hint its action would show on a key. An unknown target, an unknown key in an item, or an item without `action` SHALL be a config error with its line and column. Menus SHALL reload with the rest of `[client]`.

#### Scenario: Label from the action
- **WHEN** the file sets `pane = [{ action = { client.pane.zoom = { zoomed = "toggle" } } }]`
- **THEN** the item's label is the hint a key bound to that action would show

#### Scenario: Unknown target or key
- **WHEN** a file sets `[client.menus] pain = []`, or an item with `lable = "x"`, and the user runs `ship config check`
- **THEN** it reports the unknown key with its line and column and exits 1

### Requirement: Default and replaced menus
Ship's defaults SHALL define all three targets. A target in the user's file SHALL replace that target's default list whole, and targets the file doesn't name SHALL keep their defaults. An empty list SHALL mean that target has no menu.

#### Scenario: Replace one target
- **WHEN** the file sets `[client.menus] pane = [{ action = { client.pane.zoom = { zoomed = "toggle" } }, label = "fill" }]` and the user right-clicks a pane
- **THEN** the pane menu shows only `fill`, and the tab and global menus keep their default items

### Requirement: Default pane menu
With no `pane` target in the file, the pane menu SHALL be, in order:

| Label | Action |
| --- | --- |
| split right | `server.pane.create = { direction = "right", select = true }` |
| split down | `server.pane.create = { direction = "down", select = true }` |
| zoom | `client.pane.zoom = { zoomed = "toggle" }` |
| right-click passthrough | `server.pane.set = { right_click_passthrough = "toggle" }` |
| close | `server.pane.close = {}` |

#### Scenario: Default pane menu
- **WHEN** a user with no config right-clicks a pane
- **THEN** the menu shows split right, split down, zoom, right-click passthrough and close, in that order

