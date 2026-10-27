# Keyboard tab work in tabs mode

Status: Locked on 2026-10-27. Written from a design conversation with Cyan on 2026-10-27 for [#83](https://github.com/shipmux/ship/issues/83), which settled every choice below ("sounds good! go for it!"), then approved in Plannotator review ("LGTM").

## Goal

Without a mouse, you can walk the tab tree and fold rows, but you can't move a tab or jump between a parent and its children. In tabs mode, `h` and `l` do nothing on a row that's already folded or open, or on a tab with no children, and reordering needs a mouse drag or `ship tab move` with IDs. This change gives tabs mode the tree keys file trees and outliners already use, so tabs can be navigated, reordered and nested from the keyboard. It's part of milestone 5, getting Ship ready to share ([#42](https://github.com/shipmux/ship/issues/42)).

```diff
 tabs mode
   j / k        next / previous visible row          (unchanged)
-  h            fold the row
+  h            fold an open row; on a folded row or a leaf, select the parent
-  l            unfold the row
+  l            unfold a folded row; on an open row, select its first child
+  shift-k / j  move the tab up / down among its siblings
+  shift-l      move it in: last child of the sibling above
+  shift-h      move it out: just after its parent
```

The conventions behind this:

- `h`/`l` follow the file-tree convention (VS Code's explorer, Windows Explorer, the WAI-ARIA treeview pattern), in the vim-key form yazi, ranger and Neovim tree plugins use.
- Moving one step up or down matches every multiplexer that binds tab reordering (Zellij, kitty, WezTerm) and outliners such as Org and Logseq.
- No multiplexer nests tabs, so `in` and `out` follow outliners. `in` is the same in all of them. `out` follows Workflowy and Logseq's logical outdenting rather than Org, which leaves the item in place and adopts the siblings below it.

## User stories

### P1: Move a tab with the keyboard

Tree used below:

```text
▾ api
    server
    tests
▸ web
  notes
```

- **Given** `notes` is selected in tabs mode, **when** the user presses `shift-k`, **then** the order at the top level is `api`, `notes`, `web`, and `notes` stays selected.
- **Given** `tests` is selected, **when** the user presses `shift-l`, **then** `tests` becomes `server`'s last child, carrying its panes, and its programs keep running.
- **Given** `notes` is selected and `web` is folded, **when** the user presses `shift-l`, **then** `notes` becomes `web`'s last child, `web` unfolds, and `notes`'s row is visible and selected.
- **Given** `server` is selected, **when** the user presses `shift-h`, **then** `server` sits at the top level right after `api`, and `tests` stays under `api`.
- **Given** a pane in `tests` is selected, **when** the user presses `shift-k`, **then** `tests` moves above `server`, and the same pane stays selected.

Tested independently by moving tabs in a running client and reading the tree back with `ship tab list`.

### P2: Nothing happens at an edge

- **Given** `api` is selected, **when** the user presses `shift-k` (first sibling), `shift-h` (top level) or `shift-l` (no sibling above), **then** the tree is unchanged and no error is shown.
- **Given** `notes` is selected, **when** the user presses `shift-j` (last sibling), **then** the tree is unchanged.

Tested independently by pressing each move on a tab at that edge.

### P3: Hop between parent and children

- **Given** `server` is selected, **when** the user presses `h`, **then** `api` is selected and its landing selection is used, as for `j`/`k`.
- **Given** `api` is selected and open, **when** the user presses `h`, **then** `api` folds and stays selected, as today.
- **Given** `api` is folded, **when** the user presses `h`, **then** nothing changes, since `api` is at the top level.
- **Given** `web` is selected and open, **when** the user presses `l`, **then** `web`'s first child is selected.
- **Given** `web` is folded, **when** the user presses `l`, **then** `web` unfolds and stays selected, as today.
- **Given** `notes` is selected, **when** the user presses `l`, **then** nothing changes, since it has no children.

Tested independently by walking the tree above with `h` and `l` in a running client.

## Non-goals

- Bindings in normal mode. `alt-shift-hjkl` is pane swap, and tabs mode is one `alt-g` away. Users can bind `client.tab.move` themselves.
- Wrapping at edges. A tab never jumps from one end of its sibling list, or of the tree, to the other.
- Moving more than one step per press, or to an absolute position. `ship tab move` and the mouse cover those.
- Org-style outdenting, where the siblings below a moved-out tab become its children.
- Any server change. Moves use the existing `server.tab.move`.
- A CLI form of `client.tab.move`. `ship tab move` keeps taking explicit IDs.
- New tab menu items for moving.

## Functional requirements

### Navigation

- **FR-001** `client.tab.collapse` MUST fold the selected tab's row when it has children and is open. On a row that's folded or has no children, it MUST select the tab's parent, using the same landing selection as `client.tab.next`. On a top-level tab that's folded or has no children, it MUST do nothing.
- **FR-002** `client.tab.expand` MUST unfold the selected tab's row when it has children and is folded. On an open row with children, it MUST select the tab's first child, using the same landing selection. On a tab with no children, it MUST do nothing.
- **FR-003** FR-001 and FR-002 replace the rule that folding never changes the selected tab to a child. Folding itself still never changes selection; only the fallbacks above do.

### Moves

- **FR-004** `client.tab.move` MUST take a required `direction` of `"up"`, `"down"`, `"in"` or `"out"`, written `client.tab.move.direction = "up"` like `client.pane.swap.direction`.
- **FR-005** It MUST move the selected tab, or the tab of the selected pane, carrying its subtree and panes, with one tab move:
  - `up`: to just before its previous sibling.
  - `down`: to just after its next sibling.
  - `in`: to the last child of its previous sibling.
  - `out`: to just after its parent, under its grandparent or at the top level. Its siblings stay where they are.
- **FR-006** At an edge it MUST send no move and show no error: `up` on a first sibling, `down` on a last sibling, `in` with no previous sibling, `out` on a top-level tab, or no selection.
- **FR-007** Siblings MUST be counted whether or not their rows are visible. Folding changes what `j`/`k` walk, not what a move steps over.
- **FR-008** The move MUST be resolved against the tree as it is at its turn, as mouse drops are. Pressing `shift-k` three times moves the tab up three places.
- **FR-009** `in` into a folded sibling MUST unfold it in the moving client, as a mouse drop into a folded tab does. If the move then fails, that sibling stays unfolded.
- **FR-010** The moved tab, or the pane that was selected in it, MUST stay selected.

### Defaults and docs

- **FR-011** Tabs mode MUST bind `shift-k`, `shift-j`, `shift-l` and `shift-h` to `up`, `down`, `in` and `out`, with status-line labels `move up`, `move down`, `move in` and `move out`. `h` and `l` keep their bindings to `collapse` and `expand`.
- **FR-012** `docs/site/keys.md` and `docs/site/tabs.md` MUST describe the new keys.

## Success criteria

- **SC-001** Starting from a flat list of five tabs and using only tabs-mode keys, a user can reorder them, nest two levels deep and un-nest them again, without the mouse or the CLI.
- **SC-002** Every case in P2 leaves `ship tab list` output byte-for-byte unchanged.
- **SC-003** Everywhere `h` and `l` did something before this change, they still do the same thing.
- **SC-004** A tab moved from the keyboard and the same move made by mouse drop produce the same tree.

## Open unknowns

None.
