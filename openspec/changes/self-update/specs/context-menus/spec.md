# Spec Delta

## MODIFIED Requirements

### Requirement: Default global menu
With no `global` target in the file, the global menu SHALL be, in order:

| Label | Action |
| --- | --- |
| new tab | `server.tab.create = { starter = "shell", select = true }` |
| hide sidebar | `client.sidebar.toggle = {}` |
| updates | `client.updates.open = {}` |
| reload config | `client.config.reload = {}` |
| detach | `client.detach = {}` |

#### Scenario: Default global menu
- **WHEN** a user with no config right-clicks the `tabs` heading
- **THEN** the menu shows new tab, hide sidebar, updates, reload config and detach, in that order
