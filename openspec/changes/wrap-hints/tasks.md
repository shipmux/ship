# Tasks

Verification uses real commands and a running client, not permanent authored tests. Use an owned temporary server on a port from 43331 to 43337, with `XDG_STATE_HOME` and `XDG_CONFIG_HOME` pointed at temporary directories, and don't touch the server on 43179. Checks run on macOS; Linux is unverified unless run there.

## 1. Hints in columns

- [ ] 1.1 In `crates/ship-client/src/ui/draw.rs`, make `status_line` lay out rows for a given width: one row when everything fits, else the first row and a column grid of whole hints. Size the overlay `Rect` from the row count, capped at the pane area's height.
- [ ] 1.2 Change "a one-line hint bar" in `docs/site/configuration.md:101`, following `docs/agents/docs.md`.
- [ ] 1.3 Verify in a running client:
  - In a terminal wide enough for resize mode on one row, the status line looks as it did before the change.
  - At 120 columns with the default sidebar, tabs mode and resize mode show every hint whole, in config order, with chords aligned in columns, and `esc normal` visible.
  - With the pane area a few rows tall, the overlay takes no more rows than the pane area has, and the hints that don't fit are cut whole from the end.
  - `stty size` in a pane reports the same size in normal mode and in each mode, and the overlay never covers the sidebar.
  - An exited pane and a config error still show on the first row.
  - `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `openspec validate wrap-hints --type change --strict`.
