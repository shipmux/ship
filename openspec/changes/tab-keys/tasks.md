# Tasks

The three groups are `design/program.md`'s build slices, in one pull request. The paper's skeleton map gives the types and signatures; record anything that differs in its deviation log.

Verification uses real commands and a running client, not permanent authored tests. Use an owned temporary server on a port from 43331 to 43337, with `XDG_STATE_HOME` and `XDG_CONFIG_HOME` pointed at temporary directories, and don't touch the server on 43179. Every code group ends with `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` and `openspec validate tab-keys --type change --strict`. Checks run on macOS; Linux is unverified unless run there.

## 1. `h` and `l` hop between parent and children

- [x] 1.1 Add `Tabs::folded` in `crates/ship-client/src/ui/sidebar/tabs.rs`, and `Hop`, `collapse` and `expand` in `crates/ship-client/src/ui/mod.rs`. Replace the shared `Expand`/`Collapse` arm in `act` with one arm per key that folds through `sidebar.tabs.fold` or selects through `select` and `Memory::landing`.
- [x] 1.2 Write `specs/session-observation/spec.md`: MODIFIED "Local folding and selected-row visibility" so only folding keeps the selection, and ADDED "Parent and child hops" with the P3 cases.
- [x] 1.3 Verify in a running client on the product paper's example tree: every P3 case, and folding and unfolding an open or folded row as before (SC-003).

Slice 1 verification: on macOS, build, formatting, workspace Clippy with warnings denied, workspace tests (80 passed, 1 ignored) and strict OpenSpec validation passed. An owned temporary server on port 43331 and two clients in an isolated tmux server passed 27 live checks, including every P3 case, unchanged folding/unfolding, selected-pane hops, default/remembered/zoomed landing against `j`, and independent client folds. The temporary server and clients were stopped afterward. Linux is unverified. Captured frames and selections: `/var/folders/n8/y68yxhhn3tl_0zh1r9kscs3w0000gn/T/ship-tab-keys-slice1-pgaiufex/evidence.json`.

## 2. `client.tab.move` and its bindings

- [x] 2.1 Add `ClientTab::Move { direction }` and `TabDirection` in `crates/ship-client/src/keymap.rs`, `shift` and the `Move` arm returning `Turn::First(vec![Item::Move(..)])` in `crates/ship-client/src/ui/mod.rs`, and the four labeled bindings in `crates/ship-core/src/defaults.toml`.
- [x] 2.2 Write `specs/keymap/spec.md`: MODIFIED "Client actions" to list `client.tab.move`, and ADDED "Relative tab moves", "Tab moves at their turn" and "Default tabs keys" with the P1 and P2 scenarios.
- [x] 2.3 Verify with `ship tab list`: every P1 case gives the stated tree with the selection kept (FR-010); every P2 case leaves the output byte-for-byte unchanged with no status message (SC-002); three quick `shift-k` presses move three places (FR-008); the status line shows the four labels in tabs mode.
- [x] 2.4 Walk SC-001 once with keys only. For one `in` and one `out`, compare against the same move made by mouse drop (SC-004).

Slice 2 verification: on macOS, build, formatting, workspace Clippy with warnings denied, workspace tests (80 passed, 1 ignored) and strict OpenSpec validation passed. An owned temporary server on port 43331 and a client in an isolated tmux server passed 31 live checks: every P1/P2 case, no-selection and nested edges, labels, pane/subtree preservation, zoom preservation, rapid up/down repeats, the keys-only SC-001 workflow, and byte-for-byte keyboard/mouse `in` and `out` equivalence. Server request logs confirmed edges sent no move and valid presses sent one each. All 27 slice-1 live checks also passed again. Temporary servers and clients were stopped afterward. Linux is unverified. Frames, selections, trees and request counts: `/var/folders/n8/y68yxhhn3tl_0zh1r9kscs3w0000gn/T/ship-tab-keys-slice2-yxhsy9g8/evidence.json`; regression evidence: `/var/folders/n8/y68yxhhn3tl_0zh1r9kscs3w0000gn/T/ship-tab-keys-slice1-mvydh551/evidence.json`.

## 3. Docs

- [x] 3.1 Following `docs/agents/docs.md`, update the `alt-g` row in `docs/site/tabs.md` and the tabs mode sentence in `docs/site/keys.md` with the move keys and the `h`/`l` fallbacks.

Slice 3 verification: on macOS, `pnpm --dir website build`, strict OpenSpec validation and `git diff --check` passed. Checked both built HTML pages and their raw Markdown copies for the four move keys and `h`/`l` fallbacks, checked the keys page's link to the tabs page, and compared the documented keys with `defaults.toml`. Both page scopes are unchanged. The site build emitted warnings about the blog's `MarkdownContent` override and the missing or empty `i18n` collection. No Rust changed, so Rust checks were not rerun. Linux is unverified.
