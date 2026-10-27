# Design

The design for this change lives in three papers under `design/`. They are the source of truth, and this file only points at them.

- [`design/product.md`](design/product.md): in tabs mode, `h`/`l` fall back to selecting the parent or first child, and `shift-k`/`shift-j`/`shift-l`/`shift-h` move the selected tab up, down, in and out with the new `client.tab.move` action. Locked.
- [`design/architecture.md`](design/architecture.md): shape A. The client resolves a direction at its turn and queues the same `Item::Move` a mouse drop queues, so the server doesn't change. Locked.
- [`design/program.md`](design/program.md): the file map and three build slices: `h`/`l`, then `client.tab.move` and its bindings, then docs. Locked.
