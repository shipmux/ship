# Spec Delta

## MODIFIED Requirements

### Requirement: Buildable application and command help
The project SHALL provide a buildable Ship executable with help and version output that need no running server. Version output SHALL include the protocol, as `ship 0.1.0 (protocol 21)`. Help SHALL describe bare `ship` opening the client and `alt-q` detaching, `ship server status`, server startup with `--name` and `--starter`, target selection by URL or name, `--config`, `ship config check`, `ship config defaults`, `ship completions`, `ship update` and automatic background-server persistence.

#### Scenario: Foundation help without a server
- **WHEN** a developer builds the foundation and runs `ship --help` or `ship --version` with no server running
- **THEN** the requested output is displayed and the command exits successfully without starting a server

#### Scenario: Command discovery
- **WHEN** a user requests help
- **THEN** help describes `ship`, `ship server status`, `ship --server-url URL`, `ship --server-name NAME`, `ship --config FILE`, `ship config check`, `ship config defaults`, `ship completions SHELL`, `ship update [--check]`, `ship server [--name NAME] [--starter]` and that automatic startup leaves the server running

#### Scenario: Version with protocol
- **WHEN** a user runs `ship --version` on version 0.1.0 with protocol 21
- **THEN** it prints `ship 0.1.0 (protocol 21)`

### Requirement: End-user documentation
The README SHALL explain what Ship does at this checkpoint, the install command, prerequisites, build/run commands, target selection, background startup, normal shutdown, the default keys and the config file. For chord spelling it SHALL point to crokey instead of repeating it. Internal implementation/update instructions and slice evidence SHALL remain in development planning rather than the README.

#### Scenario: Run from documented instructions
- **WHEN** a user follows the README for the completed checkpoint
- **THEN** they can build Ship, run the supported workflows and stop the server normally

#### Scenario: Change a key from the README
- **WHEN** a user follows the README's config section to rebind a key
- **THEN** the binding works as described and `ship config check` accepts the file

#### Scenario: Install from the README
- **WHEN** a user on a supported platform runs the README's install command
- **THEN** `ship` is installed, as installation's Install script describes
