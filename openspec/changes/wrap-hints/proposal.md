# Proposal

Status: Locked. Cyan asked for this change and approved it in conversation on 2026-10-27 ("go for that then!"), with hints in aligned columns. It is small enough to need no design papers; this proposal and the deltas under `specs/` are the contract.

## Why

The status line is one row, and hints that don't fit are cut from the end. ratatui clips the line wherever it runs out, so a hint can lose half its label. In a 120-column terminal with the default 26-column sidebar, the pane area is 94 columns wide, but resize mode needs about 134 and tabs mode, after #83 adds its four move keys, needs 127. In both, `esc normal` is cut, so a user can't see from the hints how to leave the mode.

## What Changes

- When everything fits on one row, the status line looks as it does today.
- When it doesn't, the first row keeps the mode's name and any messages, and the hints go below it in aligned columns, like Neovim's which-key. Each column is as wide as the widest hint, as many columns fit as the pane area allows, and hints fill left to right in config order. The overlay grows upward to fit.
- A hint is never split across rows or cut in the middle. Only when the pane area is too short for every row are hints cut from the end, whole.
- The overlay still covers only the pane area, never the sidebar, and still changes no pane's size.
- The "Active mode" scenario now lists resize mode's `shift-j split down` and `shift-k split up`, which the defaults already bind.

Not in this change:
- Hints in normal mode, grouping or ordering hints other than by config order, or a key that toggles the hints.
- Truncating long messages any differently. A message still shows its first line, cut at the row's edge.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `terminal-client`: "Status line" allows more than one row, and "Mode key hints" keeps hints whole. A new "Hint layout" requirement says how they wrap.

## Impact

- `ship-client`: `status_line` in `ui/draw.rs` lays out rows instead of one `Line`, and the overlay `Rect` there takes its height from them. Clearing the cells and hiding the cursor beneath it already work for any `Rect`.
- Docs: `docs/site/configuration.md:101` says "a one-line hint bar".
- No dependency changes. About 40 lines of implementation Rust, and no tests.
