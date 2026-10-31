# Proposal

Status: Locked with `design/program.md` on 2026-10-31. That paper is the only design paper and the contract; Cyan chose to skip the product and architecture papers because product behavior stays the same. This proposal is a view assembled from it.

## Why

Ship's implementation is 13,732 lines of Rust by `cloc` (code lines only), and parts of it say the same thing more than once. GET routes go through the state actor and recompute geometry `commit` just published. A shell prompt can turn into three commits and three full replicas. New panes are sized by a second copy of the layout arithmetic. The client's event loop holds the attach stream's whole lifecycle in loose variables, and its input passes through enums that mostly rename lane items. Smaller repetitions run through every crate: lookups written out by hand, conversions written twice, and suppressed lints that cleaner code wouldn't trip.

## What Changes

- **Reads come from the published replica.** The state actor handles writes only and publishes each replica inside `commit`. The revision `AtomicU64` goes. The relay bus stays for pane events and clipboard.
- **Panes report their state.** One event per change carries a pane's title, directory, program records and exit status, so one frame is at most one commit.
- **New panes are sized by `geometry::tab`**, through a trial placement, as resize already works.
- **The client loop becomes a dispatcher.** Keys and the mouse produce lane items directly, navigation reads one view per turn, and a `Connection` owns attaching, reconnecting and area updates.
- **Shared shapes are stated once**, in `ship-core` where both sides use them, and four restriction lints encode `docs/agents/code.md`'s conventions.
- **The sidebar's tab tree is drawn from Ship's own rows**, and `tui-tree-widget` goes (O-1, settled by Cyan).

Not in this change: removing the relay bus, any protocol, config or key change, new tests, and user-visible behavior changes.

Visible on the wire only: every-revision feeds see fewer, merged revisions; a commit can no longer fail with `Unavailable` after its edit; reads no longer wait behind queued writes.

## Capabilities

None modified. No requirement in `openspec/specs/` changes, so this change has no spec deltas.

## Impact

- `ship-core`: `tree.rs`, `screen.rs`, `layout.rs`, `command.rs`, `id.rs`, `model.rs`, `geometry.rs`, `error.rs`, `relay.rs`.
- `ship-server`: `lib.rs`, `app.rs`, `replicas.rs`, `state.rs` and `state/`, `pane.rs`, `routes.rs`, `views.rs`, `input.rs`, `order.rs`, `attach.rs`, `saver.rs`.
- `ship-client`: `lib.rs`, `api.rs`, `execute.rs`, and most of `ui/`, with new `ui/nav.rs` and `ui/connection.rs`.
- `ship`: `main.rs`, `cli.rs`, `diagnostics.rs`, `runlog.rs`.
- Workspace `Cargo.toml`: four new Clippy lints. `docs/agents/code.md`: one line naming them.
- Size: about 855 fewer code lines by `cloc`, 85 of them from the sidebar slice, estimated, not measured. No tests are planned.
- One pull request for the whole change.
