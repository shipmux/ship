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

- [x] 2.1 In `crates/ship-core/src/defaults.toml`, add `alt-p` and the `panes` mode, and remove normal mode's `alt-shift-h/j/k/l`.
- [x] 2.2 Write `specs/keymap/spec.md`: MODIFIED "Default pane keys" and "Modes", ADDED "Default panes keys".
- [x] 2.3 Verify every P3 case, the swap labels, `alt-shift-l` reaching a program, and SC-005.

### Slice-2 evidence

- Only defaults changed: `alt-p` enters sticky panes mode; focus, swap and cycle reuse existing actions; Esc returns to normal. The four normal-mode `alt-shift-h/j/k/l` bindings are removed. Both `r` bindings remain for slice 3, as the paper specifies.
- P3 and SC-005 passed in a running client on both a direct connection and the approved 500 ms each-way Unix-socket proxy, 17 checks each. Read back selection, geometry and stack order with `ship view selected` and `ship tab get`: all four focus and swap directions work, swapped panes stay selected, and both cycle directions work in a three-pane stack. Normal-mode focus, Alt-Tab and both cycle directions still work.
- Captured the panes-mode bar with all four `swap` labels. Verified the mode stays active after navigation, swaps and cycling, drops unbound typing, and returns ordinary program input after Esc. In normal mode a raw-input program printed and recorded `1b48`, `1b4a`, `1b4b` and `1b4c` for Alt-Shift-H/J/K/L, without moving selection or changing geometry.
- Parsed old and new defaults to confirm every moved binding uses its original action and unrelated defaults are unchanged. Formatting, Clippy with warnings denied, workspace tests (80 passed, 1 ignored), build and strict OpenSpec validation passed on macOS. No Rust or permanent test code added; Linux and physical-terminal interaction remain unverified.
- Disposable verifier: `/tmp/ship-ui-slice2-verify.py`, reusing slice 1's isolated harness. Evidence and screen/layout captures: `/tmp/ship-ui-s2-vlv_wt6e/` (direct) and `/tmp/ship-ui-s2-j5tfrihe/` (delayed). Owned servers and tmux sessions were stopped; existing servers were not used.

## 3. The rename field

- [x] 3.1 Add `tui-input` to the workspace and `ship-client` manifests with `default-features = false, features = ["crossterm"]`, and confirm one crossterm and one ratatui with `cargo tree -d`.
- [x] 3.2 Add `crates/ship-client/src/ui/rename.rs`, plus `ClientTab::Rename` and `ClientPane::Rename` in `crates/ship-client/src/keymap.rs`.
- [x] 3.3 In `crates/ship-client/src/ui/mod.rs`:
  - Add `Overlay::Rename`, the two `act` arms and `reconcile_rename`.
  - Keep the field open on resize and cut.
- [x] 3.4 In `crates/ship-client/src/ui/draw.rs`, split the label functions, add `placeholder`, and draw the field row and its cursor in place of the mode bar.
- [x] 3.5 Add both `r` bindings to `defaults.toml`.
- [x] 3.6 Write the slice-3 deltas:
  - `specs/keymap/spec.md`: MODIFIED "Client actions" and "Default tabs keys", ADDED "Rename actions".
  - `specs/terminal-client/spec.md`: ADDED the six rename field requirements.
- [x] 3.7 Verify every slice-3 check in `design/program.md`.

### Slice-3 evidence

- Added `ui/rename.rs`, both client rename actions and both `r` bindings. The field reuses the input lane, menu pick result and existing server name-set commands. It holds one fixed target and a `tui-input` buffer; drawing reads the automatic label from the replica. Closed targets are reconciled before queued input runs. No server, protocol, CLI, sidebar accent or menu-entry changes.
- Cargo added only `tui-input` 0.15.5, with defaults off and `crossterm` enabled. `cargo tree -d`, inverse trees and the lockfile confirm one version each of crossterm (0.29.0) and ratatui (0.30.2). The feature tree shows tui-input enables crossterm defaults, already enabled by existing dependencies.
- P1, P2's keyboard cases and P4 passed in a running client, with 21 direct and 24 delayed checkpoints. Read back stored names with `ship tab get` and `ship pane get`. Verified prefill and end cursor, dim automatic placeholders, trimming and clearing, line editing, multiline paste, mode preservation, hidden-sidebar and lone-pane fields, missing-target no-ops, fixed targets after selection changes, and local text winning over a concurrent rename.
- With 500 ms each way, create then `alt-p r foo` Enter passed 10/10: only the new pane is named `foo`, and neither shell receives `foo`. Queued tab navigation then rename names the newly selected tab. A second client sees no field or half-typed name, then sees the submitted name after Enter.
- Esc and drawn left/right clicks left `ship tab list` byte-for-byte unchanged; clicks neither changed selection nor reached a program or opened a menu. Pane exit and external tab closure closed the field with `pane closed` or `tab closed`; after closure and geometry updates settled, tab-list bytes stayed unchanged. A deliberately raced delayed rename received HTTP 404 and showed the ordinary request-failed status.
- Title and directory placeholders updated during editing. Unicode grapheme editing and cursor placement, horizontal scrolling, resize, and config reload passed. A cut preserved field text; offline Enter closed it, and the restarted owned server's log contained no PATCH after reconnect.
- Seven additional delayed checks passed: queued left and right clicks, queued wheel suppression with a mouse-tracking positive control, live directory changes, an empty nested tab's sibling-number placeholder, and retaining/submitting text across a two-column, two-row terminal. Fourteen existing idle, offline, menu, mouse, drag, reload, paste and resize regression checkpoints also passed.
- macOS: formatting, Clippy with warnings denied, workspace tests (80 passed, 1 ignored), build, strict OpenSpec validation and `git diff --check` passed. No permanent tests added. Linux and physical-terminal interaction remain unverified.
- Disposable verifiers: `/tmp/ship-ui-slice3-verify.py` and `/tmp/ship-ui-slice3-extra.py`. Final evidence, screen captures and logs: `/tmp/ship-ui-s3-waxhwek0/` (direct), `/tmp/ship-ui-s3-sm_948fs/` (delayed), and `/tmp/ship-ui-s3-extra-h7uoayv_/` (extra). Regression evidence: `/tmp/ship-ui-s1-rqhfxrk7/` and `/tmp/ship-ui-s1-ee9s4ln2/`. All used isolated owned servers; servers and tmux sessions were stopped.

## 4. Accent and menu items

- [x] 4.1 Accent the target's row:
  - Pass the target to `Sidebar::draw` and `Tabs::draw` in `crates/ship-client/src/ui/sidebar/`.
  - Reuse the selected-pane title accent in `crates/ship-client/src/ui/panes.rs`; the approved selection-cancellation amendment supersedes the original `Scene::renaming` plan.
- [x] 4.2 Add the two `rename` menu items to `defaults.toml`.
- [x] 4.3 Write `specs/context-menus/spec.md`: MODIFIED "Default pane menu" and "Default tab menu".
- [x] 4.4 Verify the accent, both menu items, and SC-001. The approved cancellation-on-repair decision supersedes the original persistent rear-pane accent case; verify that a stack-front change cancels a pane edit when the server repairs this attachment's selection.
  - Also verify Cyan's approved selection-cancellation amendment: leaving the target or clearing selection cancels, same-tab pane changes keep a tab edit, other-client navigation keeps the edit, and deletion messages take precedence.

### Slice-4 evidence

- Added `rename` immediately before `close` in both default menus. The sidebar's selected row takes the bold cyan accent while its tab field is open, including through the tree widget's selected-band styling. Pane fields reuse the existing selected-pane title accent; no separate pane drawing state is needed under the amended selection invariant.
- Cyan approved cancelling without saving when this attachment leaves the captured tab or pane, including absent selection. Same-tab pane changes preserve a tab field; other attachments navigating preserve an edit unless they change this attachment's relevant selection. Deletion feedback still takes precedence. The product, architecture, program and terminal-client delta record the amendment; the original FR-002 is marked superseded for selection changes.
- The live shared-stack probe showed the server repairs each viewer's pane selection to the stack's front. Cyan explicitly chose `Cancel on repair`, superseding the old persistent rear-pane edit workflow and its accent check. Verified that another client fronting a different pane cancels the pane field without a PATCH and restores the old target's dim rear label. The corresponding tab edit remains open because its viewed tab is unchanged.
- On the final build, the slice-4 verifier passed 19 direct and 20 delayed checkpoints through the approved 500 ms each-way Unix-socket proxy. ANSI captures verify the selected tab's bold cyan foreground on the dark-gray band, the selected stack pane's title accent, and clearing the rename accent. Keyboard-only tab and pane name/rename/clear workflows passed SC-001. Both menus preserve their required order and rename the right-clicked unselected item, using a keyboard pick for the tab menu and a mouse pick for the pane menu.
- Selection probes covered another tab selected by tab or pane ID, another pane in the same or another tab, selecting the owning tab without a pane, clearing selection for both field kinds, reselecting the same pane, same-tab pane changes and second-client navigation. Cancelled drafts left all checked names unchanged and produced no PATCH; pane and tab deletion kept `pane closed` and `tab closed`. Delayed create then `alt-p r foo` Enter passed 10/10 without typing into either shell.
- The earlier rename verifier, with its superseded fixed-target selection case updated to expect cancellation, passed 21 direct and 24 delayed regression checkpoints on the final build. These cover line editing, paste, placeholders, hidden sidebar, missing targets, capture and click dismissal, Unicode/resize, second-client visibility, concurrent naming, reload, deletion, server rejection and offline/reconnect. Its delayed rapid-typing case also passed 10/10. Eleven idle/offline input regressions and three delayed paste/menu/resize regressions passed: 98 runtime checkpoints total across the six final runs.
- macOS: formatting, Clippy with warnings denied, workspace tests (80 passed, 1 ignored), build, strict OpenSpec validation and `git diff --check` passed. Cyan approved two local documented `#[expect(clippy::too_many_arguments)]` annotations on the sidebar draw functions; no other warnings are suppressed. No permanent tests added. Linux and physical-terminal interaction remain unverified.
- Disposable verifiers: `/tmp/ship-ui-slice4-verify.py` and `/tmp/ship-ui-slice4-rename-regression.py`, plus the existing slice-1 idle/extra verifiers. Final slice-4 evidence and captures: `/tmp/ship-ui-s4-pbtntoo0/` (direct), `/tmp/ship-ui-s4-2csle2i2/` (delayed). Rename regressions: `/tmp/ship-ui-s4-regression-j9o83ezj/` and `/tmp/ship-ui-s4-regression-mnvxptdp/`. Input regressions: `/tmp/ship-ui-s1-a7rpqpm5/` and `/tmp/ship-ui-s1-__0ydjas/`. All used owned temporary servers with isolated XDG config/state/runtime directories; the servers and tmux sessions were stopped. Existing servers were not used.

- Follow-up spacing requested by Cyan: added one blank column between the rename pill and input for both target kinds, collapsing the gap when needed to retain an input cell. Ten live padding/cursor/resize checks passed, including two-column width and submitting the preserved text after expansion; formatting, Clippy, workspace tests (80 passed, 1 ignored), build and diff checks passed on macOS. Disposable probe: `/tmp/ship-ui-rename-padding.py`; evidence: `/tmp/ship-ui-s1-8xdkcv70/`. Linux and physical terminals remain unverified.

- Follow-up color correction approved by Cyan: the spacer now shares the input bar's explicit black background instead of exposing the terminal default. Paint the row background once before the cyan pill and input. Twelve live checks passed, including ANSI background assertions across both spacers and input bars, cursor placement and narrow resizing. Evidence: `/tmp/ship-ui-s1-s1u6cb7x/`; formatting, Clippy, workspace tests (80 passed, 1 ignored) and build passed on macOS. Linux and physical terminals remain unverified.

### Follow-up: default tabs shortcut

- Cyan approved replacing normal mode's `alt-g` with `alt-t`, without an alias, and updating its guide tables and current OpenSpec records. The three locked papers carry the approved amendment; current keymap and terminal-client delta scenarios use `alt-t`. Earlier evidence remains a record of the former binding. Only the shortcut entries in `docs/site/keys.md` and `docs/site/tabs.md` changed; task 5.1 remains pending.
- The final shortcut verifier passed seven direct and eight delayed checkpoints through the approved 500 ms each-way Unix-socket proxy. A raw-input program received exactly `ESC g` for `alt-g` in normal mode; `alt-t` opened tabs mode without reaching the program. Navigation, remembered-pane landing, `alt-t r` rename, returning to normal mode, and queued `alt-t j r` all passed. `alt-p`, `alt-r` and `alt-tab` retained their existing actions.
- Delayed create then `alt-t j r` and immediate typing passed 10/10: beta was renamed rather than alpha or the newly created tab, and no draft reached either shell or the raw-input program. All probes used owned temporary servers with isolated XDG config/state/runtime directories; servers and tmux sessions were stopped.
- macOS: formatting, Clippy with warnings denied, workspace tests (80 passed, 1 ignored), build, strict OpenSpec validation, diff checks and the website build (`pnpm --dir website build`, 21 pages) passed. No permanent tests added. Linux and physical terminals remain unverified.
- Disposable verifier: `/tmp/ship-ui-tabs-shortcut.py`. Final evidence and captures: `/tmp/ship-ui-s1-8k2ckt73/` (direct) and `/tmp/ship-ui-s1-ekuhtq9j/` (delayed). Build log: `/tmp/ship-tabs-shortcut-site.log`.

## 5. Docs

- [ ] 5.1 Following `docs/agents/docs.md`, update `docs/site/keys.md`, `docs/site/tabs.md` and `docs/site/panes.md` for panes mode, `alt-p`, renaming, and the removal of `alt-shift-hjkl` from normal mode.
