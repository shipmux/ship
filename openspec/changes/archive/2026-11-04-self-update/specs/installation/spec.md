# Spec Delta

## ADDED Requirements

### Requirement: Install script
`install.sh` SHALL download the latest release's archive for the current platform and its `.sha256`, verify the checksum, and install `ship` to `~/.local/bin`. When `~/.local/bin` isn't on `PATH`, it SHALL say how to add it. It SHALL install nothing and say why on an unsupported platform, a missing archive or a checksum mismatch, and on an unsupported platform it SHALL link to building from source.

#### Scenario: Install on a supported platform
- **WHEN** a user on Linux x86_64, Linux arm64 or macOS arm64 runs `curl -fsSL https://raw.githubusercontent.com/shipmux/ship/main/install.sh | sh`
- **THEN** `~/.local/bin/ship --version` prints the latest release's version

#### Scenario: Not on PATH
- **WHEN** the install finishes and `~/.local/bin` isn't on `PATH`
- **THEN** the script prints how to add it

#### Scenario: Unsupported platform
- **WHEN** a user runs the script on a platform without a prebuilt archive
- **THEN** it says so, links to building from source, installs nothing and exits non-zero

#### Scenario: Checksum mismatch
- **WHEN** the downloaded archive doesn't match its `.sha256`
- **THEN** the script installs nothing, says the checksum didn't match and exits non-zero

### Requirement: Install script location
The install script SHALL be served from `https://raw.githubusercontent.com/shipmux/ship/main/install.sh` until a project domain is chosen ([#64](https://github.com/shipmux/ship/issues/64)).

#### Scenario: README command
- **WHEN** a user copies the install command from the README
- **THEN** it fetches the script from that URL

### Requirement: Release platforms
Every release SHALL publish a `.tar.gz` archive and a `.sha256` for Linux x86_64 (glibc), Linux arm64 (glibc) and macOS arm64, built with self-replace on. Intel macOS SHALL be published only if it cross-compiles and runs under Rosetta without real fixes.

#### Scenario: Release assets
- **WHEN** a release is published
- **THEN** it carries `ship-x86_64-unknown-linux-gnu.tar.gz`, `ship-aarch64-unknown-linux-gnu.tar.gz` and `ship-aarch64-apple-darwin.tar.gz`, each with its `.sha256`

#### Scenario: Release binaries update themselves
- **WHEN** a user installed with the script runs `ship update` after a newer release
- **THEN** the binary replaces itself and `ship --version` prints the newer version
