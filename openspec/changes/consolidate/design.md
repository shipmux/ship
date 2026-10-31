# Design

The design for this change lives in one paper under `design/`. It is the source of truth, and this file only points at it.

- [`design/program.md`](design/program.md): the virtual skeleton and four build slices. Reads come from the published replica, panes report their state, new panes are sized by `geometry::tab`, the client loop becomes a dispatcher over lane items, a navigation view and a `Connection`, and shared shapes and lints are stated once. Behavior stays the same. Locked on 2026-10-31.

There is no `product.md` or `architecture.md`. Cyan chose to skip them because product behavior doesn't change; the program paper records its five key decisions in its rationale.
