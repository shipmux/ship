# Tasks

Verification uses real commands and a running client, not permanent authored tests. Use an owned temporary server on a port from 43331 to 43337, with `XDG_STATE_HOME` and `XDG_CONFIG_HOME` pointed at temporary directories, and don't touch the server on 43179. Checks run on macOS; Linux is unverified unless run there.

## 1. Hints in columns

- [x] 1.1 In `crates/ship-client/src/ui/draw.rs`, make `status_line` lay out rows for a given width: one row when everything fits, else a column grid of whole hints above the mode's row. Size the overlay `Rect` from the row count, capped at the pane area's height.
- [x] 1.2 Change "a one-line hint bar" in `docs/site/configuration.md:101`, following `docs/agents/docs.md`.
- [x] 1.3 Verify in a running client:
  - In a terminal wide enough for resize mode on one row, the status line looks as it did before the change.
  - At 120 columns with the default sidebar, tabs mode and resize mode show every hint whole, in config order, with chords aligned in columns, and `esc normal` visible.
  - With the pane area a few rows tall, the overlay takes no more rows than the pane area has, and the hints that don't fit are cut whole from the end.
  - `stty size` in a pane reports the same size in normal mode and in each mode, and the overlay never covers the sidebar.
  - An exited pane and a config error still show on the mode's row, the last one.
  - Hints wider than the pane area are omitted whole, and remaining hints stay in config order, including labels with wide or combining characters (Cyan's implementation-time amendment).
  - `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `openspec validate wrap-hints --type change --strict`.

## Verification evidence

Verified on macOS on 2026-10-27 using an owned temporary server on port 43331 and isolated state/config directories. Linux remains unverified.

- All 23 live-client checks passed. At 120 columns both modes show every hint, including `esc normal`, in four aligned columns below the header. At 1, 2 and 3 rows the overlay stays within the pane area and omits whole hints from the end. Checks also covered 80-column terminals and exact one-row width boundaries.
- Wide one-row text and ANSI styles matched captures from the pre-change client byte-for-byte. `stty size`, published geometry, attachment area and sidebar cells were unchanged by either mode. Leaving a mode restored covered pane cells and the cursor; the overlay hid a cursor beneath it.
- An exited pane and a real config hot-reload error stayed on the header row. Over-wide hints were omitted without hiding later hints; CJK and combining labels aligned by display width. Empty modes and modes with only over-wide hints used just the header.
- `cargo build --workspace`, formatting, Clippy with warnings denied, workspace tests (80 passed, 1 ignored), strict change validation and `git diff --check` passed. No dependencies or permanent tests were added.
- After review, the mode's row moved from the top of the overlay to the bottom, with the hint grid above it, and a short pane area now drops hint rows above that row instead of the row itself. Checked in a tmux client on port 43331: at 120 columns the three hint rows sit above ` TABS `, at 200 columns tabs mode is still one row, and at 3 rows the overlay shows two hint rows and ` TABS `, without `esc normal`. Clippy with warnings denied and formatting passed. The checks above that mention the header row were run on the earlier layout.
- The website build passed, and the updated sentence appeared in the built HTML and raw Markdown. The build repeated the existing blog `MarkdownContent` override and missing/empty `i18n` collection warnings; neither was changed by this slice.
