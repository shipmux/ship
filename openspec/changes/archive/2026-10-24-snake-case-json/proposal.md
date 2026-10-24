# Proposal

Status: Locked. Cyan asked for this change and approved it for implementation in conversation on 2026-10-24. It is small enough to need no design papers; this proposal and the deltas under `specs/` are the contract.

## Why

A setting is spelled one way in the config file and another way in JSON: `right_click_passthrough` in `config.toml` and in key-binding actions, but `rightClickPassthrough` in `ship pane get` and the HTTP API. A script that reads a value from JSON and writes it into the config has to know both spellings. JSON enum values and error kinds are already snake case, so a single object mixes the two styles (`{"exitBehavior": "keep_failed"}`). Ship has no releases yet, so changing this now costs one protocol bump. After a release, every script reading the old keys would break.

## What Changes

- **BREAKING: snake case JSON.** Every JSON object key and string value Ship reads or writes uses snake case. That covers HTTP request and response bodies, attach stream messages, CLI stdout, error lines and the saved tabs file. The names that change are:
  - keys: `protocolVersion`, `causedBy`, `exitBehavior`, `rightClickPassthrough`, `scrollbackBytes` and `underlineColor`;
  - values: the end reason `serverShutdown` and the text attributes `slowBlink`, `rapidBlink` and `crossedOut`.
- Single-word names, such as `id`, `name`, `status`, `program` and `kind`, are unchanged.
- **CLI flags stay kebab case** (`--exit-behavior`), as clap and most CLIs write them. A name shared by config, key-binding actions and JSON is spelled the same in all three.
- **BREAKING: protocol 16.** The health check refuses clients built for an earlier protocol, and a new client refuses an older server.
- **Saved tabs from protocol 15 or earlier** restore their structure, names, proportions and zoom. Each pane's right-click passthrough, exit behavior and history size fall back to the defaults. No alias reads the old keys.
- The health-exchange spec still says protocol 14, although protocol 15 shipped with pane defaults. This change corrects it to 16.

Not in this change:
- Renaming CLI flags or config keys. They are already snake or kebab case.
- Aliases that accept the old camel case keys anywhere.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `session-structure`: a new JSON names requirement, and the pane attribute requirements and HTTP set fields with snake case keys.
- `health-exchange`: health and error bodies use snake case, and the protocol version is 16.
- `session-restore`: restored pane settings use snake case keys.
- `session-observation`: style runs use `underline_color`.
- `pane-history`: the create scenario reads `scrollback_bytes`.
- `pane-automation`: send errors put the cause under `caused_by`.
- `config-file`: pane defaults scenarios read `exit_behavior`.

## Impact

- `ship-core`: the `rename_all = "camelCase"` attributes in `protocol.rs`, `model.rs`, `error.rs`, `screen.rs`, `layout.rs` and `geometry.rs` become `snake_case`, and `PROTOCOL_VERSION` goes to 16. The OpenAPI schema follows from the types.
- `ship-server`: one doc comment names `serverShutdown`.
- Docs: `docs/site/scripting/reading-command-errors.md` (`causedBy`) and the pull request template's `protocolVersion`.
- No dependency changes. Implementation Rust changes by a few lines; tests by none planned.
