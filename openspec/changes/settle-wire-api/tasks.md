# Tasks

Two groups, one per slice in `design/program.md`. The change ships as one pull request.

Verification uses servers on their own names, not any server already running. Each group ends with `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings`; the last also runs `openspec validate --all --strict`. Probes are temporary and aren't committed. Record the platform each check ran on; others are unverified.

## 1. Settle the shapes: renames and protocol bump

- [ ] 1.1 Bump `PROTOCOL_VERSION` to 24, rename `Pane.status` to `process` with `#[serde(alias = "status")]`, and rename `PaneStatus` to `ProcessState` at every use.
- [ ] 1.2 Move every route and client path from `/api/v0` to `/api`, including doc comments.
- [ ] 1.3 Update `docs/site/scripting/README.md`, the promise and key-order rule in `docs/site/scripting/writing-a-plugin.md`, the Process state entry in `GLOSSARY.md`, and `protocol=24` in `examples/plugins/git.sh`.
- [ ] 1.4 Verify `rg -n 'api/v0' crates docs/site examples` is empty, `ship pane get` shows `process` and no `status`, `ship pane wait --status exited` still ends on exit, and a saved file written by the main-branch build restores.
- [ ] 1.5 Drive a normal session in the full client: attach, split, type, resize, select.

## 2. Tolerant client: skipping decoder, watch pipe, health on reattach

- [ ] 2.1 Derive `SseKind` on `SseEvent`, add `EndReason::Unknown` with `#[serde(other, skip_serializing)]`, and skip undecodable records with `MapSkipError` on `Pane.program`.
- [ ] 2.2 Split `Client::attach` into `attach_lines` and the skipping `decode`, add `observe_lines`, and handle `EndReason::Unknown` in `automation::next` as `lost`.
- [ ] 2.3 Make `ship watch` print `observe_lines` as received and fail when the stream ends.
- [ ] 2.4 Replace `attach_after` with `reattach_after`, which checks `/health` first; add `Link::Incompatible`, exit through `updates::mismatch`, and reset the backoff on the first event after `attached`.
- [ ] 2.5 With a fake server sending `attached`, an unknown `bell` event, a `state` whose root record is `thinking`, and an `ended` with reason `rebooting`: `ship watch` prints the data lines byte for byte, and the full client stays up, draws the pane without a report and reconnects once.
- [ ] 2.6 With the fake server sending an undecodable known event after every attach, the client log shows growing reconnect delays. With a real server swapped for a protocol-25 build under the same name, the attached client exits with the mismatch message after one reconnect attempt, and the new server logs no attach.
- [ ] 2.7 Check that every delta under `specs/` matches what was built, and run `openspec validate --all --strict`.
