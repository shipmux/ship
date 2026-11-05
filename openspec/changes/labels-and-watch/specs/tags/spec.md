# Spec Delta

## Purpose

Let scripts put short text, optionally colored, on tabs and panes, which every client and watcher receives and the sidebar shows where the user's config asks. A **tag** is a named value with an optional foreground and background color.

## ADDED Requirements

### Requirement: Tags on tabs and panes
Every tab and pane SHALL carry tags: a map from a tag name to an object holding a string `value` and an optional `fg` and `bg`. They SHALL appear as `tags` in `ship tab get`, `ship tab list`, `ship pane get` and the state every client and watcher receives. A tab or pane with no tags SHALL omit the field.

#### Scenario: A pane with two tags
- **WHEN** a pane has tags `git.branch` with value `main` and `git.ahead` with value `2`
- **THEN** `ship pane get` shows `"tags": {"git.branch": {"value": "main"}, "git.ahead": {"value": "2"}}`

#### Scenario: No tags
- **WHEN** a tab has no tags
- **THEN** `ship tab get` output has no `tags` field

### Requirement: Tag names
A tag name SHALL be any non-empty string without `=` or control characters. Ship SHALL give no meaning to `.` or any other character in a name. A request with an invalid name SHALL fail with 400 `validation` and change nothing.

#### Scenario: Dotted and plain names
- **WHEN** a script sets tags named `git.branch` and `todo`
- **THEN** both are stored as given, and neither is treated as nested

#### Scenario: Name with an equals sign
- **WHEN** a caller patches a pane with `{"tags": {"a=b": {"value": "x"}}}`
- **THEN** it receives 400 `validation` and the pane is unchanged

#### Scenario: Empty name
- **WHEN** a caller patches a pane with `{"tags": {"": {"value": "x"}}}`
- **THEN** it receives 400 `validation` and the pane is unchanged

### Requirement: Tag values
A tag value SHALL be any string without control characters, including the empty string. A request with a control character in a value SHALL fail with 400 `validation` and change nothing. Bidirectional formatting characters SHALL be removed from a value before it is stored.

#### Scenario: Control character in a value
- **WHEN** a caller patches a pane with `{"tags": {"k": {"value": "a\nb"}}}`
- **THEN** it receives 400 `validation` and the pane is unchanged

#### Scenario: Bidirectional override in a value
- **WHEN** a caller sets tag `k` to a value containing U+202E
- **THEN** the stored value doesn't contain it

### Requirement: Tag colors
A tag's `fg` and `bg` SHALL each be absent or one of the 16 named terminal colors. Any other color, including hex and palette numbers, SHALL fail with 400 `validation` and change nothing. `null` on `fg` or `bg` SHALL also fail with 400 `validation`.

#### Scenario: Named colors
- **WHEN** a caller sets `{"value": "FAIL", "fg": "white", "bg": "red"}` on tag `gh.checks`
- **THEN** `ship pane get` shows that tag with those colors

#### Scenario: Hex color
- **WHEN** a caller sets `{"value": "x", "fg": "#ff0000"}`
- **THEN** it receives 400 `validation` and nothing changes

### Requirement: Setting tags from the CLI
`ship pane set` and `ship tab set` SHALL accept a repeatable `--tag NAME=VALUE`, which sets tag `NAME` to `VALUE` without colors, and a bare `--tag NAME`, which removes it. A value SHALL run from after the first `=` to the end, so it may contain `=`. A repeated name SHALL take its last occurrence. An empty value SHALL set a tag with an empty value.

#### Scenario: Set two tags
- **WHEN** a script runs `ship pane set --pane <p> --tag git.branch=main --tag git.ahead=2`
- **THEN** `ship pane get` shows both tags

#### Scenario: Remove one tag
- **WHEN** that pane then gets `ship pane set --pane <p> --tag git.ahead`
- **THEN** only `git.ahead` is removed

#### Scenario: Empty value
- **WHEN** a script runs `ship pane set --pane <p> --tag git.ahead=`
- **THEN** the pane has `git.ahead` with value `""`

#### Scenario: Repeated name
- **WHEN** a script runs `ship tab set --tab <t> --tag k=1 --tag k=2`
- **THEN** the tab has `k` with value `2`

### Requirement: Clearing all tags from the CLI
`ship pane set` and `ship tab set` SHALL accept `--clear-tags`, which removes every tag on the pane or tab.

#### Scenario: Clear tags
- **WHEN** a pane has three tags and a script runs `ship pane set --pane <p> --clear-tags`
- **THEN** `ship pane get` shows no `tags` field

### Requirement: Tags are CLI and HTTP only
Tags SHALL NOT be settable from `server.pane.set` or `server.tab.set` in the config file, its menus or its keys. A config that gives those actions tags SHALL fail loading as an unknown field.

#### Scenario: Tags in a key binding
- **WHEN** a config binds a key to `server.pane.set = { tags = { k = { value = "x" } } }` and the user runs `ship config check`
- **THEN** it reports the unknown field with its line and column and exits 1

### Requirement: Setting tags over HTTP
`PATCH /api/v0/tabs/{id}` and `PATCH /api/v0/panes/{id}` SHALL accept `tags` as a JSON Merge Patch (RFC 7396): a name left out SHALL stay as it is, a tag object SHALL set that name, `null` SHALL remove it, and `"tags": null` SHALL remove every tag.

#### Scenario: Patch one tag among several
- **WHEN** a pane has `a` and `b`, and a caller patches it with `{"tags": {"b": null, "c": {"value": "3"}}}`
- **THEN** the pane has `a` unchanged and `c`, and no `b`

#### Scenario: Remove all over HTTP
- **WHEN** a caller patches a tab with `{"tags": null}`
- **THEN** the tab has no tags

### Requirement: Writing a tag replaces it
Setting a tag SHALL replace its value and colors whole. The last write SHALL win, whichever client or script made it. No tag SHALL have an owner.

#### Scenario: Colors cleared by a plain write
- **WHEN** tag `gh.checks` has `fg` `red` and a script runs `ship pane set --pane <p> --tag gh.checks=ok`
- **THEN** the tag's value is `ok` and it has no colors

#### Scenario: Two writers
- **WHEN** one script sets `git.branch=main` and another then sets `git.branch=dev` on the same pane
- **THEN** the pane shows `dev`

### Requirement: Tag changes publish once
Tag changes and other attribute changes in one request SHALL take effect in one edit, published as one state. A request whose tags and other attributes match what is stored SHALL publish nothing and leave the revision unchanged.

#### Scenario: Name and tag together
- **WHEN** a script runs `ship pane set --pane <p> --name api --tag git.branch=main`
- **THEN** the revision advances by one, and that state shows both

#### Scenario: Same tag again
- **WHEN** a pane has `git.branch` with value `main` and a script sets `--tag git.branch=main` again
- **THEN** the revision is unchanged

### Requirement: Tag lifetime
A tag SHALL last until it is overwritten or removed, or its pane or tab closes. Tags SHALL NOT survive a server restart.

#### Scenario: Pane closes
- **WHEN** a tagged pane closes
- **THEN** its tags are gone with it, and no other pane or tab gains them

#### Scenario: Server restart
- **WHEN** a pane and its tab have tags and the server restarts with `ship server stop && ship`
- **THEN** the restored pane and tab have no tags

### Requirement: Tags don't drive attention
Tags SHALL NOT affect display states, unseen marks, tab rollups, the agent list, notifications, popups or sounds.

#### Scenario: A tag that reads like a state
- **WHEN** a script sets `--tag status=blocked` on a pane with no status records
- **THEN** the pane gets no display state, no agent entry and no notification
