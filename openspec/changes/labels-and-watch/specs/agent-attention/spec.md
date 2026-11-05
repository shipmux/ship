# Spec Delta

## ADDED Requirements

### Requirement: Tag row items
A tag item `{ tag = "NAME" }` in agent rows SHALL show the value of that tag on the agent's pane, in the tag's colors, as terminal-client's Tag colors in the sidebar describes. A pane without that tag, or with an empty value, SHALL leave the item out like a token with no value.

#### Scenario: Branch in an agent entry
- **WHEN** the rows are `[["icon", "app", "state", { tag = "git.branch" }], ["tab"]]` and Claude Code's pane has `git.branch` `main`
- **THEN** its first line shows the icon, `claude`, its state and `main`

#### Scenario: Pane without the tag
- **WHEN** with those rows, an agent's pane has no `git.branch`
- **THEN** its first line shows the icon, app and state only
