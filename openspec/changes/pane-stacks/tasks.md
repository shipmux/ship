# Tasks

These are checkpoints for implementing the locked plan, one slice at a time. Cyan locked the papers in `design/` on 2026-10-29, and they are the contract. `design/program.md` holds the file map, signatures and checks for each slice. Complete and verify one slice, then stop for Cyan's review before starting the next. Record surprises in the program paper's deviation log.

Cyan approved removing backward-compatibility requirements on 2026-10-29. The revised slice 1 uses direct serde and verifies current-format save/restore instead of legacy migration.

Verification uses real workflows and temporary probes, not permanent authored tests. For each slice, run these checks:
- `cargo fmt --all -- --check`;
- `cargo clippy --workspace --all-targets -- -D warnings`;
- `cargo deny check`;
- debug and release builds;
- `openspec validate pane-stacks --type change --strict`.

Use an owned temporary server on a scratch name, with `XDG_RUNTIME_DIR`, `XDG_STATE_HOME` and `XDG_CONFIG_HOME` pointed at temporary directories. Don't touch existing user work, including the `default` server. Checks run on macOS. Linux counts only when run there and is otherwise reported unverified.

## 0. Settle the blocking unknowns

- [x] 0.1 Find the mitsein 0.9 call that turns a one-item `VecDeque1<Pane>` into its item by value, with a throwaway probe outside the workspace. If none exists, plan `Slot::take` to convert to `VecDeque` first. Record the answer in the program paper's deviation log.
- [x] 0.2 Resolve the cycle-key blocker: the current-client PTY probe missed legacy `alt-[`; Cyan approved `alt-,` (previous) and `alt-.` (next), and both replacement bindings fired in a scratch config. Physical terminal checks remain unverified.

## 1. Every leaf is a slot

- [x] 1.1 Add `mitsein` with `serde` to the workspace and ship-core. Add `Slot`, `Taken` and `CycleDirection` to `ship-core/src/layout.rs`, replace `Layout::Pane` with `Layout::Slot`, and add `Tab::slot` to `model.rs`. Derive `Layout` serde directly, removing the existing private legacy leaf adapter and its conversion.
- [x] 1.2 Set `PROTOCOL_VERSION` to 20.
- [x] 1.3 In `ship-server`, replace `layout::close` with `layout::take` (still clearing zoom as today), make geometry walk slots, and build `CreateTab` and `restore_tab` leaves as slots of one. Update client code that matched `Layout::Pane`.
- [x] 1.4 Verify: `ship tab get | jq .layout` shows `"kind": "slot"` leaves with a `panes` list; slot-format saved tabs restore with the same IDs, names, nesting, pane order, proportions and zoom; splitting, closing, swapping, resizing and zooming behave as before; `ship server status` reports protocol 20.

## 2. Stack and cycle from the CLI and keys

- [x] 2.1 Add `Place` and `CyclePane` to `protocol.rs`, replacing `CreatePane.direction` with `place`. Add `Placement`, `Create.place` and `Cycle` to `command.rs`, without a creation alias for `direction`.
- [x] 2.2 In `state/layout.rs`, add `stack`, `cycle`, `bring_to_front` and `successor`, and give `take` the zoom rules in the program paper's table. Add the title row for borderless stacked slots in `state/geometry.rs`.
- [x] 2.3 In `state.rs`, match create on `Place`, add the `Cycle` message, add the front rule to `repair` before the zoom rule, bring a behind pane forward in the select handler and in `SetTab`'s zoom, and compare slots for the select unzoom. Add `POST /api/v0/panes/{id}/cycle` in `routes.rs` and `lib.rs`.
- [x] 2.4 Wire `cycle` and `place` through `ship-client`'s `api.rs` and `execute.rs`. In `defaults.toml`, change `direction` to `place`, and add `alt-s`, `alt-.`, `alt-,` and the "stack" menu item.
- [x] 2.5 Update `docs/site/keys.md`, `GLOSSARY.md` (slot, stack) and `docs/agents/code.md` (`direction` to `place` in the create-body rule).
- [x] 2.6 Verify: `ship pane create --place stack` twice, then `ship pane cycle --direction next` and `--direction prev`, give P1's orders in `ship tab get`; `--place right` splits; closing the front shows the next pane and closing the last removes the slot; each P3 step leaves `zoomed` set and naming the front; a selection on a pane pushed behind ends on the new front; `ship view select` of a behind pane brings it forward; a restart restores a stacked slot in order; in the client, `alt-s`, `alt-.` and `alt-,` work and only the front is drawn.

## 3. Titles and title clicks

- [x] 3.1 In `ui/panes.rs`, draw every pane's label in the slot's top border, front first and marked, joined with ` · `, and draw the title row of a borderless stacked slot.
- [x] 3.2 In `ui/mouse.rs`, add `title_at` and send a select for a clicked title, with a behind pane's title winning over a border grab.
- [x] 3.3 Make `Step::Pane` in `ui/mod.rs` walk fronts only, and map the remembered pane to its slot's front in `ui/memory.rs`'s `landing`.
- [x] 3.4 Describe stacks and cycling in `docs/site/panes.md`.
- [x] 3.5 Verify: a stacked slot's border reads `X · A · B` with the front marked; a lone stacked slot shows the title row and loses it when unstacked; clicking `B`'s title selects `B` and the order becomes `[B, X, A]`; `alt-tab` skips panes behind; cycling feels as quick as moving between split panes, judged in use.

## 4. Move panes

- [x] 4.1 Add `MovePane` to `protocol.rs` and `Move` to `command.rs`, with exactly one of `--onto` and `--beside`, and `--direction` only with `--beside`.
- [x] 4.2 Add the `PaneMove` message in `state.rs`: refuse `at == id` with `invalid_structure`, `take` the pane, place it with the same `Place` match as create, and answer with the destination tab. Unzoom the destination if needed to keep a selected moved pane visible, including a same-tab move. Add `POST /api/v0/panes/{id}/move` in `routes.rs` and `lib.rs`.
- [x] 4.3 Wire `move_pane` through `api.rs` and `execute.rs`, and describe moving in `docs/site/panes.md`.
- [x] 4.4 Verify: `ship pane move --onto` and `--beside --direction right` behave as in P4 within a tab and across tabs; `top` running in the moved pane keeps running with the same pane ID; an emptied slot is gone; moving onto or beside itself fails and `ship tab get` is unchanged; selected panes follow their move and unzoom the destination only when needed to remain visible.

## 5. Edit-scrollback

- [x] 5.1 Add `etcetera`, `tempfile` and tokio's `process` feature to ship-client, and `EditScrollback` to `command.rs`.
- [x] 5.2 In `automation.rs`, add `edit_scrollback`: resolve the target from `--pane` or the pane behind `--from`, read all lines unwrapped, sweep unlocked files from the scrollback directory, write and lock a 0600 temp file, run the editor through `sh -c`, then delete the file. Off Unix, fail with a message. Route it from `execute.rs`.
- [x] 5.3 Add `alt-e` and the "edit scrollback" menu item to `defaults.toml`, and describe edit-scrollback in `docs/site/panes.md` and `docs/site/keys.md`.
- [x] 5.4 Verify: `alt-e` on a pane that printed `seq 1 5000` opens the editor with all 5000 lines; quitting returns to the pane with the layout unchanged; an `EDITOR="code --wait"` style value works; the temp file is mode 0600 while open and gone after; a file left by a killed run is removed by the next run while a concurrent run's file is kept; running it in a pane alone in its slot fails with the "no pane behind" message; with `EDITOR=/nonexistent` the pane stays open showing the error.

## 6. Specs and size

- [ ] 6.1 Check the deltas under `specs/` against what slices 1 to 5 built, and update them and `proposal.md` for any deviation.
- [ ] 6.2 Verify: `openspec validate pane-stacks --type change --strict` passes, and the size report gives implementation Rust and tests separately.
