# Design

The design for this change lives in three papers under `design/`. They are the source of truth, and this file only points at them.

- [`design/product.md`](design/product.md): a one-line rename field on the mode bar for tabs and panes, opened from tabs mode's `r`, a new panes mode's `r`, or the right-click menus. Panes mode (`alt-p`) also takes over pane focus, swap and cycling, and `alt-shift-hjkl` leaves normal mode. Locked.
- [`design/architecture.md`](design/architecture.md): shape D. Keys and pastes wait in the lane unread and are interpreted at their turn, so the field opens at its turn and every key typed after it reaches it. Mode, sidebar and reload actions lose their press-time path, and the right-click menu opens at its turn. Locked.
- [`design/program.md`](design/program.md): the file map and five build slices: input at its turn, panes mode, the rename field, the accent and menu items, then docs. Locked.
