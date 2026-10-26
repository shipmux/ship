# Spec Delta

## MODIFIED Requirements

### Requirement: Mode key hints
Outside normal mode the status line SHALL show the active mode's name and key hints for that mode: each bound chord in the config file's order, labeled by its binding's `label` when set, else by a word from its action: the mode's name, the direction, the key sent, else the action's name. Hints that don't fit SHALL be cut from the end.

#### Scenario: Active mode
- **WHEN** the user presses `alt-r`
- **THEN** the status line appears showing `resize` with `h left  j down  k up  l right  shift-h sidebar left  shift-l sidebar right  esc normal`, and disappears after `esc`

#### Scenario: Custom binding hint
- **WHEN** the file binds `"f" = { action = { client.pane.zoom = { zoomed = "toggle" } } }` in `resize` mode and the user enters it
- **THEN** the hints include `f zoom`

#### Scenario: Set label
- **WHEN** the file binds `"f" = { action = { client.pane.zoom = { zoomed = "toggle" } }, label = "fill" }` in `resize` mode and the user enters it
- **THEN** the hints include `f fill` and not `f zoom`

### Requirement: Derived labels
An unnamed pane's label SHALL be its program's current title, else its command. An unnamed tab's label SHALL be the last component of its first own pane's current directory in layout order, `~` for the home directory, else that pane's label, else its 1-based position among its siblings. Labels SHALL update as titles and current directories change and SHALL return to the derived form when a name is cleared.

#### Scenario: Label follows the title
- **WHEN** an unnamed pane runs a shell and the user starts Neovim in it
- **THEN** the pane's label changes to Neovim's title, and changes again when Neovim exits and resets it

#### Scenario: Clear a tab name
- **WHEN** a user clears a named tab's name with `ship tab set --tab <id> --name`, with no value
- **THEN** the tab's label becomes its first own pane's current directory's last component, else that pane's label

#### Scenario: Empty name
- **WHEN** a user runs `ship tab set --tab <id> --name ""`
- **THEN** the tab's sidebar row shows an empty label, not a derived one

#### Scenario: Tab label follows the directory
- **WHEN** an unnamed tab's first pane runs a shell that reports its directory and the user runs `cd ~/Documents/projects/ship`
- **THEN** the tab's sidebar row reads `ship`, reads `~` after `cd ~`, and shows the pane's title while the pane has no current directory

#### Scenario: Swap changes first pane
- **WHEN** a swap places a different pane first in an unnamed tab's layout
- **THEN** the tab's derived label follows that pane without changing either pane's identity
