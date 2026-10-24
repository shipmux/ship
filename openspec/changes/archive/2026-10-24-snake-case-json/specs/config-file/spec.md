# Spec Delta

## MODIFIED Requirements

### Requirement: Pane defaults
`[server.defaults.pane]` SHALL hold what a new pane starts with: `shell`, `scrollback_bytes`, `right_click_passthrough` and `exit_behavior`, defaulting to the login shell, 10,000,000, `false` and `"close"`. The server SHALL read the table each time it creates a pane, so a saved change applies to panes created afterwards and not to existing ones. A value given in the create request SHALL override the table for that pane.

#### Scenario: Change applies to the next pane
- **WHEN** a user sets `exit_behavior = "keep"` in `[server.defaults.pane]`, saves, and creates a pane on a running server
- **THEN** the new pane shows `exit_behavior: keep` and existing panes keep their values

#### Scenario: Create request overrides the table
- **WHEN** the table sets `exit_behavior = "keep"` and a script runs `ship pane create --exit-behavior close`
- **THEN** the new pane shows `exit_behavior: close`
