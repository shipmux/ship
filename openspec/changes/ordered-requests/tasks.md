# Tasks

Verification uses real commands and temporary probes, not permanent authored tests. Use an owned temporary server on a port from 43331 to 43337, with `XDG_STATE_HOME` and `XDG_CONFIG_HOME` pointed at temporary directories, and don't touch the server on 43179. Checks run on macOS; Linux is unverified unless run there.

## 1. Ordered input from a script

- [x] 1.1 In `ship-core`, add `ORDER_HEADER` (`x-ship-order`), `InputBody { frames }` and `ErrorCode::OutOfOrder` (409, `out_of_order`). Remove `INPUT_LINE_MAX` and set `PROTOCOL_VERSION` to 18. Verify that `cargo build --workspace` passes and that `ship server status` prints `protocol_version: 18`.
- [x] 1.2 Add `crates/ship-server/src/order.rs` with the lane map on `AppState`, `Lane::turn`/`finish`, lane removal in `AttachmentGuard`, and the middleware, layered inside `local_only` and `stamp` (design decisions 2 and 3). Add `version` and `order` to the `trace_request` span. Verify with `curl --http2-prior-knowledge` against a temporary server, using an active attachment's ID from `ship view list`:
  - an unnumbered `ship tab create` while numbered requests wait is handled at once;
  - `x-ship-order: nonsense` gets 400, and an unknown attachment gets 404;
  - request 2 (a key for a pane) sent before request 1 (removing it) runs after the removal, and the debug log shows the key dropped;
  - request 1 sent twice gets 409 `out_of_order` saying it already ran;
  - requests 1 and 3 without 2: request 3 gets 409 about 1 s after 1 finished, and the log has a warning naming request 2;
  - a temporary 3 s sleep in one handler doesn't refuse the request behind it (remove the sleep afterwards);
  - a new attachment's request 1 runs at once, and an attachment's lane is gone once it detaches (temporary debug log of the map size, removed afterwards).
- [x] 1.3 Rewrite `input.rs` to take `body::Body<InputBody>`, refuse a headerless mouse frame before routing any, and route every frame (design decision 4). Change `Client::input` to post one body through `request`, remove `send_stream`, and update `automation.rs`'s `write`. Drop `tokio-util`'s `io` and `codec` features. Verify:
  - `ship pane send --pane <id> "hello" --key esc --key enter` against `cat -v` shows them in order;
  - piping 1 MB to `ship pane send --pane <id> -` arrives as one paste;
  - with `curl`: a paste and two keys in one body arrive in order with 204; an extra `colour` field is ignored; a bad `text` after a good paste gets 400 `validation` with nothing routed; a body that isn't JSON gets 400 with one syntax entry; a key and a mouse frame without the attachment header get 422 with neither routed; `x-ship-attachment-id: nonsense` gets 400;
  - `ship pane send --wait` still waits for the program to pick up the input;
  - the OpenAPI document lists `InputBody` as the input request type.

  Until 2.2, the full-screen client's input channel feeds `forward` in `ui/mod.rs`, which sends one input request at a time with whatever queued meanwhile. 2.2 replaces it with the outbox.
- [x] 1.4 Verify the slice: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and `openspec validate ordered-requests --type change --strict`. Report implementation and test line counts separately.

## 2. Ordered full-screen client

- [x] 2.1 Add the ordering slot to `Client` (`order`, `next_order`, `last_order`), with `build` adding `x-ship-order` while it's set and `Client::input` taking an explicit number (design decision 1). Fill it on `Attached` and clear it on a lost connection in `ui/mod.rs`. Verify from the debug log that a TUI session's requests after attaching carry consecutive numbers, the attach POST carries none, and `ship tab list` from another shell carries none.
- [x] 2.2 Add `ui/outbox.rs` (design decision 5) and replace `Local::input`, the `sending` future and the detach drain with it. Move the lost-connection arm into one function and call it on an `out_of_order` error from the lane or the outbox. Show other input errors on the status line (design decision 6). Verify:
  - idle typing into a shell sends each key in its own request and feels no slower;
  - writing 2000 keys into the client's terminal at once, without bracketed paste, delivers all 2000 in order in fewer than 2000 requests, with never more than 4 input requests in flight in the debug log;
  - a 3 MB paste shows the server's refusal on the status line, and typing afterwards still arrives;
  - a temporary change that skips one number makes the client log a warning and reattach, and typing after the reattach arrives (remove the change afterwards).
- [x] 2.3 Write a throwaway TCP latency proxy in `experiments/latency-proxy/` that delays each chunk 500 ms each way. Verify through it, ten runs each: `alt-n` then `echo hi` and Enter lands in the new tab; `alt-|` then `alt-f` zooms the new pane; typing `ls` then `alt-shift-x` routes both keys before the removal with no "input for a removed pane dropped" line in the debug log; cutting the proxy mid-burst after `alt-n` drops the waiting keys and doesn't resend the create; `touch done`, Enter and `alt-q` leaves `done` behind after detaching.
- [x] 2.4 Verify the slice with fmt, clippy, tests and `openspec validate` as in 1.4, and remove `experiments/latency-proxy/` unless slice 3 still needs it.

## 3. HTTP/2

- [x] 3.1 Add reqwest's `http2` feature. Make `client()` in `crates/ship/src/main.rs` use `http2_prior_knowledge()`, and make `connect` retry an explicit target's failed health over `http1_only()` once, for errors other than a refused connection or a timeout, logging a warning on success (design decision 7). Verify:
  - the server's debug log shows `HTTP/2.0` on a local `ship` session's requests and on `ship tab list`;
  - `ship server status --server-url http://127.0.0.1:43336` with nothing listening reports the refused connection once, with no HTTP/1.1 retry in the client log;
  - through a throwaway HTTP/1.1-only reverse proxy in front of the temporary server, `ship --server-url` logs the fallback warning, attaches, and typing and actions work, with the server log showing `HTTP/1.1`.
- [x] 3.2 Verify the change with fmt, clippy, tests and `openspec validate` as in 1.4, remove any throwaway proxies, and report implementation and test line counts against the proposal's estimate.

## Workflow follow-up

- Update #20 with the outcome, and the product brief's milestone 5 line, once Cyan approves.
- Archive with `/openspec-archive-change` after Cyan's review.
