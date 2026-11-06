# Tasks

One group, `design/program.md`'s single slice, shipped in one pull request. It was built as a demo before the papers were written, and the papers record it.

Verification uses an owned server on its own socket, not any server already running. The group ends with `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` and `openspec validate focus-reports --type change --strict`. Checks ran on macOS; Linux is unverified.

## 1. Focus reports end to end

- [x] 1.1 Add `focused` to `AttachRequest`, `ViewInput` and `ViewingRecord`, and bump `PROTOCOL_VERSION` to 23. Verify that `/health` reports 23.
- [x] 1.2 Add `focused_panes` and `apply_focus` to `state.rs`, store `focused` in `Attach` and `SetView`, and add `PaneCommand::Focus`. Verify with a witness program that enables mode 1004 that attaching, blurring, refocusing, selecting, cycling, closing and crashing a window give the reports `design/product.md` describes.
- [x] 1.3 In the vendored session, report only when mode 1004 is on, report the current focus when it turns on, and render with `.focused(true)`. Verify that a pane that never enabled the mode receives no bytes, that a pane created with no window attached receives `ESC [ O`, and that the published cursor is unchanged when the window blurs.
- [x] 1.4 In the client, send `focused` on attach, reattach and through the view PUT, renaming `resized`/`Resize` to `changed_view`/`ViewPut`. Verify by hand in Ghostty that switching applications and panes gives the expected reports.
- [x] 1.5 Verify with a randomized stress run over attach, crash, focus, select pane or tab, split, stack, cycle and close, including concurrent bursts, that every program's last report matches the focus rule, with no repeated reports. Check the same harness fails against a build that ignores `focused`.
- [x] 1.6 Add the focus sentence to `docs/site/windows-and-the-server.md` and the protocol number in `docs/site/scripting/writing-a-plugin.md`.
- [x] 1.7 Check that the `session-observation` and `health-exchange` deltas match what was built.
