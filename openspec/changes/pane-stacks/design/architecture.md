# Stacked panes: architecture

Status: Locked on 2026-10-29. Built on the locked `product.md`. Cyan picked shape A in chat ("i agree with a. soudns good to me"), with the deciding outcome that a pane in two places or an empty stack can't be represented, and accepted the follow-on decisions and the `SHIP_BIN` launch in the same message. `place` replaced the `--stack` flag during review, and the mitsein spike findings were folded into decisions 1 and 8 and the risks. Then approved in Plannotator review on the updated version ("LGTM! this is on the new one"). Reopened on 2026-10-29 under the ripple rule when FR-007 changed to `--direction`, and to make the move body reuse create's `at` and `place` in decision 6, following `docs/agents/code.md` ("Reuse a named shape instead of restating it"; "go for it"). Re-locked on 2026-10-29 with the program paper ("lock it!"). Reopened with the product paper and re-approved in chat on 2026-10-29 ("yes please. g for it") to remove backward-compatibility requirements before Ship's first release. Earlier legacy-format and creation-alias decisions are superseded.

Diagrams are code and diffs instead of Mermaid, at Cyan's request.

## Context and constraints

- **Pane membership is stored once, as layout leaves.** `ship-core/src/layout.rs` has `enum Layout { Pane(Pane), Split(Split) }`, serialized with `#[serde(tag = "kind")]`. `Tab::panes()` iterates the layout and there is no parent index. `tree.rs` finds panes and owners by traversal.
- **Size lives only in split ratios.** `state/geometry.rs` walks the layout into `TabGeometry { panes: IndexMap<IdOf<Pane>, PaneGeometry> }`, adding a border only when the root is a split. A zoomed tab's geometry holds just the zoomed pane. `apply_sizes` (`state.rs`) resizes only panes with a geometry entry, so unviewed and zoom-hidden panes already keep their size.
- **Layout edits live in `state/layout.rs`.** `split`, `close`, `swap`, `resize` and `successor`, and every one of them sets `tab.zoomed = None`.
- **`commit` repairs selections after every edit.** `repair` and `repair_removed` (`state.rs`) move a selection off removed panes and onto the zoomed pane. `retain_runtimes` then drops runtimes whose pane left the tree, which kills their programs.
- **The select handler unzooms** when the selected pane isn't the zoomed one (`state.rs`, `impl Message<Select>`).
- **Saved tabs use the wire serde.** `saver.rs` writes `Tabs` as JSON with live fields blanked, and a file that fails to parse is left in place and ignored. `PROTOCOL_VERSION` is 19.
- **Restore walks `tab.panes()`** and restarts each as a shell. A pane that can't start is removed with `layout::close`.
- **The client draws labels in the top border** for every pane in the geometry (`ui/panes.rs`, using `draw::pane_label`).
- **`CreatePane { at, direction, pane, select }`** is the create body. The CLI's `pane::Create` maps onto it in `ship-client/src/execute.rs`, and key bindings deserialize the same command struct.
- **`pane read --lines N --unwrap`** already reads history with soft-wrapped lines joined. Client-side commands such as `pane wait` live in `ship-client/src/automation.rs`.
- **Every pane gets `SHIP_BIN`, `SHIP_PANE_ID` and `SHIP_SERVER_URL`** (`pane.rs`). `tempfile` and `etcetera` are already workspace dependencies.

## Candidate shapes

### A. A slot is a layout leaf holding the stack (winner)

```diff
 enum Layout {
-    Pane(Pane),
+    Slot(Slot),
     Split(Split),
 }
+struct Slot { panes: VecDeque1<Pane> }   // mitsein; index 0 is the front
```

Every pane sits in exactly one slot, and a slot can't be empty because its type can't hold zero panes. Geometry emits the front pane of each slot, so sizing and `apply_sizes` work without a special case. The cost is touching every `Layout::Pane` match: about ten in `state/layout.rs`, plus geometry, `ship-core/layout.rs` and the client's border drawing.

### B. Keep the leaf, add a side table of the panes behind

```diff
 struct Tab {
     layout: Option<Layout>,                     // Layout::Pane(front)
+    behind: HashMap<IdOf<Pane>, Vec<Pane>>,      // keyed by the front pane
 }
```

Fewer lines change today, but the table is keyed by the front pane's ID. Every close, swap, move and cycle has to re-key it, and any path that forgets leaves panes running with no slot. Zellij's `suppressed_panes` has this shape and this bug class (`zellij-server/src/tab/mod.rs:8184`).

### C. The front pane owns the panes behind it

```diff
 struct Pane {
     ...
+    behind: Vec<Pane>,
 }
```

This is A's data hidden inside a pane. Cycling hands the rest of the list to the new front, closing the front moves the list to a successor, and every `Pane` on the wire, in `ship pane get` and in the agents sidebar, carries a nested list.

## Decision

Shape A. What matters most is that a pane in two places, an orphaned pane and an empty stack can't be represented at all. A gets that from its types, while B and C get it only from every edit path remembering to do the right thing.

## Structure

```text
ship-core
├── layout.rs     Layout { Slot, Split }, Slot { panes: VecDeque1<Pane> }
│                 owns: membership, stack order, front = index 0
├── protocol.rs   CreatePane.place, CyclePane, MovePane; PROTOCOL_VERSION 20
└── command.rs    pane create --place, pane cycle, pane move, pane edit-scrollback

ship-server
├── state/layout.rs    the only code that edits slots:
│                      stack, cycle, rotate_to_front, take, split, close, swap, move
│                      owns: keeping tab.zoomed on the zoomed slot's front
├── state/geometry.rs  one PaneGeometry per slot, for its front pane
│                      owns: the title row of a borderless slot with >1 panes
├── state.rs           commit → repair (selection) → retain_runtimes → apply_sizes
│                      owns: "selection never rests behind the front"
└── saver.rs           unchanged; saves and restores the slot format

ship-client
├── execute.rs     maps the new commands to requests
├── automation.rs  edit-scrollback: find target, read, tempfile, editor
└── ui/panes.rs    border titles for every pane in the slot, click to select

ship-core/defaults.toml   alt-s, alt-[, alt-], alt-e, pane menu items
```

What flows where:

```text
key / CLI ──▶ client execute ──▶ request ──▶ ServerState::commit
                                               edit (state/layout.rs)
                                               repair selections
                                               retain_runtimes      (moved panes keep their ID, so survive)
                                               publish replica ──▶ clients draw slots
                                               apply_sizes          (front panes only)
```

## Key decisions

1. **The slot's stack is `mitsein::vec_deque1::VecDeque1<Pane>` (`mitsein = { version = "0.9", features = ["serde"] }`).** A spike on 2026-10-28 confirmed the operations: `from_one` builds a slot, `push_front` stacks, `rotate_left(1)`/`rotate_right(1)` cycle, and `front()` is the shown pane. Rotate-to-front has no direct method: find the index with `iter1().into_iter().position(..)`, then call `rotate_left(index)`, which keeps the others' order. Removal is `remove_if_many(index).or_get(..)`, returning `Some(Ok(pane))` when others remain and `Some(Err(&only))` when it's the last pane and nothing was removed. That `Err` branch is where the slot gets removed. Its serde feature rejects an empty list ("failed to construct non-empty collection: no items"). Rejected: a hand-written non-empty deque (owned mechanism, against "rent the mechanisms"), and `VecDeque<Pane>` with a runtime check (the invariant moves back into code).
2. **The front is index 0, and slots have no IDs.** A slot is named by any pane in it, like splits are named by nothing. Rejected: slot IDs, which add a second identity to keep in step with membership for no product need.
3. **Zoom stays `Option<IdOf<Pane>>`, always naming the zoomed slot's front.** Each edit in `state/layout.rs` that changes a front (stack, cycle, rotate-to-front, removal from a slot that survives) points `zoomed` at the new front. Edits that today clear zoom keep clearing it, except close and move-away from other slots (FR-015). Rejected: letting `zoomed` name any pane in the slot, which would change its meaning for clients (the sidebar's `Z`, `ship tab get`), and a separate zoomed-slot reference, which slots without IDs can't have.
4. **Selection repair gains one rule, beside the zoom rule:** a selection on a pane behind its slot's front moves to that front (FR-009). The select handler rotates a behind pane to the front before repair runs, so an explicit select wins (FR-008). The handler's unzoom check compares slots instead of pane IDs (FR-011). Rejected: rotating in repair, which would let any edit that pushes a pane behind be undone by the next commit.
5. **`CreatePane.direction` becomes `place`, beside the unchanged `at`.** On the wire it's `enum Place { Split(SplitDirection), Stack }`, defaulting to split right, so a stack can't carry a direction. The CLI and bindings share one flat field, `place = right | down | stack`, that maps onto it. Bindings write `server.pane.create = { place = "stack" }`. Rejected: a `--stack` bool beside `--direction`, which is two fields that can contradict each other; the nested `place = { split = "right" }` in config, which is clunkier and no safer; and a separate stack-create request, which would duplicate `NewPane` handling and selection.
6. **Cycle and move are new requests.** `CyclePane { pane, direction: next | prev }` and `MovePane { pane, at: Y, place: Place }`, reusing create's `Place`: `--onto Y` is `place: stack`, and `--beside Y --direction right` is `place: split(right)`. Move is one commit that edits both tabs, so the pane never leaves the tree and `retain_runtimes` keeps its program. Rejected: building move from close plus create on the client, which kills the program, and from two requests, which leaves a window where the pane is nowhere.
7. **Slot removal reuses close's collapse.** Taking the last pane out of a slot collapses its split the way `close` does today, through one shared helper used by close and move.
8. **Wire and saved format:** a slot serializes as `{"kind": "slot", "panes": [...]}`, front first. `Layout` derives serde directly with only `Slot` and `Split` variants; no legacy leaf adapter or migration is needed before the first release. `PROTOCOL_VERSION` goes to 20 because the layout shape changes. The `ToSchema` for `Slot.panes` uses `#[schema(value_type = Vec<Pane>)]`, which comes out as an array of `Pane`. It's a little imprecise because it doesn't say "non-empty".
9. **Geometry emits only the front pane of each slot.** Panes behind have no geometry entry, so `apply_sizes` leaves them alone and resizes them when they come to the front (FR-002). A slot is bordered when the root is a split, as today. A borderless slot with more than one pane keeps a one-row title strip at the top of its frame (FR-029). Zoom uses the same rule for the zoomed slot.
10. **The client draws stacks from the layout.** For each geometry entry, it looks up the slot holding that pane and draws every pane's label in the top border, front first and marked, and remembers each label's cells for clicks. A click sends a normal select for that pane. Rejected: titles in geometry, which would put drawing data on the server.
11. **Edit-scrollback is a client command in `automation.rs`.** It resolves the target (`--pane`, else the pane behind `SHIP_PANE_ID` in its slot) from `ship tab get`'s layout. It reads with the existing pane read, all lines and `unwrap`, and writes a 0600 `tempfile` in a Ship directory under `etcetera`'s cache dir, sweeping old files first. It runs `sh -c '<editor> "$1"' sh <file>` with `VISUAL`, `EDITOR`, then `vi`, and deletes the file after. The server learns nothing about it. Rejected: a server action, which would put editor knowledge and a tempfile on the server.
12. **The `alt-e` binding launches through `SHIP_BIN`:** `command = ["sh", "-c", "exec \"$SHIP_BIN\" pane edit-scrollback"]` with `exit_behavior = "keep_failed"`. Edit-scrollback is Unix-only to begin with because the editor launch needs `sh` anyway. Rejected: `["ship", ...]`, which depends on the server's inherited `PATH`, and making the server rewrite `ship` to its own binary, which is a hidden special case.
13. **Restore needs no new code path.** `tab.panes()` yields every pane in every slot, so restore starts each one. A pane that fails to start is removed from its slot through the same helper as close, and an emptied slot collapses.

## Risks and unknowns

- **mitsein is 0.9.** Upgrades may change the API. It's used in one type with a few methods, so a break stays local to `layout.rs` and `state/layout.rs`. Its `rust-version` is 1.88, and the workspace toolchain (1.98) builds it. It adds one crate to the compiled graph. The lockfile gains `heapless`, `hash32` and `byteorder` as lock-only entries, and `cargo deny check` passes. The spike ran on macOS only, so Linux is unverified, and cargo-about's release license check wasn't run.
- **Two mitsein sharp edges.** `iter1()` returns an `Iterator1`, which isn't an `Iterator` until `.into_iter()`. A bare `remove_if_many(i);` statement still removes when there are several panes, so every call has to consume the result with `or_get` or similar, or the last-pane case is silently ignored. The program paper should route removal through one helper.
- **"All lines" for pane read.** `--lines` takes a `u32` count, and edit-scrollback will pass the pane's whole history by asking for `u32::MAX`. If the server caps or slows on that, the program paper adds an explicit "all" form.
- **Scripts reading `.layout` break.** The leaf changes from `"pane"` to `"slot"` with a `panes` list. The docs example `ship tab get | jq .layout` still works, but any script matching on `kind == "pane"` needs updating. Ship has no stability promise yet, and the protocol bump makes the client and server mismatch loud.
- **Windows.** Edit-scrollback needs `sh`, so it fails there with an error kept on screen by `keep_failed`. Everything else is platform-independent. Linux is unverified until the build runs there.
- **Selection churn from CLI stacking.** Per FR-009, `ship pane create --place stack` without `--select` still moves selections that were on the old front to the new pane. That follows the product paper, but if it feels wrong in use, it's a product revisit.
- **Click targets for titles.** A border shorter than its titles truncates them, and truncated panes can't be clicked. That's acceptable for a first version; cycling still reaches them.
