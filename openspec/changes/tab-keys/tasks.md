# Tasks

The three groups are `design/program.md`'s build slices, in one pull request. The paper's skeleton map gives the types and signatures; record anything that differs in its deviation log.

Verification uses real commands and a running client, not permanent authored tests. Use an owned temporary server on a port from 43331 to 43337, with `XDG_STATE_HOME` and `XDG_CONFIG_HOME` pointed at temporary directories, and don't touch the server on 43179. Every code group ends with `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` and `openspec validate tab-keys --type change --strict`. Checks run on macOS; Linux is unverified unless run there.

## 1. `h` and `l` hop between parent and children

- [ ] 1.1 Add `Tabs::folded` in `crates/ship-client/src/ui/sidebar/tabs.rs`, and `Hop`, `collapse` and `expand` in `crates/ship-client/src/ui/mod.rs`. Replace the shared `Expand`/`Collapse` arm in `act` with one arm per key that folds through `sidebar.tabs.fold` or selects through `select` and `Memory::landing`.
- [x] 1.2 Write `specs/session-observation/spec.md`: MODIFIED "Local folding and selected-row visibility" so only folding keeps the selection, and ADDED "Parent and child hops" with the P3 cases.
- [ ] 1.3 Verify in a running client on the product paper's example tree: every P3 case, and folding and unfolding an open or folded row as before (SC-003).

## 2. `client.tab.move` and its bindings

- [ ] 2.1 Add `ClientTab::Move { direction }` and `TabDirection` in `crates/ship-client/src/keymap.rs`, `shift` and the `Move` arm returning `Turn::First(vec![Item::Move(..)])` in `crates/ship-client/src/ui/mod.rs`, and the four labeled bindings in `crates/ship-core/src/defaults.toml`.
- [x] 2.2 Write `specs/keymap/spec.md`: MODIFIED "Client actions" to list `client.tab.move`, and ADDED "Relative tab moves", "Tab moves at their turn" and "Default tabs keys" with the P1 and P2 scenarios.
- [ ] 2.3 Verify with `ship tab list`: every P1 case gives the stated tree with the selection kept (FR-010); every P2 case leaves the output byte-for-byte unchanged with no status message (SC-002); three quick `shift-k` presses move three places (FR-008); the status line shows the four labels in tabs mode.
- [ ] 2.4 Walk SC-001 once with keys only. For one `in` and one `out`, compare against the same move made by mouse drop (SC-004).

## 3. Docs

- [ ] 3.1 Following `docs/agents/docs.md`, update the `alt-g` row in `docs/site/tabs.md` and the tabs mode sentence in `docs/site/keys.md` with the move keys and the `h`/`l` fallbacks.
