# Proposal

Status: Locked. Cyan approved it in conversation on 2026-10-27, including the defaults in design decisions 2, 4, 6, 7 and 8, after asking for it on 2026-10-26 and choosing to skip the design papers. This proposal, `design.md` and the deltas under `specs/` are the contract.

## Why

Keys typed just before a key binding can reach the server after that binding has acted ([#20](https://github.com/shipmux/ship/issues/20)). Typing into a pane and then closing it with a key can drop the typed keys without any error. The cause is that keys travel on a long-lived streaming POST, while actions are separate requests, and nothing orders one against the other. Locally the window is a few milliseconds. Over a network it would grow to a round trip.

The fix doesn't need a pipeline endpoint or any request-mapping code in the client. Every request the full-screen client sends carries a number, and one server middleware runs each attachment's numbered requests in that order. Input stops being a stream and becomes an ordinary JSON request, so it is ordered like everything else. A probe measured this as fast as the stream (`docs/research/request-ordering-probe.md`).

## What Changes

- **Ordered requests.** A request with an `x-ship-order: {attachment}/{n}` header runs only after that attachment's request `n - 1` has finished. A number that already ran, or one whose predecessor never arrives, is refused with 409 `out_of_order`. A request without the header is handled as today. Only the full-screen client sends the header. The CLI and scripts don't.
- **BREAKING: input is a JSON request.** `POST /api/v0/attach/input` takes `{"frames": [...]}` as `application/json` and answers 204 once every frame is routed. The NDJSON stream and its 64 KiB line limit go away. A paste can now be as large as a request body (2 MB).
- **BREAKING: all or nothing.** A body with a bad frame, or a mouse frame without an attachment, is refused before any frame is routed. Today the frames before the bad line are delivered.
- **The full-screen client numbers everything it sends after attaching.** It keeps up to 4 input requests in flight. Input that arrives while 4 are in flight goes out together in the next request.
- **A refused number makes the client reattach**, dropping input it hadn't sent yet, as a lost connection does today.
- **The client prefers HTTP/2.** It uses HTTP/2 without upgrade negotiation, which Ship's server already accepts. If an explicit target can't be reached that way, it retries over HTTP/1.1 and logs a warning.
- **The exception in Ordered input is removed.** A `server.` action no longer takes effect before keys typed just before it.
- **BREAKING: protocol 18.**

Not in this change:
- Ordering for the CLI or scripts. Each command already waits for its own response.
- Retrying or replaying refused requests. A refusal means reattaching.
- Tuning the number of input requests in flight over a real network. Remote servers don't exist yet. Before they do, measure with a latency proxy.

## Capabilities

### New Capabilities

- `request-ordering`: the `x-ship-order` header, how the server runs numbered requests in order, and its refusals.

### Modified Capabilities

- `terminal-client`: Ordered input loses its server-action exception and covers closing a pane right after typing. Input across a disconnect or detach covers in-flight input requests and the reattach after a refused number. A new requirement covers input bursts.
- `pane-automation`: the input endpoint takes a JSON body. The input requirements no longer describe a stream, and Input stream lines gives way to Input body and Refused input body.
- `health-exchange`: protocol 18, and the client's HTTP/2 preference with its HTTP/1.1 fallback.

## Impact

- `ship-core`: `PROTOCOL_VERSION` 18, an `ORDER_HEADER` constant, an `InputBody { frames }` request type, and an `OutOfOrder` error code (409). `INPUT_LINE_MAX` is removed.
- `ship-server`: a new ordering middleware with lane state on `AppState`. `input.rs` loses its stream reader and replica `select!` and decodes one body.
- `ship-client`: the client numbers requests once the full-screen client sets its attachment. `send_stream` is removed. The full-screen client's input channel and `sending` future are replaced by a small queue of input requests. `automation.rs` sends one JSON body.
- `ship`: reqwest gains the `http2` feature. The client builder uses HTTP/2 prior knowledge, with an HTTP/1.1 fallback for explicit targets.
- Dependencies: reqwest's `http2` feature is added. `tokio-util`'s `io` and `codec` features are dropped, since only `input.rs` uses them. Its `sync` module stays.
- Docs: `docs/research/request-ordering-probe.md` is new. No user docs describe the input endpoint's wire format.
- Size: about +150 implementation lines net, estimated, not measured. No tests are planned.
