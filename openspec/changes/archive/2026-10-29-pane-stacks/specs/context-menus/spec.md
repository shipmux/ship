## MODIFIED Requirements

### Requirement: Default pane menu
With no `pane` target in the file, the pane menu SHALL be, in order:

| Label | Action |
| --- | --- |
| split right | `server.pane.create = { place = "right", select = true }` |
| split down | `server.pane.create = { place = "down", select = true }` |
| stack | `server.pane.create = { place = "stack", select = true }` |
| edit scrollback | the same action as the default `alt-e` binding |
| zoom | `client.pane.zoom = { zoomed = "toggle" }` |
| right-click passthrough | `server.pane.set = { right_click_passthrough = "toggle" }` |
| close | `server.pane.close = {}` |

#### Scenario: Default pane menu
- **WHEN** a user with no config right-clicks a pane
- **THEN** the menu shows split right, split down, stack, edit scrollback, zoom, right-click passthrough and close, in that order
