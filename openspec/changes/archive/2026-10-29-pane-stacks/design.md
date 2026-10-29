# Design

Summary of the three locked papers in `design/`, which are the source of truth. Where this file and a paper disagree, the paper wins. Cyan locked all three on 2026-10-29 and approved removing backward compatibility before the first release later that day. Locking does not start implementation: each slice runs on Cyan's request and stops for review before the next.

- [`design/product.md`](design/product.md): what and why. User stories P1 to P5, FR-001 to FR-032, success criteria SC-001 to SC-005, no open unknowns.
- [`design/architecture.md`](design/architecture.md): shape A, a slot as a layout leaf holding a non-empty `VecDeque1<Pane>`, and key decisions 1 to 13.
- [`design/program.md`](design/program.md): the touched files and signatures, slices 1 to 5, two blocking unknowns, six open choices, and the deviation log.

## Context

A layout leaf holds exactly one pane, so opening something next to a pane means splitting it. Editing a pane's scrollback ([#85](https://github.com/shipmux/ship/issues/85)) wants the editor in the same place and at the same size as the pane. See the product paper's Goal.

## Goals / Non-Goals

**Goals:** see the product paper's user stories P1 (stack and cycle panes), P2 (edit a pane's scrollback), P3 (zoom a stack), P4 (move panes between stacks and splits) and P5 (stacks survive a restart).

**Non-goals:** see the product paper's Non-goals.

## Decisions

1. **A slot is a layout leaf holding the stack** (architecture shape A). `Layout::Slot(Slot)` replaces `Layout::Pane`, and `Slot` wraps `mitsein`'s `VecDeque1<Pane>`, so an empty stack or a pane in two places can't be represented (decision 1). The front is index 0, and slots have no IDs (decision 2).
2. **Slot edits are `Slot` methods in ship-core**, which own every mitsein call. `Slot::take` returns a named `Taken` (`Pane { pane, rest }` or `Last(pane)`), and `state/layout.rs`'s `take` is the one removal path, replacing `close` (program Rationale, decision 7).
3. **Zoom names the zoomed slot's front.** Stack, cycle, bring-to-front and removal from a surviving slot keep zoom and retarget it. Split, swap, resize and removing the zoomed slot clear it (decision 3). A move also unzooms its destination when needed to keep a selected moved pane visible (FR-020).
4. **Selection repair gains a front rule**, and the select handler brings a behind pane forward first, so an explicit select wins (decision 4). Attach/reconnect hints instead follow the current front and zoom. A selecting create unzooms when needed to keep its new pane visible.
5. **Create takes `place`.** On the wire, `Place` is `Split(SplitDirection)` or `Stack`. The CLI and bindings take a flat `place = right | down | stack` (decision 5).
6. **Cycle and move are new requests.** `CyclePane { direction }` and `MovePane { at, place }`, which reuses create's `Place`. A move is one commit across both tabs, so the program keeps running (decision 6).
7. **Layout derives serde directly** for slot and split nodes, and the protocol goes to 20. No legacy leaf adapter is needed (decision 8).
8. **Geometry has fronts only**, plus a title row for a borderless stacked slot. The client draws titles from the layout (decisions 9 and 10).
9. **Edit-scrollback is a client command**, launched by `alt-e` through `SHIP_BIN` in a stacked pane with `keep_failed`. It's Unix-only to begin with (decisions 11 and 12).

## Implementation as vertical slices

Each slice runs from a real command or key to observable output. Files and checks for each are in the program paper's Build order.

1. **Slot leaf, no behavior change.** `ship tab get` shows `"kind": "slot"` leaves, and slot-format saved tabs restore with every pane in order.
2. **Stack, cycle, selection and zoom on the server and CLI.** `ship pane create --place stack`, `ship pane cycle`, the front repair rule, zoom retargeting, and `alt-s`, `alt-.` and `alt-,`.
3. **Titles and clicks in the client.** Borders list the stack, a title click selects, `alt-tab` walks fronts.
4. **Move.** `ship pane move --onto` and `--beside`, within and across tabs.
5. **Edit-scrollback.** `ship pane edit-scrollback`, `alt-e` and the menu item.

## Risks / Trade-offs

- [Legacy `alt-[` encoding collides with the start of an escape sequence] → Resolved by Cyan's approval of `alt-,` (previous) and `alt-.` (next). Both replacement bindings passed a current-client PTY probe.
- [mitsein may have no call that hands back the only pane by value] → Settle before slice 1. The fallback converts to `VecDeque` inside `Slot::take` (Blocking unknowns 2).
- [The schema says `panes` is an array without saying it's non-empty] → Accepted imprecision (decision 8).
- [Stacking onto a lone pane adds a title row and costs it one row] → The front reflows by one row. Accepted by FR-029.
- [Linux] → Checks run on macOS. Linux is reported unverified unless run there.

## Migration Plan

No backward-compatibility migration is required before Ship's first release. Saved tabs use only the slot format; pane creation uses `place` in request bodies, CLI flags and config. Client and server come from the same build, and protocol 20 refuses a mismatch with a clear error.

## Open Questions

The program paper's six open choices, each with a recommendation that review accepted: `alt-tab` walks fronts only, tab landing goes to the slot's front, `tab set --zoomed` on a behind pane brings it forward, a behind pane's title wins over a border grab, slot edits are `Slot` methods, and title cells are recomputed.
