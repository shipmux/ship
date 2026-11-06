# Focus reports: product

Status: Draft

## Summary

A program that asks for focus reports (DEC mode 1004) hears `\e[I` when it gains focus and `\e[O` when it loses it, as it would running straight in Ghostty:

```text
you alt-tab away from the Ship window   → the selected pane's program gets \e[O
you come back                           → \e[I
you move to another pane in Ship        → old pane \e[O, new pane \e[I
```

Editors use this to reload changed files and stop blinking cursors, and agent CLIs to decide whether to notify. Today Ship's client sees focus changes but uses them only for unseen marks, and no program hears anything.

## Goal

Programs in Ship get the focus reports they asked for, matching what established multiplexers do. Settled in chat on 2026-11-05 and 2026-11-06, after a demo and a stress test.

## User stories

### P1: A program learns it lost focus

A user runs an editor in a pane, then switches to their browser. The editor gets `\e[O` and acts on it. When they switch back, it gets `\e[I`.

### P2: Several windows

A user has two Ship windows on the same pane and switches between them. The pane keeps focus throughout, because one of the windows always has focus, and its program hears nothing. When the user leaves both windows, it gets one `\e[O`.

## Non-goals

- Changing how any cursor is drawn. An unfocused Ship window keeps its cursor, drawn the way the outer terminal draws unfocused cursors. Programs' focus doesn't hide or restyle it.
- Focus for scripts. An observer attachment (`ship watch`, `ship pane wait`) never gives a pane focus.
- Focus reports to programs that didn't ask for them.

## Functional requirements

- **FR-001** A pane has focus while at least one attachment whose terminal has focus has that pane selected. A selected tab gives no pane focus.
- **FR-002** A program that enabled focus reports gets `\e[I` or `\e[O` each time its pane's focus changes, and at no other time except FR-003.
- **FR-003** When a program enables focus reports, it gets its pane's current focus at once, as Ghostty does.
- **FR-004** A program that never enabled focus reports gets no bytes from focus changes.
- **FR-005** Every way focus can change applies: the window's focus, selecting, stack cycling, zoom, closing the selected pane, and a window detaching or crashing.

## Success criteria

- **SC-001** In a real Ghostty window, alt-tabbing and moving between panes give the sequences in the summary.
- **SC-002** A randomized run over attach, crash, focus, select, split, stack, cycle and close, with concurrent bursts, ends with every program's last report matching FR-001, with no repeated reports and no bytes to programs that didn't ask.

## Amendments
