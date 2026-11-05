# Design

The design for this change lives in three papers under `design/`. They are the source of truth, and this file only points at them.

- [`design/product.md`](design/product.md): `ship watch` prints the server's state as JSON lines. Tags put short text, optionally colored, on tabs and panes through `ship pane set` and `ship tab set` or a merge-patch `PATCH`. `[client.tabs] rows` and `[client.agents] rows` show tags in the sidebar, with a tab falling back to its first pane's. A git-branch example plugin proves the pieces fit. Locked; OU-1 to OU-3 open.
- [`design/architecture.md`](design/architecture.md): shape A. Tags are attributes on `Pane` and `Tab`, applied by the existing `Set` handlers. `watch` is client-side over the observer attach. Tab entries become one sidebar row per line. Locked.
- [`design/program.md`](design/program.md): the file map and six slices: tags in core and the server, the CLI flags, `ship watch`, agent rows, multi-line tab entries, then the example and docs.
