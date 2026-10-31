# Tasks

The four groups are `design/program.md`'s build slices. The paper's skeleton map gives the types and signatures, and each slice's "Done when" is its completion evidence; record anything that differs in its deviation log.

Verification uses real commands and a running client, not permanent authored tests. Use an owned temporary server with `XDG_STATE_HOME`, `XDG_CONFIG_HOME` and `XDG_RUNTIME_DIR` pointed at temporary directories, and never a server already running. Every group ends with the paper's regression walk, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo build`, `cargo test --workspace` and a `cloc --include-lang=Rust crates` count against the previous group. Checks run on the implementing machine; the other priority platform is reported unverified.

## 1. Shared shapes and lints

- [x] 1.1 Add `tree::pane_tab`, `tree::parent` and `tree::panes` in `ship-core`, and `pane_tab_mut` and `pane_mut` in `ship-server/src/state/tree.rs`; replace the hand-written chains.
- [x] 1.2 Move screen style and color conversions into `ship-core/src/screen.rs`, add `From<Rect> for Size` and `Display for Scrolled`, and use them from `pane.rs`, `ui/panes.rs` and `ui/draw.rs`.
- [x] 1.3 Add the `Axis` and `Direction` helpers and use them in `state/layout.rs`, `state/geometry.rs`, `ui/mouse.rs`, `ui/mod.rs` and `sidebar/mod.rs`.
- [x] 1.4 Make `Prefixed` an associated const, rename `Id::new` to `Id::random`, and add `id_map_schema` and `RectSchema`.
- [x] 1.5 Return `ErrorCode` by value from `AppError::code`, share `data_mut`, and replace `rename::Target` with `NodeId`.
- [x] 1.6 Settle open choice O-3, add the four lints to the workspace `Cargo.toml`, and name them in `docs/agents/code.md`.
- [x] 1.7 Rewrite `err!` without the glob import, add `ServeOptions` with `SavedTabs`, and `Startup`, move `AttachmentGuard` into `Progress`, and replace `runlog.rs`'s `unwrap`s.
- [x] 1.8 Remove the unused sinks from `relay.rs`.
- [x] 1.9 Apply the server cleanups: state helpers, `layout::place`, `unzoom_unless`, `edit_slot`, `Json<T>` handlers, `live_pane`, `Saver::flush` and `pty_size`.
- [x] 1.10 Apply the client and `ship` cleanups: `Watched::new`, `Observer::selection` and `tabs`, one `Sidebar::nudge`, the merged mouse arms, `checked_target` with `Checked`, `env_target` and `first_signal`.
- [x] 1.11 Verify slice 1's checks in `design/program.md`, including the OpenAPI before-and-after diff, `cargo tree -d` and an empty `grep -rn '#\[allow' crates`.

## 2. Server: replica reads, pane reports and new-pane sizing

- [x] 2.1 Before any change, profile GET and commit latency with `SHIP_TRACE` and count replicas per prompt.
- [x] 2.2 Give `ServerState` the `ReplicaSink`, publish inside `commit`, and remove the revision `AtomicU64` and the bus subscription for replicas.
- [x] 2.3 Serve `list_tabs`, `get_tab`, `get_pane`, `list_views` and `get_view`, `input`'s attachment check and `order::lane` from `Replicas::current`, and remove the six read messages.
- [x] 2.4 Replace `PaneChange` with the state-carrying `PaneEvent`, send it only when it differs, and rewrite the state actor's handler.
- [x] 2.5 Add `start_size` and `placeholder` in `state/geometry.rs` and use them in `CreatePane`.
- [x] 2.6 Verify slice 2's checks, including the after-profile, the replica count and the `stty size` matrix on two client sizes.

## 3. Client loop and sidebar

- [ ] 3.1 Return `Option<Item>` from `Keys::handle` and `Local::read`, replace `Clicked` with `Routed` and `SidebarEdit`, and add `Scene::on_screen` and `Scene::relative`.
- [ ] 3.2 Add `ui/nav.rs` with `View`, move the navigation functions into it with `Step` over `CycleDirection` and `Hop::Collapse` and `Hop::Expand`, and give `act`'s Collapse and Expand arms one shared `Local::hop`.
- [ ] 3.3 Add `ui/connection.rs`, move attach, reconnect, cut and the area PUT into it, and add `Local::apply`, `Local::reconcile` and `tolerant`.
- [ ] 3.4 Draw the tab tree from Ship's own rows in `ui/sidebar/tabs.rs`, split `Tabs::fold` into `collapse` and `expand`, and remove `tui-tree-widget`.
- [ ] 3.5 Verify slice 3's checks: the gesture list, the delayed-link ordering, the navigation list, reconnect and area PUTs, and the tab tree side by side.

## 4. Client API

- [ ] 4.1 Replace `request`, `numbered` and `stream` with `call`, `build`, `Prepared` and `exchange`, and settle U-2.
- [ ] 4.2 Add the `Scope` helpers and `scope_tabs`, and rewrite the target resolvers over them.
- [ ] 4.3 Verify that every `ship tab` and `ship pane` subcommand prints the same output and errors as before.

## Deviation log
