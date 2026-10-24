# Tasks

Verification uses real commands and temporary probes, not permanent authored tests. Use an owned temporary server on a port from 43331 to 43337, with `XDG_STATE_HOME` and `XDG_CONFIG_HOME` pointed at temporary directories, and don't touch the server on 43179. Checks run on macOS; Linux is unverified unless run there.

## 1. Snake case on the wire

- [x] 1.1 In `crates/ship-core/src/{protocol,model,error,screen,layout,geometry}.rs`, change every `rename_all = "camelCase"` to `"snake_case"`, keeping `deny_unknown_fields` where present. Set `PROTOCOL_VERSION` to 16.
- [x] 1.2 Update the `serverShutdown` doc comment in `crates/ship-server/src/lib.rs`, `causedBy` in `docs/site/scripting/reading-command-errors.md`, and `protocolVersion` in `.github/pull_request_template.md`.
- [x] 1.3 Verify against the temporary server:
  - `ship server status` prints `protocol_version: 16`.
  - `ship pane create --exit-behavior keep --scrollback-bytes 50000000 --right-click-passthrough`, then `ship pane get`, shows `exit_behavior`, `scrollback_bytes` and `right_click_passthrough`.
  - `PATCH /api/v0/panes/{id}` with `{"exit_behavior": "keep_failed"}` changes the pane, and `{"exitBehavior": "keep"}` is rejected with nothing changed.
  - `ship pane send --wait` on a blocked pane prints an error line with `caused_by`.
  - A pane printing a colored underline and struck-through text publishes `underline_color` and `crossed_out`, and stopping the server ends an attach stream with `server_shutdown`.
  - `grep -rnE 'protocolVersion|causedBy|exitBehavior|rightClickPassthrough|scrollbackBytes|underlineColor|serverShutdown|slowBlink|rapidBlink|crossedOut'` over the repo, excluding `target/`, `.git/` and `openspec/changes/archive/`, matches only this change's own files and the main specs its deltas replace at archive.
  - `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and `openspec validate snake-case-json --type change --strict`.
