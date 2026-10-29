## ADDED Requirements

### Requirement: Edit a pane's scrollback
`ship pane edit-scrollback` SHALL open a pane's full history and screen as plain text, with soft-wrapped lines joined, in the editor named by `VISUAL`, else `EDITOR`, else `vi`. Editor values with arguments, such as `code --wait`, SHALL work. The target SHALL be `--pane X` when given, else the pane directly behind the pane the command runs in. It SHALL have no key binding form of its own; keys run it in a stacked pane. It SHALL be available on Unix only.

#### Scenario: Open the pane behind
- **WHEN** a pane printed `seq 1 5000` and the user presses `alt-e` on it
- **THEN** the editor opens in front of it with all 5000 lines

#### Scenario: Editor with arguments
- **WHEN** `EDITOR` is `code --wait` and `VISUAL` is unset
- **THEN** the file opens in that editor and the command waits for it

#### Scenario: Nothing behind
- **WHEN** a pane alone in its slot runs `ship pane edit-scrollback` with no `--pane`
- **THEN** it fails saying there is no pane behind it, exits 1, and nothing else changes

### Requirement: Scrollback file lifetime
The text SHALL be written to a file in a Ship cache directory that only the user can read. The file SHALL be deleted after the editor exits, and files left by a crashed run SHALL be deleted by the next run, without touching a file a running editor still has open. Edits SHALL be discarded.

#### Scenario: File while open and after
- **WHEN** the editor is open and the user checks the file, then quits the editor
- **THEN** the file is mode 0600 while open and gone after

#### Scenario: Left by a killed run
- **WHEN** one run was killed leaving its file, another run's editor is still open, and a third run starts
- **THEN** the killed run's file is deleted and the open run's file is kept

### Requirement: Edit-scrollback failures
`ship pane edit-scrollback` SHALL exit with a non-zero status and a message when it can't read the scrollback or start the editor. Run from the default `alt-e` binding, its pane SHALL stay open showing the error until the user closes it.

#### Scenario: Editor can't start
- **WHEN** `EDITOR` is `/nonexistent` and the user presses `alt-e`
- **THEN** the stacked pane stays open showing the error, and closing it brings the original pane back
