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

### Requirement: Fitting agent lines
When an agent line's values are wider than the sidebar, each value except the icon SHALL get a share of the width left after the icon and separators, given one cell at a time in turns, so values that fit their share stay whole. A value cut to its share SHALL end in `…`, except `tab`, which SHALL lose its start behind `…`. A cut value SHALL keep its colors, and a value left no cells SHALL disappear with its separator.

#### Scenario: Long branch tag
- **WHEN** the rows are `[["icon", "app", "state", { tag = "git.branch" }]]`, the sidebar is 26 columns, and a working `demo` agent's pane has `git.branch` `labels-and-watch`
- **THEN** its line reads `● demo working labels-a…`

#### Scenario: Path beside a tag
- **WHEN** a line holds `tab` and a tag that together don't fit
- **THEN** both are cut, the path from its start and the tag at its end
