# Focus reports: program

Status: Draft

## Summary

One slice, already built: the code came first as a demo on 2026-11-05 and this paper records it. Nine files change; no file or dependency is added.

```diff
 pub struct ViewInput {
     pub area: Size,
+    #[serde(default)]
+    pub focused: bool,
 }
```

`AttachRequest` and `ViewingRecord` gain the same field, `commit` gains `self.apply_focus(&replica, &before)`, and the pane passes `PaneCommand::Focus(bool)` to the session.

## Rationale

The server side copies `apply_sizes`: derive from the published replica, compare with the one before, send only changes. The client side reuses the existing view PUT, which already compares the wanted area with the server's record; focus becomes a second field in the same comparison, so it gets the same retry and one-PUT-at-a-time behavior.

## Skeleton map

### Changed

#### `crates/ship-core/src/protocol.rs`

`focused: bool` on `AttachRequest`, `ViewInput` and `ViewingRecord`, each `#[serde(default)]`. `PROTOCOL_VERSION` 22 → 23.

#### `crates/ship-server/src/state.rs`

```rust
/// Panes selected by a focused attachment. A selected tab focuses nothing,
/// as keys then reach no pane.
fn focused_panes(replica: &Replica) -> HashSet<IdOf<Pane>>;

impl ServerState {
    /// Like sizes, only panes whose focus differs from the `before` replica's
    /// are told. A pane absent from `before` is new, so it's always told.
    fn apply_focus(&self, replica: &Replica, before: &Replica);
}
```

`Attach` stores `request.focused`; `SetView` stores `view.focused`.

#### `crates/ship-server/src/pane.rs`

`PaneCommand::Focus(bool)`, handled as `self.session.send_focus(gained)`. An exited program ignores it, like keys.

#### `vendor/ratatui-ghostty/src/session.rs`

```rust
impl SessionState<'_> {
    /// Whether the program enabled focus events (mode 1004).
    fn reports_focus(&self) -> bool;
    fn write_focus(&self);
}
```

- `SessionCommand::Focus` sets `focused` and writes only if `reports_focus()`. It no longer requests a render.
- After each `vt_write`, a change of `reports_focus()` from off to on calls `write_focus()`.
- The render passes `.focused(true)`, so the widget always draws the cursor.
- `SessionConfig::focused` now means the focus programs are told before any `send_focus`.

#### `crates/ship-client/src/ui/`

- `observer.rs`: `resized` → `changed_view(area, focused)`, returning `Some` when either differs from the record. `remembered` takes `focused` for reattach.
- `connection.rs`: `Resize` → `ViewPut`.
- `attention.rs`: a `focused()` getter for the state it already tracks.
- `mod.rs`: the first attach sends `focused: true`; the loop passes `attention.focused()`.

#### `crates/ship-client/src/automation.rs`

`observe()` attaches with `focused: false`.

#### `docs/site/windows-and-the-server.md`

One sentence in the several-windows paragraph. `docs/site/scripting/writing-a-plugin.md`'s sample health line shows protocol 23.

### Untouched

Selection, repair, geometry, input routing, unseen marks and how the client draws cursors.

## Build order

### 1. Focus reports end to end

Built as above. Verified with a witness program that enables mode 1004 and logs each report, driven over HTTP on an owned server: a scripted demo, a randomized stress run (attach, crash, focus, select pane or tab, split, stack, cycle, close, plus concurrent bursts), a cursor check, and a by-hand run in Ghostty by Cyan.

## Deviation log

- The first demo build let the session's focus feed the widget, which hid the selected pane's cursor while the window was unfocused. Cyan saw it in Ghostty; KD-5 records the fix.
