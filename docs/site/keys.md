# Keys

<!--
Scope: Every default key, resize, tabs and panes modes, and the mouse, one line each.
Not here: how to rebind (configuration.md), how scrolling and selection behave (scrolling-copying-and-the-mouse.md).
-->

In normal mode, Ship's default keys are Alt chords. Any other key goes to the program in the selected pane. On macOS, your terminal needs [Option set as Alt](README.md#first-run) for them to work.

| Key | Does |
| --- | --- |
| `alt-n` | New tab with a shell |
| `alt-shift-n` | New tab nested under the current one |
| `alt-x` | Close the tab |
| `alt-\|` / `alt--` | Split the pane right / down |
| `alt-s` | Stack a new shell in front of the pane |
| `alt-,` / `alt-.` | Previous / next pane in the stack |
| `alt-e` | Open [scrollback in your editor](panes.md#edit-scrollback) |
| `alt-shift-x` | Close the pane |
| `alt-left` / `alt-right` | Previous / next tab |
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

In resize mode, `h` / `j` / `k` / `l` move the pane's edge, `shift-h` / `shift-l` resize the sidebar and `shift-j` / `shift-k` move the line between tabs and agents.

In tabs mode, `j` / `k` select tabs, `h` folds an open row or selects its parent, and `l` unfolds a folded row or selects its first child. `shift-k` / `shift-j` move a tab up / down among its siblings, while `shift-l` / `shift-h` move it [in / out](tabs.md). `r` [renames the tab](tabs.md#renaming).

In panes mode, `h` / `j` / `k` / `l` select the pane left, below, above or right, and `shift-h` / `shift-j` / `shift-k` / `shift-l` swap with that neighbor. `,` / `.` cycle the stack and `r` [renames the pane](panes.md#renaming). The former `alt-shift-h` / `j` / `k` / `l` chords are no longer bound in normal mode and go to the program instead.

## The mouse

Click a pane, a tab or an agent to select it, scroll with the wheel, drag to select and copy text, drag pane borders and the sidebar's dividers to resize, and drag sidebar rows to move tabs. Right-click for actions, or click `≡` beside `tabs` for the global menu. [Scrolling, copying and the mouse](scrolling-copying-and-the-mouse.md) has the details.

## Changing them

Every key here can be rebound or turned off, and you can add your own modes, like a tmux-style `ctrl-b` prefix. [Configuration](configuration.md#keys-and-modes) shows how.
