# Spec Delta

## MODIFIED Requirements

### Requirement: Send outcome in errors
On `send --wait`, the outermost error's `message` SHALL be exactly `nothing was sent` for failures before writing, `the input may have been sent` when writing fails, and `the input was sent` for failures after writing, with the specific failure under `caused_by`.

#### Scenario: Refused before writing
- **WHEN** a script runs `send --wait` on a `blocked` pane without `--if blocked`
- **THEN** the last line on stderr has `.message` equal to `nothing was sent` and the exit status is 1
