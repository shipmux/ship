# Scrolling, copying and the mouse

<!--
Scope: Scrolling, selecting and copying, clicking and dragging, and right-click menus.
Not here: changing the menus (configuration.md).
-->

## Scrolling

`alt-pageup` and `alt-pagedown` scroll a page at a time, and the mouse wheel scrolls whatever pane it's over. An `[offset/history]` marker shows how far up you are. New output doesn't pull you back down, but typing does. Each pane keeps about 10 MB of history.

Full-screen programs like Neovim and `less` have no scrollback in Ship, because they manage their own screen.

Press `alt-e` to [open scrollback in your editor](panes.md#edit-scrollback), where you can search and copy with the keys you already know, since Ship doesn't have built-in search or a keyboard copy mode.

From a script, read the history and search it with your usual tools:

```sh
ship pane read --pane "$PANE" --lines 2000 | grep error
```

## Selecting and copying

Drag to select text and release to copy. Double-click selects a word and triple-click a line. Dragging past the edge of a pane scrolls it.

Copying uses your terminal's clipboard support (OSC 52), which most modern terminals have and macOS Terminal.app doesn't. When a program copies on its own, like Neovim's `"+y`, that reaches your clipboard too.

Programs that use the mouse themselves, like Neovim with `mouse=a`, get clicks and drags instead. To use your terminal's own selection, hold its override key (usually Shift), or turn Ship's mouse handling off with `mouse = false` in the [config](configuration.md#client-settings).

## Clicking and dragging

- Click a pane to select it, or a tab or agent in the sidebar to jump there.
- Click the version at the bottom of the sidebar to see what's new and update. It turns `v0.1.0 → v0.2.0` when a release is out.
- Drag the border between panes to resize them.
- Drag the sidebar's right edge to make it wider or narrower. Drag the `agents` line to change how much room tabs and agents get.
- Drag a tab in the sidebar to move it. Move left or right while dragging to choose how deeply it nests, and an underline shows where it will go.

## Right-click menus

Right-click a pane, a tab or the empty sidebar space for a menu of actions, or click `≡` beside `tabs`. Pick with the mouse, or with the arrow keys and Enter, and `esc` closes it.

| Menu | Items |
| --- | --- |
| Pane | Split right, split down, stack, edit scrollback, zoom, right-click passthrough, rename, close |
| Tab | New tab, new child tab, rename, close |
| Global | New tab, hide sidebar, updates, reload config, detach |

Some programs want right-clicks for themselves. Turn on **right-click passthrough** for that pane, from its menu or with `ship pane set --right-click-passthrough true`, and right-clicks inside go to the program whenever it's using the mouse. Its label and border still open Ship's menu. You can change what's in each menu in the [config](configuration.md#menus).
