# Spec Delta

## ADDED Requirements

### Requirement: Attachment focus
Each attachment SHALL carry `focused`, whether its terminal has focus, in its viewing record. An attach request MAY set it and SHALL default to false. `PUT /attach/view` SHALL set it together with the area. The terminal client SHALL attach with `focused` true and SHALL send each change its terminal reports.

#### Scenario: Window loses focus
- **WHEN** a user switches from a Ship window to another application
- **THEN** `ship view list` shows that window's attachment with `focused` false

#### Scenario: Script attachment
- **WHEN** `ship watch` is running
- **THEN** its attachment has `focused` false

### Requirement: Pane focus reports
A pane SHALL have focus while at least one attachment with `focused` true has that pane selected; a selected tab SHALL give no pane focus. A program that enabled focus reports (DEC mode 1004) SHALL receive `ESC [ I` when its pane gains focus and `ESC [ O` when it loses it, and SHALL receive its pane's current focus when it enables the mode. A program that has not enabled the mode SHALL receive no bytes from focus changes. Focus SHALL NOT change how a pane's cursor is published.

#### Scenario: Switching applications
- **WHEN** a program that enabled focus reports is in the selected pane of the only Ship window, and the user switches away and back
- **THEN** the program receives `ESC [ O` and then `ESC [ I`

#### Scenario: Two windows on one pane
- **WHEN** two focused windows have the same pane selected and one loses focus
- **THEN** the program receives nothing, and receives `ESC [ O` once the other loses focus too

#### Scenario: Moving between panes
- **WHEN** a focused window moves its selection from pane A to pane B, both running programs that enabled focus reports
- **THEN** A's program receives `ESC [ O` and B's receives `ESC [ I`

#### Scenario: Closing the selected pane
- **WHEN** a focused window's selected pane closes and selection repair moves it to another pane
- **THEN** that pane's program receives `ESC [ I`

#### Scenario: Window crashes
- **WHEN** the only focused window on a pane loses its connection without detaching
- **THEN** once the server drops its attachment, the program receives `ESC [ O`

#### Scenario: Enabling reports late
- **WHEN** a program in a pane that has no focused window enables focus reports
- **THEN** it receives `ESC [ O` at once

#### Scenario: Program that didn't ask
- **WHEN** a shell that never enabled focus reports is in a pane that gains and loses focus
- **THEN** it receives no bytes

#### Scenario: Cursor while unfocused
- **WHEN** the window showing a pane loses focus
- **THEN** the pane's published cursor is unchanged
