# Tasks

The four groups are `design/program.md`'s build slices, each one commit in one pull request. The paper's skeleton map gives the types and signatures; record anything that differs in its deviation log.

Verification uses real commands and temporary probes, not permanent authored tests. Use an owned temporary server on a port from 43331 to 43337, with `XDG_STATE_HOME` and `XDG_CONFIG_HOME` pointed at temporary directories, and don't touch the server on 43179. Every group ends with `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace`. Checks run on macOS; Linux is unverified unless run there.

## 1. Shapes

- [x] 1.1 In `crates/ship-core/src/protocol.rs`, add `NewPane`, `NewTab`, `MoveTabBody` and `present`; nest `CreatePane` and `CreateTab`; make `SetTab` and `SetPane` names `double_option`; remove `PaneInput`, `PaneSpec` and `set_name`; put `deny_unknown_fields` and `skip_serializing_none` on every request type and `present` per the paper's table; correct the `cwd` default doc comments; set `PROTOCOL_VERSION` to 17. Verify with `cargo check --workspace`.
- [x] 1.2 In `crates/ship-core/src/model.rs`, remove `OptionalName` and make `Tab` and `Pane` `name` a plain `Option<String>`. In `command.rs`, make `--name` on `tab set` and `pane set` an `Option<Option<String>>`. Verify `ship tab set --help` shows `--name [<NAME>]`.
- [x] 1.3 Wire the new types through `ship-server` (`state.rs`, `routes.rs` with `MoveTabBody` replacing the hand decode, `lib.rs`) and `ship-client` (`execute.rs`, `api.rs`, `ui/draw.rs`). Verify `cargo build --workspace` and a dump of the API description showing `NewPane` and `NewTab` under `pane` and `tab`, with the fields `skip_serializing_none` rewrote still optional.
- [x] 1.4 Verify against the temporary server: `POST /panes {"at": {"tab": T}, "pane": {"cwd": "/tmp", "exit_behavior": "keep"}}` creates that pane; `POST /panes {"at": {"tab": T}, "cwd": "/tmp"}` is refused and creates nothing; `POST /tabs {"tab": {"name": "x"}}` and `POST /tabs/{id}/move {"at": {"after": T}}` work.
- [x] 1.5 Verify PATCH and the CLI: `{"name": null}` clears, `{"name": "  x "}` stores `"  x "`, `{"right_click_passthrough": null}` is refused, `{"right_click_passthrough": "toggle"}` flips; `ship pane set --name` clears, `--name ""` stores `""`, and the same for `ship tab set`; an empty name draws as an empty sidebar label; `ship server status` prints `protocol_version: 17`.
- [x] 1.6 Update `docs/site/tabs.md` (and any page showing `--name ""` as clearing) so a bare `--name` clears. Verify the documented commands run as written.
- [x] 1.7 Run the group's fmt, clippy and test commands, then commit.

## 2. The error path

Cyan's approved slice 2 revision supersedes the unknown-field rejection recorded in 1.1 and 1.4: API requests, input lines and queries ignore extras. Config remains strict. The current checks below replace those original acceptance criteria. Cyan subsequently approved replacing jsonschema with boon; the schema-library decisions in `design/architecture.md` and `design/program.md` supersede the original library-specific plan. Slice 2 must remain uncommitted until Cyan asks.

- [x] 2.1 Add `boon = "0.6.1"` to `[workspace.dependencies]`, and `boon.workspace = true` plus `serde_path_to_error.workspace = true` to `crates/ship-server/Cargo.toml`. Record release binary size and `cargo build --release` time before and after. Verify `cargo build --workspace`.
- [x] 2.2 Add `Problem` to `crates/ship-core/src/error.rs` and the new `crates/ship-server/src/body.rs` with `Body<T>`, `Query<T>`, `decode`, the server-local `ValidatorMap` (boon's `Schemas` plus component-name `SchemaIndex` entries) built once from the document's components, `explain` and `flatten` over boon's error tree. Verify `cargo check --workspace`.
- [x] 2.3 Move every JSON body and query in `routes.rs`, `attach.rs` and `views.rs` onto `Body<T>` and `Query<T>`, make `input.rs` decode lines through `body::decode`, add `mod body;`, and list `400 validation` on each route in place of the 422 malformed-body lines. Verify the API description lists 400 for every route with a body or query.
- [x] 2.4 Verify against the temporary server: the four-mistake `POST /panes` body (missing `at`, bad direction, exit behavior and scrollback type, plus an ignored `exitBehavior`) gets one 400 listing all four with pointers; `GET /panes/{id}/text?lines=nope` gets 400 naming `lines`; non-JSON gets one entry with serde's syntax message; a body serde refuses but the schema accepts gets one entry with `serde_path_to_error`'s pointer; a body without `Content-Type: application/json` gets 400, not 415.
- [x] 2.5 Verify body/query decoding still works with an extra field or parameter and extras do not become stored attributes; a PATCH containing only extras leaves the revision unchanged; every field that can't be cleared, sent `null` in each body that has it, answers 400 with its pointer and changes nothing; every valid nested request from 1.4 and 1.5 still succeeds. Verify the generated frame schema accepts a valid paste and ignores extras but rejects wrong types, missing required fields and unknown tags.
- [x] 2.6 Verify on `POST /attach/input` that a line with a bad value or malformed JSON ends the stream with 400 and its `data.errors` after earlier lines were delivered; a valid line with an extra field is delivered, and mouse input without an attachment still answers 422.
- [x] 2.7 Run the group's fmt, clippy and test commands. Leave changes uncommitted until Cyan asks (supersedes the planned commit).

Verification repeated with the real boon implementation for 2.4 to 2.6 on macOS: all 69 generated component schemas compile with boon; all 15 documented body/query operations have JSON 400 responses; 32 non-clearable null fields in HTTP bodies and 12 across the input-frame variants return 400 with field pointers and unchanged state/revision. Real NDJSON delivery checks passed for ignored extras, prior-line delivery, bad values and syntax, and unattached mouse 422. The schema-looser u16 range probe falls back to serde's `/amount` pointer, and a schema-stricter uppercase UUID is accepted and delivered, confirming the schema never gates valid input. Config still rejects unknown keys. Tagged enums retain serde's structural constraints; `LogChange` still takes exactly one tag in its object form.

Release measurements for 2.1 on macOS:

| Build | Executable bytes | Wall time | Cache conditions |
| --- | ---: | ---: | --- |
| Original slice 1 baseline | 13,399,232 | 114.706 s | Existing target; populated release cache |
| Superseded initial jsonschema implementation | 16,758,992 | 100.035 s | Reused release dependencies |
| Superseded jsonschema after unknown-field revision | 16,742,480 | 98.005 s | Reused release dependencies |
| Main in `../ship` | 13,415,840 | 117.261 s | Empty target; earlier clean comparison |
| jsonschema comparison build | 16,742,480 | 141.697 s | Empty target; spike comparison |
| boon spike build | 14,672,960 | 124.085 s | Separate empty target; built after jsonschema comparison |
| Final real-checkout boon implementation | 14,672,944 | 84.777 s | Reused release dependencies; `cargo build --release --locked` |

The controlled spike builds used `cargo build --release --locked`, identical release profiles and separate empty target directories. These are single sequential timing observations, not statistically controlled benchmarks. The final real implementation saves 2,069,536 bytes versus jsonschema and is 1,257,104 bytes (9.37%) above the clean main binary. Cached build times must not be compared as clean-build costs.

The final lockfile has 497 package entries versus 490 on slice 1/main and 515 with jsonschema: 7 additions rather than 25. It contains boon 0.6.1 and no jsonschema package.

Final real-checkout `cargo check --workspace`, `cargo build --workspace`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `openspec validate entity-shapes --type change --strict` and `git diff --check` passed. The real API/CLI/NDJSON probes passed with boon, including schema-looser fallback and schema-stricter serde acceptance. Temporary server stopped and port 43331 verified closed. Linux remains unverified. No permanent tests or frame-specific handling added; no commit created.

## 3. Zoom

- [x] 3.1 Add `zoomed` to `SetTab` and apply it in `state.rs` in the same commit as `name`, refusing an unknown pane with `not_found` and another tab's pane with `invalid_structure`; remove `ZoomPane`, the `Zoom` message, `routes::zoom_pane` and its `api_routes!` entry. Add `fn nullable_pane_schema` only if the dumped description shows `zoomed` as non-nullable. Verify the description shows `zoomed` as nullable and no `/zoom` route.
- [x] 3.2 Add `--zoomed` to `ship tab set` and remove `ship pane zoom` and `PaneCommand::Zoom` from `command.rs`, `execute.rs` and `api.rs`. Verify `ship tab set --tab T --zoomed P` zooms, a bare `--zoomed` unzooms, a pane in another tab gets 422, an unknown pane gets 404, `PATCH /tabs/{id} {"zoomed": null}` unzooms, and `ship pane zoom` is an unknown subcommand.
- [x] 3.3 Add `ClientPane::Zoom { zoomed: Switch }` in `keymap.rs`, `zoom` and `leaves_zoom` in `ui/mod.rs` with `act()`'s unzoom-first rule matching `leaves_zoom`, and set `alt-f` and the pane menu's `zoom` to `client.pane.zoom = { zoomed = "toggle" }` in `defaults.toml`. Verify in a real client that `alt-f` and the menu item zoom and unzoom, and that moving focus out of a zoomed pane unzooms first.
- [x] 3.4 Verify with two attached clients that both pressing `alt-f` on an unzoomed tab leaves it zoomed; that temporary `zoomed = true` and `zoomed = false` bindings only zoom and only unzoom; and that `client.pane.zoom = {}` and `server.pane.zoom = {}` each fail `ship config check` with a line and column.
- [x] 3.5 Update `docs/site/panes.md` and `docs/site/configuration.md` to drop `ship pane zoom` and `server.pane.zoom` and show `ship tab set --zoomed` and `client.pane.zoom`. Verify the documented commands and config run as written.
- [x] 3.6 Run the group's fmt, clippy and test commands, then commit.

Verified on macOS against a temporary server on 43331. The derived description shows `zoomed` as `oneOf [Id_Pane, null]` without a schema function, and no `/zoom` path. PATCH and `ship tab set` zoom, unzoom, answer 422 `invalid_structure` for another tab's pane, 404 for an unknown one, and 400 with a `/zoomed` pointer for a wrong type; `name` and `zoomed` apply in one commit. `POST /panes/{id}/zoom` answers 404 and `ship pane zoom` is an unknown subcommand. Two real clients in a private tmux server: `alt-f` and the pane menu's `zoom` zoom and unzoom; focus out of a zoomed pane unzooms first; `zoomed = true` and `zoomed = false` bindings pressed twice only zoom and only unzoom; both clients pressing `alt-f` with no wait left the tab zoomed in 5 of 5 rounds. `client.pane.zoom = {}` fails `ship config check` with "missing field `zoomed`" at line 2, column 43, and `server.pane.zoom = {}` with "unknown variant `zoom`". The `panes.md` block and the `configuration.md` menu example run as written. fmt, clippy and tests pass. Linux is unverified.

## 4. Rules, specs and the one-off checks

- [ ] 4.1 Add the "Entity and request shapes" subsection to `docs/agents/code.md`: FR-001 to FR-008, FR-013, FR-014's rule, and "an input field takes `null` only if the entity field can be `null`", with `present` and `double_option` as the two ways to say it. Verify by reading it against the product paper's list.
- [ ] 4.2 Write a scratch binary outside the repo that loads `ship_server::openapi()` and prints every input field named like an entity field whose type differs, allowing only `toggle` on PATCH bools and `shell`, and every input field whose nullability differs from its entity field's. Verify it prints nothing, then delete it.
- [ ] 4.3 Check every `server.` action in `defaults.toml` by hand against FR-013 and list the result for the pull request.
- [ ] 4.4 Re-read the spec deltas under `specs/` against the built behavior from groups 1 to 3 and correct any that differ. Verify `grep -rnE 'server\.pane\.zoom|ship pane zoom|/zoom|ZoomPane|OptionalName|PaneInput|PaneSpec'` over the repo, excluding `target/`, `.git/` and `openspec/changes/archive/`, matches only this change's own files and the main specs its deltas replace.
- [ ] 4.5 Run `openspec validate entity-shapes --type change --strict` and the group's fmt, clippy and test commands, then commit.

## Workflow follow-up

- Open the pull request from `.github/pull_request_template.md`, reporting implementation Rust and test line counts separately, the binary size and build time from 2.1, the FR-013 list from 4.3, and Linux as unverified. File a tracking issue for any open decision it leaves.
- Archive the change after Cyan approves the pull request, then verify the synced main specs.
