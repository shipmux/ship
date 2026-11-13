# Keys

<!--
Scope: Every default key, resize, tabs and panes modes, and the mouse, one row or line each.
Not here: how to rebind (configuration.md), how scrolling and selection behave (scrolling-copying-and-the-mouse.md).
-->

In normal mode, Ship's default keys are Alt chords, and any other key goes to the program in the selected pane. On macOS, your terminal needs [Option set as Alt](README.md#first-run) for them to work.

| Key | Does |
| --- | --- |
| `alt-n` | New tab with a shell |
| `alt-shift-n` | New tab nested under the current one |
| `alt-shift-x` | Close the tab |
| `alt-\|` / `alt--` | Split the pane right / down |
| `alt-s` | Stack a new shell in front of the pane |
| `alt-,` / `alt-.` | Previous / next pane in the stack |
| `alt-e` | Open [scrollback in your editor](panes.md#edit-scrollback) |
| `alt-x` | Close the pane |
| `alt-up` / `alt-down` | Previous / next tab |
| `alt-left` / `alt-right` | Fold the tab or go to its parent / unfold it or go to its first child |
| `alt-h` / `alt-j` / `alt-k` / `alt-l` | Move to the pane left, below, above or right |
| `alt-tab` | Next pane in the tab |
| `alt-a` | Jump to the next agent that needs you |
| `alt-p` | Panes mode (below) |
| `alt-f` | Zoom the pane to fill the tab, or unzoom |
| `alt-r` | Resize mode (below) |
| `alt-t` | Tabs mode (below) |
| `alt-b` | Hide or show the sidebar |
| `alt-pageup` / `alt-pagedown` | Scroll the pane up / down a page |
| `alt-q` | Detach |

## Resize, tabs and panes modes

These modes stay active until you press `esc`.

In resize mode:

| Key | Does |
| --- | --- |
| `h` / `j` / `k` / `l` or arrows | Move the pane's edge |
| `shift-h` / `shift-l` | Resize the sidebar |
| `shift-j` / `shift-k` | Move the line between tabs and agents |

In tabs mode:

| Key | Does |
| --- | --- |
| `j` / `k` or `down` / `up` | Select the next / previous tab |
| `h` or `left` | Fold an open row, or select its parent |
| `l` or `right` | Unfold a folded row, or select its first child |
| `shift-k` / `shift-j` | Move the tab up / down among its siblings |
| `shift-l` / `shift-h` | Move the tab [in / out](tabs.md) |
| `r` | [Rename the tab](tabs.md#renaming) |
| `n` / `shift-n` | New tab after this one / nested under it |
| `x` | Close the tab and everything in it |

In panes mode:

| Key | Does |
| --- | --- |
| `h` / `j` / `k` / `l` or arrows | Select the pane left, below, above or right |
| `shift-h` / `shift-j` / `shift-k` / `shift-l` | Swap with that neighbor |
| `,` / `.` | Cycle the stack |
| `r` | [Rename the pane](panes.md#renaming) |
| `n` | Split a new pane to the right |
| `x` | Close the pane |

## The mouse

Click a pane, a tab or an agent to select it, scroll with the wheel, drag to select and copy text, drag pane borders and the sidebar's dividers to resize, and drag sidebar rows to move tabs. Right-click for actions, or click `≡` beside `tabs` for the global menu. [Scrolling, copying and the mouse](scrolling-copying-and-the-mouse.md) has the details.

## Changing them

Every key here can be rebound or turned off, and you can add your own modes, like a tmux-style `ctrl-b` prefix. [Configuration](configuration.md#keys-and-modes) shows how.
