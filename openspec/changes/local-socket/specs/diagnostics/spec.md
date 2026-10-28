## MODIFIED Requirements

### Requirement: Server run files
A server SHALL write its log under `$XDG_STATE_HOME/ship/logs/<name>/`, named for the server's name and defaulting to `~/.local/state/ship/logs/<name>/` on Linux and macOS. Each run SHALL start a new file named for its start time in local time, and SHALL never append to an earlier run's file. Lines SHALL carry local time with its offset. Background and foreground servers SHALL both write the file; a foreground server SHALL also print to its terminal. Other platforms' location is best effort.

#### Scenario: Two runs
- **WHEN** the `default` server starts, is killed with SIGKILL, and another starts under the same name
- **THEN** `logs/default/` holds two files named like `2026-10-15T18-40-07.log`, and the killed run's last lines are in its own file

#### Scenario: Foreground server
- **WHEN** a user runs `ship server --name scratch`
- **THEN** its lines print to the terminal and are also written to a new file in `logs/scratch/`

#### Scenario: Line time
- **WHEN** a user reads a line of a run file
- **THEN** its time is local with an offset, such as `2026-10-15T18:40:07.123456+02:00`, matching the file name's clock

### Requirement: Current link
Each server name's directory SHALL hold `current.log`, a link to the file the running server is writing, moved when a server starts and when it moves to a new part. Where links can't be created, it SHALL be skipped without failing the run.

#### Scenario: Follow the running server
- **WHEN** the `default` server starts
- **THEN** `logs/default/current.log` resolves to that run's file, and `tail -f` on it shows the server's lines

### Requirement: Parts and retention
When a run's file reaches 5 MiB, the run SHALL continue in a numbered next part, such as `2026-10-15T18-40-07.2.log`, without splitting a line. At start and at each new part, the oldest files in that directory SHALL be deleted until it totals at most 200 MB. The part being written SHALL never be deleted.

#### Scenario: Rollover
- **WHEN** a run's file passes 5 MiB
- **THEN** later lines go to `<start>.2.log` and `current.log` follows it

#### Scenario: Long debug run
- **WHEN** a server runs for weeks at debug detail
- **THEN** its name's log directory stays at about 200 MB, with the oldest files gone and the current part intact
