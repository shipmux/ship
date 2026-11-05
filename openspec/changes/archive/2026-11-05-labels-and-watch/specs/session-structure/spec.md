# Spec Delta

## MODIFIED Requirements

### Requirement: Set attribute fields
A tab's attributes SHALL be `name` (`--name`), `zoomed` (`--zoomed`) and `tags` (`--tag`, `--tag-fg`, `--tag-bg`, `--clear-tags`). A pane's attributes SHALL be `name` (`--name`), `right_click_passthrough` (`--right-click-passthrough`), `exit_behavior` (`--exit-behavior`), `scrollback_bytes` (`--scrollback-bytes`) and `tags` (`--tag`, `--tag-fg`, `--tag-bg`, `--clear-tags`), as the `tags` capability describes.

#### Scenario: Exit behavior and history in one edit
- **WHEN** a script runs `ship pane set --pane <id> --exit-behavior keep_failed --scrollback-bytes 50000000`
- **THEN** both change in one edit, and `ship pane get` shows `exit_behavior: keep_failed` and `scrollback_bytes: 50000000`

#### Scenario: Name and zoom in one edit
- **WHEN** a script runs `ship tab set --tab <t> --name build --zoomed <p>` for a pane `<p>` in `<t>`
- **THEN** both change in one edit, and `ship tab get` shows them

#### Scenario: Name and tag in one edit
- **WHEN** a script runs `ship tab set --tab <t> --name build --tag ci.state=green`
- **THEN** both change in one edit, and `ship tab get` shows them

### Requirement: HTTP set fields
Over HTTP, `PATCH /api/v0/tabs/{id}` SHALL take `{ "name"?, "zoomed"?, "tags"? }` and `PATCH /api/v0/panes/{id}` SHALL take `{ "name"?, "right_click_passthrough"?, "exit_behavior"?, "scrollback_bytes"?, "tags"? }`, as JSON Merge Patch (RFC 7396). An absent field SHALL leave its attribute unchanged, and JSON `null` SHALL clear it. `null` SHALL be accepted only on `name`, `zoomed` and `tags`; on any other field it SHALL get 400 `validation`. `tags` follows the `tags` capability's merge rules.

#### Scenario: Omitted and blank HTTP fields
- **WHEN** a caller patches a pane with `{}`, then `{"name": "  x "}`, then `{"name": ""}`, then `{"name": null}`
- **THEN** the first changes nothing, the next two store `"  x "` and `""` exactly, and the last clears the name

#### Scenario: Null on a field that can't be empty
- **WHEN** a caller patches a pane with `{"right_click_passthrough": null}`
- **THEN** it receives 400 `validation` pointing at `/right_click_passthrough` and the pane is unchanged

#### Scenario: Toggle over HTTP
- **WHEN** a caller patches a pane with `{"right_click_passthrough": "toggle"}`
- **THEN** the setting flips

#### Scenario: Null tags
- **WHEN** a caller patches a pane that has tags with `{"tags": null}`
- **THEN** the pane has no tags
