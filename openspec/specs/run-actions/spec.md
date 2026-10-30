# run-actions Specification

## Purpose

Let key bindings and menu entries start any program, on the server's machine with `server.run` or on the client's machine with `client.run`, so scripts can extend Ship without plugins.

## Requirements

### Requirement: Run actions in the config
`server.run` and `client.run` SHALL be accepted wherever bindings and menu entries take an action, each as a table with one required field, `command`. `command` SHALL be a non-empty array of strings. An empty array, a string, a missing `command` or an unknown field SHALL be rejected when the config loads.

#### Scenario: Valid run binding
- **WHEN** the file binds `"alt-g" = { action = { client.run = { command = ["gh", "pr", "view", "--web"] } } }` and the user runs `ship config check`
- **THEN** the check passes

#### Scenario: Rejected command forms
- **WHEN** a binding sets `server.run = { command = [] }`, or `client.run = { command = "ls" }`, and the user runs `ship config check`
- **THEN** it reports the field with its line and column and exits 1

#### Scenario: Run from a menu entry
- **WHEN** a menu entry's action is `client.run = { command = ["touch", "/tmp/hit"] }` and the user picks it
- **THEN** the command runs as it would from a key

### Requirement: Commands run as argv
A command SHALL run with its first element as the program and the rest as its arguments, with no shell involved. A shell SHALL run only when the command names one.

#### Scenario: No shell expansion
- **WHEN** a binding runs `client.run = { command = ["touch", "~/hit"] }`
- **THEN** a file named `~/hit` is created relative to the command's directory, and `~` is not expanded

#### Scenario: A shell when named
- **WHEN** a binding runs `server.run = { command = ["sh", "-c", "echo $HOME > /tmp/where"] }`
- **THEN** `/tmp/where` holds the server's home directory

### Requirement: Where a command runs
`server.run` SHALL start its command on the server's machine and `client.run` on the client's machine. `server.run` SHALL have no `ship` CLI form.

#### Scenario: No CLI form
- **WHEN** a user runs `ship --help`
- **THEN** no `run` command is listed

### Requirement: Commands don't block
Running a command SHALL NOT wait for it to exit. The binding's later actions and the user's next keys SHALL proceed while it runs, and each trigger SHALL start a new run even if an earlier one is still running.

#### Scenario: Typing during a run
- **WHEN** a key runs `server.run = { command = ["sh", "-c", "sleep 5; touch /tmp/hit"] }` and the user types into a pane at once
- **THEN** the typing reaches the pane without delay, and `/tmp/hit` appears about 5 seconds later

#### Scenario: Repeated presses
- **WHEN** a key bound to `client.run = { command = ["sleep", "30"] }` is pressed three times quickly
- **THEN** three `sleep 30` processes are running

### Requirement: Command environment
A command SHALL get `SHIP_BIN`, `SHIP_SERVER_URL`, `SHIP_ATTACHMENT_ID` for the triggering client and `SHIP_PANE_ID` for its selected pane, unset when nothing is selected. A `server.run` command SHALL otherwise get the same environment as a new pane.

#### Scenario: Calling Ship back
- **WHEN** pane X is selected and a key runs `server.run = { command = ["sh", "-c", "\"$SHIP_BIN\" pane set --name picked"] }`
- **THEN** pane X is named `picked`

#### Scenario: Nothing selected
- **WHEN** nothing is selected and either action runs `["sh", "-c", "env > /tmp/env"]`
- **THEN** `/tmp/env` has no `SHIP_PANE_ID`, and the command still ran

### Requirement: Command directory
A `server.run` command SHALL start in the directory a new pane created from the selected pane would start in. A `client.run` command SHALL start in the directory the client was started from.

#### Scenario: Server run follows the pane
- **WHEN** pane X's shell is in `~/code/app`, X is selected, and a key runs `server.run = { command = ["sh", "-c", "pwd > /tmp/where"] }`
- **THEN** `/tmp/where` holds `~/code/app`

#### Scenario: Client run uses the client's directory
- **WHEN** the client was started from `~/notes` and a key runs `client.run = { command = ["sh", "-c", "pwd > /tmp/where"] }`
- **THEN** `/tmp/where` holds `~/notes`

### Requirement: Command input and output
A command's standard input SHALL be empty. Its standard output and error SHALL go to the run log of the side that ran it, the server's for `server.run` and the client's for `client.run`, and SHALL never be drawn on the client's screen.

#### Scenario: Server output in the server log
- **WHEN** a key runs `server.run = { command = ["sh", "-c", "echo out; echo err >&2"] }`
- **THEN** the server log holds `out` and `err`

#### Scenario: Client output stays off the screen
- **WHEN** a key runs `client.run = { command = ["echo", "hello"] }`
- **THEN** `hello` is in the client log and the client's screen is unchanged

### Requirement: Start failures
When a command can't start, the triggering client's status line SHALL say so and name the command.

#### Scenario: Missing program
- **WHEN** a key runs `server.run = { command = ["no-such-program"] }` or `client.run = { command = ["no-such-program"] }`
- **THEN** the status line says the command couldn't start and names `no-such-program`

### Requirement: Exit reporting
When a `client.run` command exits non-zero or is killed by a signal, the client's status line SHALL say so and name the command. When a `server.run` command does, the server log SHALL record it and clients SHALL NOT be told.

#### Scenario: Client command fails
- **WHEN** a key runs `client.run = { command = ["sh", "-c", "exit 3"] }`
- **THEN** the status line says the command exited with 3 and names it

#### Scenario: Server command fails
- **WHEN** a key runs `server.run = { command = ["sh", "-c", "exit 3"] }`
- **THEN** the server log records the exit status 3, and the status line shows nothing about it

### Requirement: Commands outlive their owner
Ship SHALL NOT stop a command it started. A `server.run` command SHALL keep running after the server stops, and a `client.run` command after its client quits, detaches or loses its terminal, including while the command keeps writing output.

#### Scenario: Client quits
- **WHEN** a key runs `client.run = { command = ["sh", "-c", "while true; do echo tick; sleep 1; done"] }` and the user quits the client
- **THEN** the command is still running 5 seconds later

#### Scenario: Server stops
- **WHEN** a key runs the same loop with `server.run` and the user runs `ship server stop`
- **THEN** the command is still running 5 seconds later

#### Scenario: Ctrl-C on a foreground server
- **WHEN** `ship server` runs in a terminal, a `server.run` command is running, and the user presses Ctrl-C in that terminal
- **THEN** the server stops and the command keeps running
