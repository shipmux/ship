# Tasks

The six groups are `design/program.md`'s build slices, shipped in one pull request. The paper's skeleton map gives the types and signatures; record anything that differs in its deviation log. The delta specs under `specs/` were written ahead of the slices, so each slice checks that its deltas still match what it built and doesn't write them.

The paper puts all docs in slice 6. Here each slice updates the docs its own behavior touches: `GLOSSARY.md` in slice 1, the tags section of `docs/site/scripting/writing-a-plugin.md` in slice 2, its `watch` section in slice 3, `configuration.md` in slices 4 and 5, and the example in slice 6. The files and their content are the paper's.

Three product unknowns are still open, and each one gates part of a slice: OU-1 (tag colors on the CLI) in slice 2, OU-2 (control characters in values) in slice 1, and OU-3 (how `watch` ends) in slice 3. A gate task brings the unknown to Cyan with the paper's recommendation. Until Cyan decides, the gated code isn't written; once Cyan decides, the decision goes into `design/product.md` as an amendment and into the matching delta before the gated task starts.

Verification uses real commands and a running client, not permanent authored tests. Use an owned temporary server on its own socket, with `XDG_STATE_HOME`, `XDG_CONFIG_HOME` and `XDG_RUNTIME_DIR` pointed at temporary directories, and don't use any server that's already running. Every code group ends with `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` and `openspec validate labels-and-watch --type change --strict`. Checks run on macOS; Linux is unverified unless it runs there.

## 1. Tags in core and the server

- [x] 1.1 Enable serde_with's `indexmap_2` feature in the workspace `Cargo.toml`. Verify with `cargo tree -d` that it adds no duplicate crates.
- [x] 1.2 Add `crates/ship-core/src/tag.rs` with `TagName`, `Tag`, `NamedColor` (with `From<NamedColor>` for ratatui's `Color`), `Tags` and `TagPatch`, without `parse_edit`, and declare it in `lib.rs`. Verify that `cargo build` passes.
- [x] 1.3 Add `tags` to `Pane` and `Tab` in `model.rs` and to `SetPane` and `SetTab` in `protocol.rs`, and bump `PROTOCOL_VERSION` to 22. Verify that `ship pane get` on a fresh pane has no `tags` field and `/health` reports protocol 22.
- [x] 1.4 In `crates/ship-server/src/state.rs`, apply the patch in both `Set` handlers before the change comparison, and clear tab tags in `restore_tab`; fill `tags` in the `Pane` and `Tab` literals, including `state/geometry.rs`. Verify with `curl --unix-socket` that a `PATCH` with `{"tags":{"git.branch":{"value":"main","fg":"green"}}}` shows in `ship pane get`, that `{"tags":{"git.branch":null}}` removes one and `{"tags":null}` removes all, and that the same works on tabs with `ship tab list`.
- [x] 1.5 Verify the rejections: a name with `=`, an empty name, a name with a control character, `"fg":"pink"`, `"fg":"#ff0000"` and `"fg":null` each get 400 `validation` with a pointer, and nothing changes.
- [x] 1.6 Verify the revision rules by reading `x-ship-revision`: the same body sent twice leaves the revision unchanged the second time, and `name` and `tags` in one body move it by one.
- [x] 1.7 In `crates/ship-server/src/saver.rs`, clear tags in `blank()`. Verify that after `ship server stop && ship` the tags are gone, that the saved file has no `tags` field, and that a saved file written before this change still restores.
- [x] 1.8 Gate OU-2: bring Cyan the value rule with the paper's recommendation (a `TagValue` newtype that refuses control characters, and bidirectional characters stripped with `records.rs`'s `disarm`). Record the decision as a product amendment and in the `tags` delta.
- [x] 1.9 After 1.8, build the decided value rule. Verify a value with a control character and a value with a bidirectional character against it over HTTP.
- [x] 1.10 Following `docs/agents/domain.md`, add **Tag** to `GLOSSARY.md`, beside the existing meaning of a derived label.
- [x] 1.11 Check that the `tags` delta's HTTP, name, color, merge, publish and lifetime requirements, the `session-structure` delta's HTTP set fields and the `session-restore` delta match what was built.

## 2. `--tag` and `--clear-tags`

- [x] 2.1 Add `parse_edit` to `tag.rs`, the `tags` and `clear_tags` fields to the pane and tab set commands in `command.rs`, and the patch building in `ship-client/src/execute.rs` and `ui/nav.rs`. Verify that `ship pane set --help` shows both flags.
- [x] 2.2 Verify each CLI scenario with `ship pane set` and `ship tab set`, reading `get` after each: `--tag git.branch=main --tag git.ahead=2` sets both, a bare `--tag git.ahead` removes one, `--tag git.ahead=` keeps an empty value, `--tag a=b=c` stores `b=c`, a repeated name takes its last value, and `--clear-tags` removes all.
- [x] 2.3 Verify that `--clear-tags` with `--tag` in the same call is refused by clap (C6), and that a config with `server.pane.set = { tags = … }` fails `ship config check` as an unknown field with its line and column.
- [x] 2.4 Gate OU-1: bring Cyan the CLI color flags with the paper's recommendation (`--tag-fg NAME=COLOR` and `--tag-bg NAME=COLOR`). Record the decision as a product amendment and in the `tags` delta.
- [x] 2.5 After 2.4, build the decided color flags. Verify that a color set from the CLI shows in `ship pane get`, and that a bad color name is a usage error.
- [x] 2.6 Following `docs/agents/docs.md`, start `docs/site/scripting/writing-a-plugin.md` with its tags section and link it from `docs/site/scripting/README.md`. Verify that each command on the page runs as written.
- [x] 2.7 Check that the `tags` delta's CLI requirements and the `session-structure` delta's Set attribute fields match what was built.

## 3. `ship watch`

- [x] 3.1 Derive clap's `ValueEnum` on `FeedMode` in `protocol.rs`, and make `observe` in `ship-client/src/automation.rs` public with a `feed` argument. Verify that `ship pane wait` still works.
- [x] 3.2 Add `Command::Watch` to `crates/ship/src/cli.rs`, its route through `checked_target` in `main.rs`, and `watch` and `line` in `commands.rs`. Verify that `ship watch | jq -c .` prints the health line and then a replica per change, and that creating and renaming a tab from another terminal shows each change arrive.
- [x] 3.3 Verify that no line holds screen contents or clipboard text, that `ship watch | head -1` exits 0, and that `ship watch` with no server running fails without starting one (C1).
- [x] 3.4 Verify that `--feed every` prints each revision in order: a burst of ten renames shows ten consecutive revisions.
- [x] 3.5 Gate OU-3: bring Cyan the end of `watch`'s output with the paper's recommendation (a last `{"reason":…}` line, then an `Unavailable` error). Record the decision as a product amendment and in the `state-watching` delta.
- [x] 3.6 After 3.5, build the decided ending. Verify that `ship server stop` ends `watch` that way with a non-zero exit, and that a cut stream without `Ended` is a `Network` error.
- [x] 3.7 For SC-003, run `benchmarks/bench.py --mux ship --workload type_idle type_agent` with five `ship watch >/dev/null` processes attached to its server and with none, five reps each, and compare echo latency with `--summarize`. Record both results. If the harness can't reach its server's name from outside, add a flag for it to `bench.py`.
- [x] 3.8 Following `docs/agents/docs.md`, add the `watch` section to `writing-a-plugin.md`. Verify that each command on the page runs as written.
- [x] 3.9 Check that the `state-watching` delta matches what was built.

## 4. Tags in agent rows

- [x] 4.1 Add `crates/ship-client/src/ui/sidebar/tags.rs` with `on_tab` and `span`, and declare it in `sidebar/mod.rs`. Verify that `cargo build` passes.
- [x] 4.2 Add `Token::Tag` in `keymap.rs`, draw it in `sidebar/agents.rs`, and update the agents comment in `defaults.toml`. Verify in a running client that with `rows = [["icon", "app", "state", { tag = "git.branch" }], ["tab"]]` an agent whose pane has `git.branch` shows it after its state, and a pane without it shows the line as before.
- [x] 4.3 Verify that a tag with `fg = "white"` and `bg = "red"` keeps those colors on the selected agent's band, that the `tab` token still shortens on a narrow sidebar, and that `rows = [["nope"]]` fails `ship config check`.
- [x] 4.4 Following `docs/agents/docs.md`, document the tag item in `[client.agents] rows` in `docs/site/configuration.md`. Verify that it matches `defaults.toml`.
- [x] 4.5 Check that the `agent-attention` delta, the `config-file` delta's Agent settings and the `terminal-client` delta's tag colors requirement match what was built.

## 5. Multi-line tab entries

- [x] 5.1 Add `TabSettings`, `TabToken` and `TabsFile` in `keymap.rs`, and `[client.tabs]` in `defaults.toml`. Verify that `ship config check` accepts the defaults, reports `rows = [["title"]]` with its line and column, and that `ship config defaults` prints the table with its comment.
- [x] 5.2 Pass `TabSettings` through `Sidebar::new` and `configure`, and split `sidebar/tabs.rs` into `Entry` from `walk()` and `Row` per line with `Part`. Verify that with no `[client.tabs]` the sidebar matches a screenshot taken before the change (SC-004).
- [x] 5.3 Verify in a running client that with `rows = [["name"], [{ tag = "git.branch" }]]` a tab shows its branch under its name, else its first pane's (FR-015), while `ship tab get` shows only the tab's own, and that a tab with neither keeps one line. Verify that saving a change to the rows updates the sidebar without a restart.
- [x] 5.4 On a tree three levels deep with some one-line and some two-line entries, verify that clicking either line selects its tab, that only a head's marker folds, that a drop onto a second line lands above that entry, that wheel scrolling stops at the last line, that stepping through tabs keeps the whole selected entry in view, and that a `bg` tag stays visible on the selected entry.
- [x] 5.5 Following `docs/agents/docs.md`, document `[client.tabs] rows` in `docs/site/configuration.md`. Verify that it matches `defaults.toml`.
- [x] 5.6 Check that the `terminal-client` delta's tab entry requirements and drop line and the `config-file` delta's Tab settings match what was built.

## 6. The git example

- [ ] 6.1 Add `examples/plugins/git-branch.sh` and `examples/plugins/README.md`. Verify with `shellcheck` and `sh -n`.
- [ ] 6.2 Verify with the example running that a pane in a checkout carries `git.branch`, that `cd` into another repo or `git switch` updates it when the prompt returns, and that `cd /tmp` removes it.
- [ ] 6.3 Run the example through a day of normal use on macOS to settle SC-002. Report Linux as unverified unless it runs there.
- [ ] 6.4 Following `docs/agents/docs.md`, add the example to `writing-a-plugin.md` and the README. Verify that the site build strips the new page's scope comment without complaint.

## Workflow follow-up

- Open the pull request from `.github/pull_request_template.md`, with a tracking issue for any decision or workaround it leaves behind.
- After merge, sync the deltas and archive the change.
