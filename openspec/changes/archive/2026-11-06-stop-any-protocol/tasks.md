# Tasks

One group, `design/program.md`'s single slice. It ships in the same pull request as the client's keyboard fix.

Verification uses servers on their own names, not any server already running. The group ends with `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` and `openspec validate --all --strict`. Checks ran on macOS; Linux is unverified.

## 1. Stop and status accept any protocol

- [x] 1.1 Add `Accept` to `health`, `checked` and `checked_target`, pass `AnyProtocol` for `server stop` and `server status` and `Compatible` elsewhere, and route `update_server` through `health`.
- [x] 1.2 Verify against a protocol-22 server with a protocol-23 client that status prints protocol 22 and exits 0, stop stops it, and `tab list` and `server log` fail with the mismatch error.
- [x] 1.3 Verify against a socket whose health names another service that `server stop` fails without sending a stop request.
- [x] 1.4 Check that the `health-exchange` delta matches what was built.
