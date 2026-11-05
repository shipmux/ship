# Spec Delta

## MODIFIED Requirements

### Requirement: Agent settings
`[client.agents]` SHALL configure agent-attention. `rows` SHALL be a list of lines, each a list of the tokens `icon`, `app`, `state`, `tab`, `pane`, `title` and `msg`, or tag items written `{ tag = "NAME" }`, defaulting to `[["icon", "app", "state"], ["tab"]]`. `delay_ms` SHALL be the notification delay in milliseconds, defaulting to 1000. `popups` SHALL turn popups on or off, defaulting to off. An unknown token or key SHALL fail loading with its line, column and the accepted values, like every other config error.

#### Scenario: Misspelled token
- **WHEN** the file sets `rows = [["icon", "apps"]]` and the user runs `ship config check`
- **THEN** it reports `apps` with its line and column, lists the accepted tokens and exits 1

#### Scenario: No agents table
- **WHEN** the file has no `[client.agents]`
- **THEN** entries use the default rows, notifications wait 1 second, and no popups are written

#### Scenario: Tag item
- **WHEN** the file sets `rows = [["icon", "app", { tag = "git.branch" }]]`
- **THEN** it loads, and agent entries show their pane's `git.branch` after the app

## ADDED Requirements

### Requirement: Tab settings
`[client.tabs]` SHALL configure the sidebar's tab entries. `rows` SHALL be a list of lines, each a list of the bare token `name` or tag items written `{ tag = "NAME" }`, defaulting to `[["name"]]`. A tag item's name SHALL follow the tags capability's name rules. An unknown token or key, or an invalid tag name, SHALL fail loading with its line and column, like every other config error.

#### Scenario: Two-line tab rows
- **WHEN** the file sets `[client.tabs] rows = [["name"], [{ tag = "git.branch" }]]`
- **THEN** `ship config check` passes, and tab entries show the branch under the name

#### Scenario: Unknown tab token
- **WHEN** the file sets `[client.tabs] rows = [["title"]]` and the user runs `ship config check`
- **THEN** it reports `title` with its line and column and exits 1

#### Scenario: Reload
- **WHEN** a user adds `[client.tabs]` rows and saves while a client is attached
- **THEN** the sidebar's tab entries change without a restart
