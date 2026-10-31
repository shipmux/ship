# Tasks

The five groups are `design/program.md`'s build slices, in one pull request. The paper's skeleton map gives the types and signatures; record anything that differs in its deviation log.

Verification uses real commands and a running client, not permanent authored tests. Use an owned temporary server on a port from 43331 to 43337, with `XDG_STATE_HOME` and `XDG_CONFIG_HOME` pointed at temporary directories, and don't touch the server on 43179. Delayed-link checks use a temporary delay proxy of 500 ms each way, as earlier changes did. The delta specs under `specs/` were written ahead of the slices (see the program paper's deviation log), so each slice checks its delta still matches what it built instead of writing it. Every code group ends with `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` and `openspec validate ui-rename --type change --strict`. Checks run on macOS; Linux is unverified unless run there.

## 1. Input is read at its turn

- [x] 1.1 In `crates/ship-client/src/ui/keys.rs`, reshape `Handled` into `Action`, `Key`, `Paste` and `None`, and remove `Handled::Queue`, `Handled::Now` and `fn handled`.
- [x] 1.2 In `crates/ship-client/src/ui/lane.rs`, add `Item::Event`, `Item::Menu` and `Turn::Local`.
- [x] 1.3 In `crates/ship-client/src/ui/mod.rs`:
  - Replace `Local::menu` with `Local::overlay` (`Overlay::Menu` only).
  - Add `read`, `offline` and `land`, and pass `screen` to `turn`.
  - Return `Turn::Local` for mode, sidebar and reload actions.
  - Push keys, pastes and overlay mouse events to the lane unread, and right-clicks as `Item::Menu`.
  - Close an open overlay before click items run, and drop `Item::Mouse` and `Item::Resize` while one is open.
- [x] 1.4 Write the slice-1 deltas:
  - `specs/keymap/spec.md`: "Key actions act on the selection" and "Sidebar resize availability".
  - `specs/terminal-client/spec.md`: "Ordered input".
  - `specs/context-menus/spec.md`: "Selection when a menu opens", "Using an open menu" and "Menu items run as key actions".
  - `specs/mouse-input/spec.md`: ADDED "Mouse input queued before an overlay".
  - `specs/terminal-client/spec.md`: ADDED "Input while disconnected".
- [x] 1.5 Verify every slice-1 check in `design/program.md`.

### Slice-1 evidence

- macOS: formatting, Clippy with warnings denied, workspace tests (80 passed, 1 ignored), build and strict OpenSpec validation passed. No permanent tests added; Linux remains unverified.
- Owned temporary server, isolated config/state/runtime directories, and a Unix-socket proxy delaying traffic 500 ms each way. Cyan approved sockets in place of the paper's obsolete TCP-port setup. Existing servers were not used.
- Delayed `alt-n`, `alt-g`, `j`: 100/100 correct selections with no leaked `j`; mode bar stayed unchanged while create waited. Immediate typing after create and split-then-zoom: 10/10 each.
- Menu keyboard and mouse picks, typing before a right-click, queued wheel suppression with a mouse-tracking positive control, queued click dismissal, bracketed paste and resize passed. Idle mode/sidebar controls, reload, clicks, sidebar and pane-border drags, tab drops and one-shot redraw passed. The idle mode bar was observed within 9 ms in this probe, not a general latency benchmark.
- After abrupt server loss, modes, sidebar controls and explicit reload still worked; server-dependent input was dropped and `alt-q` detached.
- Cyan approved an additional client-side barrier: actions and routed clicks wait for the outbox to drain, while consecutive program input can still batch. Without it, the typing-before-menu probe sent selection request 10 while input request 9 was still buffered; the server refused the selection. The final run passed with no numbered-request refusals.
- Disposable verifier: `/tmp/ship-ui-slice1-verify.py`; final evidence, screen captures and logs: `/tmp/ship-ui-s1-_m1x1baf/`. Additional paste, menu-click and resize checks: `/tmp/ship-ui-slice1-extra.py`, evidence in `/tmp/ship-ui-s1-1eil9emo/`. Owned servers and tmux sessions were stopped.

## 2. Panes mode

- [ ] 2.1 In `crates/ship-core/src/defaults.toml`, add `alt-p` and the `panes` mode, and remove normal mode's `alt-shift-h/j/k/l`.
- [x] 2.2 Write `specs/keymap/spec.md`: MODIFIED "Default pane keys" and "Modes", ADDED "Default panes keys".
- [ ] 2.3 Verify every P3 case, the swap labels, `alt-shift-l` reaching a program, and SC-005.

## 3. The rename field

- [ ] 3.1 Add `tui-input` to the workspace and `ship-client` manifests with `default-features = false, features = ["crossterm"]`, and confirm one crossterm and one ratatui with `cargo tree -d`.
- [ ] 3.2 Add `crates/ship-client/src/ui/rename.rs`, plus `ClientTab::Rename` and `ClientPane::Rename` in `crates/ship-client/src/keymap.rs`.
- [ ] 3.3 In `crates/ship-client/src/ui/mod.rs`:
  - Add `Overlay::Rename`, the two `act` arms and `reconcile_rename`.
  - Keep the field open on resize and cut.
- [ ] 3.4 In `crates/ship-client/src/ui/draw.rs`, split the label functions, add `placeholder`, and draw the field row and its cursor in place of the mode bar.
- [ ] 3.5 Add both `r` bindings to `defaults.toml`.
- [x] 3.6 Write the slice-3 deltas:
  - `specs/keymap/spec.md`: MODIFIED "Client actions" and "Default tabs keys", ADDED "Rename actions".
  - `specs/terminal-client/spec.md`: ADDED the six rename field requirements.
- [ ] 3.7 Verify every slice-3 check in `design/program.md`.

## 4. Accent and menu items

- [ ] 4.1 Accent the target's row:
  - Pass the target to `Sidebar::draw` and `Tabs::draw` in `crates/ship-client/src/ui/sidebar/`.
  - Add `Scene::renaming` for the title row in `crates/ship-client/src/ui/panes.rs`.
- [ ] 4.2 Add the two `rename` menu items to `defaults.toml`.
- [x] 4.3 Write `specs/context-menus/spec.md`: MODIFIED "Default pane menu" and "Default tab menu".
- [ ] 4.4 Verify the accent, including a pane behind its stack's front, both menu items, and SC-001.

## 5. Docs

- [ ] 5.1 Following `docs/agents/docs.md`, update `docs/site/keys.md`, `docs/site/tabs.md` and `docs/site/panes.md` for panes mode, `alt-p`, renaming, and the removal of `alt-shift-hjkl` from normal mode.
