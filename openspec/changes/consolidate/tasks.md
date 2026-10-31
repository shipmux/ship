# Tasks

The ten groups are `design/program.md`'s build slices. The paper's skeleton map gives the types and signatures, and each slice's "Done when" is its completion evidence; record anything that differs in its deviation log. Slice 10 runs only if open choice O-1 says yes.

Verification uses real commands and a running client, not permanent authored tests. Use an owned temporary server with `XDG_STATE_HOME`, `XDG_CONFIG_HOME` and `XDG_RUNTIME_DIR` pointed at temporary directories, and never a server already running. Every group ends with the paper's regression walk, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo build`, `cargo test --workspace` and a `cloc --include-lang=Rust crates` count against the previous group. Checks run on the implementing machine; the other priority platform is reported unverified.

## 1. Shared shapes

- [ ] 1.1 Add `tree::pane_tab`, `tree::parent` and `tree::panes` in `ship-core`, and `pane_tab_mut` and `pane_mut` in `ship-server/src/state/tree.rs`; replace the hand-written chains.
- [ ] 1.2 Move screen style and color conversions into `ship-core/src/screen.rs`, add `From<Rect> for Size` and `Display for Scrolled`, and use them from `pane.rs`, `ui/panes.rs` and `ui/draw.rs`.
- [ ] 1.3 Add the `Axis` and `Direction` helpers and use them in `state/layout.rs`, `state/geometry.rs`, `ui/mouse.rs`, `ui/mod.rs` and `sidebar/mod.rs`.
- [ ] 1.4 Make `Prefixed` an associated const, rename `Id::new` to `Id::random`, and add `id_map_schema` and `RectSchema`.
- [ ] 1.5 Return `ErrorCode` by value from `AppError::code`, share `data_mut`, and replace `rename::Target` with `NodeId`.
- [ ] 1.6 Verify slice 1's checks in `design/program.md`, including the OpenAPI before-and-after diff and `cargo tree -d`.

## 2. Lints and local cleanups

- [ ] 2.1 Settle open choice O-3, add the four lints to the workspace `Cargo.toml`, and name them in `docs/agents/code.md`.
- [ ] 2.2 Rewrite `err!` without the glob import, add `ServeOptions` with `SavedTabs`, and `Startup`, move `AttachmentGuard` into `Progress`, and replace `runlog.rs`'s `unwrap`s.
- [ ] 2.3 Remove the unused sinks from `relay.rs`.
- [ ] 2.4 Apply the server cleanups: state helpers, `layout::place`, `unzoom_unless`, `edit_slot`, `Json<T>` handlers, `live_pane`, `Saver::flush` and `pty_size`.
- [ ] 2.5 Apply the client and `ship` cleanups: `Watched::new`, `Observer::selection` and `tabs`, one `Sidebar::nudge`, the merged mouse arms, `checked_target` with `Checked`, `env_target` and `first_signal`.
- [ ] 2.6 Verify slice 2's checks, including that `grep -rn '#\[allow' crates` is empty.

## 3. Reads from the published replica

- [ ] 3.1 Give `ServerState` the `ReplicaSink`, publish inside `commit`, and remove the revision `AtomicU64` and the bus subscription for replicas.
- [ ] 3.2 Serve `list_tabs`, `get_tab`, `get_pane`, `list_views` and `get_view`, `input`'s attachment check and `order::lane` from `Replicas::current`, and remove the six read messages.
- [ ] 3.3 Profile GET and commit latency before and after with `SHIP_TRACE`, and verify slice 3's checks.

## 4. Pane reports

- [ ] 4.1 Replace `PaneChange` with the state-carrying `PaneEvent`, send it only when it differs, and rewrite the state actor's handler.
- [ ] 4.2 Count replicas per prompt before and after, and verify slice 4's checks.

## 5. New-pane sizing

- [ ] 5.1 Add `start_size` and `placeholder` in `state/geometry.rs` and use them in `CreatePane`.
- [ ] 5.2 Verify slice 5's `stty size` matrix on two client sizes.

## 6. Input items straight from keys and mouse

- [ ] 6.1 Return `Option<Item>` from `Keys::handle` and `Local::read`, replace `Clicked` with `Routed` and `SidebarEdit`, and add `Scene::on_screen` and `Scene::relative`.
- [ ] 6.2 Verify slice 6's gesture list and the delayed-link ordering check.

## 7. Navigation view

- [ ] 7.1 Add `ui/nav.rs` with `View`, move the navigation functions into it, and give `act`'s Collapse and Expand arms one shared `Local::hop`.
- [ ] 7.2 Verify slice 7's navigation list.

## 8. Connection

- [ ] 8.1 Add `ui/connection.rs`, move attach, reconnect, cut and the area PUT into it, and add `Local::apply`, `Local::reconcile` and `tolerant`.
- [ ] 8.2 Verify slice 8's reconnect and area-PUT checks.

## 9. Client API

- [ ] 9.1 Replace `request`, `numbered` and `stream` with `call`, `build` and `exchange`, and settle U-2.
- [ ] 9.2 Add the `Scope` helpers and `scope_tabs`, and rewrite the target resolvers over them.
- [ ] 9.3 Verify that every `ship tab` and `ship pane` subcommand prints the same output and errors as before.

## 10. Sidebar tree (only if O-1 says yes)

- [ ] 10.1 Draw the tab tree from Ship's own rows in `ui/sidebar/tabs.rs` and remove `tui-tree-widget`.
- [ ] 10.2 Verify slice 10's side-by-side checks.
