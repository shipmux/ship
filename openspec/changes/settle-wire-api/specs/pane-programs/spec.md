# Spec Delta

## MODIFIED Requirements

### Requirement: Pane inspection
`ship pane get` SHALL report the pane's command, starting directory, current title, current directory when known, its process state as `process`, which is running or exited with its exit code and signal, and its status records as program-status describes. The title SHALL follow the title the program sets, and SHALL be absent when the program has set none or has cleared it. Pane JSON SHALL NOT carry a `status` field.

#### Scenario: Program sets a title
- **WHEN** a pane runs `sh -c 'printf "\033]0;hello\007"; sleep 100'`
- **THEN** `ship pane get` reports the title `hello` and `process` with state `running`

#### Scenario: Program reports its status
- **WHEN** a pane's program reports `working` through OSC 7501
- **THEN** `ship pane get` shows the record under `program` with the key `""`
