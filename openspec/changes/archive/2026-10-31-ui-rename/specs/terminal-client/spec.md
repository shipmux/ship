# Spec Delta

## MODIFIED Requirements

### Requirement: Ordered input
A client's bindings, menu items, keys, pastes, `client.send`, clicks, fold clicks, wheel turns, drags and program mouse input SHALL take effect on the server in input order, including `server.` actions after earlier typing. Anything depending on selection, layout or folds SHALL resolve against the state after earlier input takes effect. Keys and pastes SHALL be interpreted through the active mode, menu or rename field at their turn.

#### Scenario: Type right after a new tab
- **WHEN** 500 ms of delay is added each way between client and server, and the user presses `alt-n` then types `echo hi` and Enter at once
- **THEN** `hi` is printed in the new tab's shell, and nothing reaches the pane that was selected before, in ten runs of ten

#### Scenario: Zoom what was just split
- **WHEN** 500 ms of delay is added each way and the user presses `alt-|` then `alt-f` at once
- **THEN** the new pane is the one zoomed, in ten runs of ten

#### Scenario: Type then close
- **WHEN** 500 ms of delay is added each way, and the user types `ls` into a shell pane and presses `alt-shift-x` at once
- **THEN** the server routes both keys to that pane before removing it, and its debug log shows no input dropped for a removed pane, in ten runs of ten

#### Scenario: Idle typing
- **WHEN** nothing is waiting and the user types into a shell on a local server
- **THEN** each key is sent at once and feels no slower than before

#### Scenario: Keys after a mode change
- **WHEN** 500 ms of delay is added each way, and the user presses `alt-n`, then `alt-t` and `j` at once
- **THEN** `j` moves the tab selection in `tabs` mode and nothing reaches a pane, in ten runs of ten

#### Scenario: Typing into a rename field
- **WHEN** 500 ms of delay is added each way, and the user presses `alt-n`, then `alt-p`, `r`, `foo` and Enter at once
- **THEN** the new tab's pane is named `foo`, and nothing reaches its program, in ten runs of ten

## ADDED Requirements

### Requirement: Input while disconnected
While a client is disconnected, it SHALL read keys and pastes as they arrive. Mode changes, sidebar actions and config reloads SHALL apply, detach SHALL detach, and every other action, key and paste SHALL be dropped.

#### Scenario: Mode change while disconnected
- **WHEN** the server is stopped and the client shows `disconnected, reconnecting`, and the user presses `alt-t`
- **THEN** the client enters `tabs` mode

#### Scenario: Detach while disconnected
- **WHEN** the server is stopped and the user presses `alt-q`
- **THEN** the client detaches

### Requirement: Rename field
The rename field SHALL appear on the bottom row of the pane area, in place of the status line, with a pill reading `RENAME TAB` or `RENAME PANE`. It SHALL appear whether or not the sidebar is shown and whether or not the pane has a title row. It SHALL belong to the client that opened it, and other clients SHALL see nothing of it.

#### Scenario: Tab field with the sidebar hidden
- **WHEN** the sidebar is hidden and the user presses `r` in `tabs` mode
- **THEN** the bottom row shows `RENAME TAB` and the field

#### Scenario: Lone pane
- **WHEN** the selected pane is its tab's only pane, so it has no title row, and the user presses `alt-p` then `r`
- **THEN** the bottom row shows `RENAME PANE` and the field

### Requirement: Rename field contents
The field SHALL open holding its target's stored name, with the cursor at the end. With no stored name it SHALL open empty and show, dimmed, the label the target would display without a name. That placeholder SHALL follow the target's title and directory while the field is open.

#### Scenario: Named tab
- **WHEN** tab `api` has the name `api` and the user presses `r` on it in `tabs` mode
- **THEN** the field holds `api` in normal brightness with the cursor after it

#### Scenario: Unnamed tab
- **WHEN** tab `web` has no name and its first pane is in `~/web`, and the user presses `r` on it in `tabs` mode
- **THEN** the field is empty and shows `web` dimmed

#### Scenario: Placeholder follows the title
- **WHEN** the field is open for an unnamed pane and its program changes its title
- **THEN** the placeholder shows the new title

### Requirement: Rename field takes all input
While the field is open, it SHALL take every key, paste and mouse event. No binding SHALL run, and nothing SHALL reach a pane's program. Enter SHALL submit, Esc SHALL cancel, printable keys and pastes SHALL insert text, with pasted line breaks turned into spaces, and Backspace, left, right and `ctrl-u` SHALL edit. A mouse press SHALL cancel.

#### Scenario: Bound keys become text
- **WHEN** the field is open from `panes` mode and the user types `hello`
- **THEN** the field holds `hello`, and the selection and layout don't change

#### Scenario: Multi-line paste
- **WHEN** the field is open and the user pastes `a` and `b` on two lines
- **THEN** the field holds `a b`

#### Scenario: Click cancels
- **WHEN** the field is open and the user clicks a pane
- **THEN** the field closes, nothing is sent, and the click doesn't select or reach that pane

### Requirement: Submitting a rename
Enter SHALL close the field and trim surrounding whitespace. An empty result SHALL clear the target's name; any other result SHALL set it, through `server.tab.set` or `server.pane.set`. Esc SHALL close the field and send nothing. A server error SHALL show on the status line. The mode active when the field opened SHALL still be active after it closes.

#### Scenario: Name a tab
- **WHEN** the field is open for unnamed tab `web`, and the user types `frontend` and presses Enter
- **THEN** the sidebar shows `frontend`, `ship tab get` reports the name `frontend`, and the client is still in `tabs` mode

#### Scenario: Clear a name
- **WHEN** the field holds `api`, and the user presses `ctrl-u` then Enter
- **THEN** `api`'s name is cleared and its label falls back to the automatic one

#### Scenario: Cancel
- **WHEN** the field is open and the user presses Esc
- **THEN** the field closes and `ship tab list` output is byte-for-byte unchanged

#### Scenario: Other clients wait for Enter
- **WHEN** a second client shows the same tab while the first types into its field
- **THEN** the second client sees no change until Enter, then sees the new name

### Requirement: Rename target goes away
If the field's target closes while the field is open, the field SHALL close, send nothing, and show `tab closed` or `pane closed` on the status line. While disconnected, Enter SHALL close the field and send nothing. A terminal resize SHALL keep the field open, and a config reload SHALL close it.

#### Scenario: Pane closes during a rename
- **WHEN** the field is open for a pane and another shell runs `ship pane close` on it
- **THEN** the field closes and the status line shows `pane closed`

#### Scenario: Tab closes during a rename
- **WHEN** the field is open for a tab and another client closes that tab
- **THEN** the field closes and the status line shows `tab closed`

#### Scenario: Enter while disconnected
- **WHEN** the field is open, the server stops, and the user presses Enter
- **THEN** the field closes and nothing is sent after the client reconnects

### Requirement: Rename selection changes
A rename field SHALL keep its captured target. It SHALL cancel without saving when this attachment no longer views the target tab or selects the target pane, including absent selection. Changing panes within the target tab SHALL keep a tab field open. Other attachments navigating SHALL NOT cancel it unless this attachment's selection changes. Automatic selection repair SHALL follow the same rule. Target deletion SHALL retain the Rename target goes away feedback.

#### Scenario: Another tab is selected
- **WHEN** a tab field is open for A and a command selects tab B for this attachment
- **THEN** the field closes without submitting and both names are unchanged

#### Scenario: Another pane is selected
- **WHEN** a pane field is open for A and a command selects pane B for this attachment
- **THEN** the field closes without submitting and both names are unchanged

#### Scenario: Selection is cleared
- **WHEN** a rename field is open and a command clears this attachment's selection
- **THEN** the field closes without submitting

#### Scenario: Same tab, different pane
- **WHEN** a tab field is open and this attachment selects another pane in that tab
- **THEN** the field and its draft stay open for the original tab

#### Scenario: Another attachment navigates
- **WHEN** a rename field is open and another attachment changes its selection without changing this attachment's selection
- **THEN** the field and its draft stay open

#### Scenario: Shared stack front repairs selection
- **WHEN** a pane field is open for A, another attachment brings B to the stack's front, and the server repairs this attachment's selection from A to B
- **THEN** the field closes without submitting and A's name is unchanged

### Requirement: Rename accent
While the field is open, its target's label SHALL take the accent wherever it's drawn: the sidebar row for a tab, and the title row for a pane.

#### Scenario: Accented sidebar row
- **WHEN** the field is open for tab `web` and the sidebar is shown
- **THEN** `web`'s sidebar row is drawn in the accent

#### Scenario: Accented stacked pane
- **WHEN** the field is open for the selected pane in a stack
- **THEN** that pane's label on the stack's title row is drawn in the accent
