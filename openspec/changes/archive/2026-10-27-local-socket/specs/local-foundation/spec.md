## ADDED Requirements

### Requirement: Windows CI
CI SHALL build and test Ship on Windows in a job whose failure does not fail the workflow. The Linux and macOS jobs SHALL stay required. Windows behavior beyond what that job runs SHALL be reported as unverified.

#### Scenario: Windows job on a pull request
- **WHEN** a change is pushed to a pull request
- **THEN** a Windows job builds and tests Ship, and its result shows on the pull request without blocking the merge

### Requirement: Socket documentation
The guide SHALL explain where a server's socket is, how to run and select named servers, and how to reach a server on another machine by forwarding its socket over ssh. It SHALL NOT describe ports.

#### Scenario: Reach a remote server from the guide
- **WHEN** a user follows the guide to use Ship running on another machine
- **THEN** they can find the remote socket with `ship server status`, forward it with `ssh -N -L`, and run `ship --server-url unix://<local socket>` against it

## MODIFIED Requirements

### Requirement: Buildable application and command help
The project SHALL provide a buildable Ship executable with help and version output that do not require a running server. Help SHALL describe bare `ship` opening the client and `alt-q` detaching, `ship server status`, explicit server startup with `--name` and `--starter`, target selection by URL and by name, `--config`, `ship config check`, `ship config defaults` and `ship completions`, and automatic background-server persistence.

#### Scenario: Foundation help without a server
- **WHEN** a developer builds the foundation and runs `ship --help` or `ship --version` with no server running
- **THEN** the requested output is displayed and the command exits successfully without starting a server

#### Scenario: Command discovery
- **WHEN** a user requests help
- **THEN** help describes `ship`, `ship server status`, `ship --server-url URL`, `ship --server-name NAME`, `ship --config FILE`, `ship config check`, `ship config defaults`, `ship completions SHELL`, `ship server [--name NAME] [--starter]` and that automatic startup leaves the server running
