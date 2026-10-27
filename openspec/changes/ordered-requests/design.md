# Design

## Context

The full-screen client already puts keys, pastes, clicks and actions into one ordered lane (`crates/ship-client/src/ui/lane.rs`). An item takes its turn only when nothing is running and the replica has caught up to the client's last write (`Lane::next`). Order breaks after the item leaves the client, because the two kinds of item leave on different channels:

- A key, paste or mouse turn hands a frame to the long-lived `POST /api/v0/attach/input` stream (`Local::send`, `ui/mod.rs:673`) and returns `Turn::Done` at once.
- An action turn returns `Turn::Wait` with its own HTTP request, which can reach the server before frames still queued on the stream.

On the server, `route` in `crates/ship-server/src/input.rs` drops a frame for a removed pane with only a debug log. Ship's server already accepts HTTP/2 without upgrade negotiation (axum's `http2` feature), but the client speaks HTTP/1.1.

The probe (`docs/research/request-ordering-probe.md`) measured the replacement on macOS loopback. A JSON POST per event cost about 0.1 to 0.15 ms more than the stream at human speed. A 2000-key burst sent as 2000 concurrent requests took 80 ms over HTTP/2 and opened about 1800 connections over HTTP/1.1. The same burst with batching took two requests and 0.6 ms. Under that burst, requests reached the server out of order even over one HTTP/2 connection, because each request runs in its own task on both sides.

A research pass over other ordering systems (Kafka, SQS FIFO, Pub/Sub ordering keys, Service Bus sessions, TCP, LSP, DAP, VS Code IPC, Eternal Terminal) found that none of them orders across independent requests. All of them number messages within one client's sequence. None skips a gap silently: they refuse, block or abort. A reset is always explicit (Kafka's producer epoch, Pub/Sub's `resumePublish`).

## Goals / Non-Goals

**Goals:**
- The server applies the full-screen client's requests in the order the client made them, with all of the ordering logic in one middleware. Handlers and unnumbered callers don't know it exists.
- Input is an ordinary JSON request, ordered like every other request.

**Non-Goals:**
- Ordering for the CLI, scripts or other attachments. Two clients' requests stay unordered relative to each other, as today.
- Retries, replay or idempotency keys. A refused request makes the client reattach.
- Choosing how many input requests to keep in flight over a real network.

## Decisions

### 1. Number in the client's request path

The client numbers requests itself, so call sites don't change. `Client` gains an ordering slot that the full-screen client fills on `Attached` and clears when the connection drops. While it is set, every request the client builds carries `x-ship-order` with the next number:

```rust
// crates/ship-client/src/lib.rs
struct Sequence { attachment: IdOf<Attachment>, last: u64 }

impl Client {
    pub fn order(&self, attachment: Option<IdOf<Attachment>>);   // set on Attached, None on lost
    fn next_order(&self) -> Option<(IdOf<Attachment>, u64)>;     // called by build()
    pub(crate) fn last_order(&self) -> Option<u64>;              // for the input queue (decision 5)
}
```

`build` already runs when a request is first polled. This keeps number order equal to lane order:

- A key turn takes its number synchronously, inside the turn (decision 5).
- An action turn's request is built when the select loop first polls `lane.finished()`. `Lane::next` starts no other turn while one is running, so nothing else can take a number in between.
- An action that sends several requests, such as `execute` looking up which tab owns a pane, numbers them in the order it sends them.
- The view PUT on the frame tick is numbered too. It may wait behind input, which is harmless.
- The attach POST goes out while the slot is empty, so it is never numbered. The CLI never fills the slot.

Rejected:
- **Numbering at each call site.** Every action path in `execute.rs` would need it.
- **A pipeline endpoint that carries actions and input on one stream** (#20 option 2). The prototype measured +797 / -214 lines, with boxed decoders and a plan/fetch split.

### 2. Ordering is one middleware

`crates/ship-server/src/order.rs` holds the middleware and its state. `AppState` gains `lanes: Arc<Mutex<HashMap<IdOf<Attachment>, Arc<Lane>>>>`, which nothing else reads.

```rust
struct Progress { done: u64, started: u64, running: bool }
struct Lane(watch::Sender<Progress>);

async fn order(State(app): State<AppState>, request: Request, next: Next) -> Result<Response> {
    let Some((attachment, n)) = parse(request.headers())? else {  // 400 if malformed or n == 0
        return Ok(next.run(request).await);                       // unnumbered: untouched
    };
    let lane = app.lane(attachment).await?;                       // 404 if not active
    lane.turn(n).await?;                                          // waits for n - 1; 409 out_of_order
    // Spawned so a dropped connection can't cancel a request that has started.
    let finished = tokio::spawn(async move {
        let response = next.run(request).await;
        lane.finish(n);
        response
    });
    Ok(finished.await.expect("handler panicked"))
}
```

`Lane::turn(n)` loops over the watch:

- If `started >= n`, it refuses: already ran.
- If `done == n - 1`, it claims the number: it sets `started = n` and `running = true` in one `send_modify`, then returns.
- Otherwise it waits for the next change. That wait has no timeout while `running` is true. While nothing is running, it times out after `ORDER_WAIT` (1 s), then logs a warning and refuses with "request n - 1 never arrived".

`finish(n)` sets `done = n` and `running = false`.

A second copy of a number claims nothing. It finds `started >= n` on its next pass and is refused.

The middleware is layered inside `local_only`, so a refused host never touches lane state, and inside `stamp`, so a numbered response still carries `x-ship-revision`.

Rejected:
- **Storing responses so a retry can be answered** (Kafka's idempotent producer). The client never retries, so nothing would read them.
- **Skipping a gap after a timeout.** That silently loses input, which is the bug.
- **Timing the whole wait, including a running predecessor.** A slow handler would refuse the request behind it, though nothing was missing.

### 3. Lanes follow attachments

`lane(app, attachment)` returns the existing lane, or creates one after the state actor's views (`ListViews`) confirm the attachment is active. The views come from the state actor rather than the latest replica, because a replica can lag behind a new attachment. The attachment's guard in `attach.rs`, which already starts the detach, removes its lane when the attachment ends, so the map only holds active attachments. Pruning ended attachments' lanes whenever a lane is created was rejected: the views it prunes against can be stale by the time it takes the lock, and would delete a lane created in between. A refused attachment's lane stays stuck until it detaches, which is fine: its client reattaches as a new attachment, with a new ID and a new sequence. A restarted server has no lanes, and the client's old attachment ID gets 404.

### 4. Input is one JSON body

`InputBody { frames: Vec<InputFrame> }` in `protocol.rs`. The handler:

1. decodes the body with the existing `body::Body` extractor, so the 2 MB default limit and the validation errors match every other request;
2. refuses a mouse frame without an attachment before routing anything;
3. routes every frame in order.

The `StreamReader` and `LinesCodec` loop and the replica `select!` that ended the stream with its attachment go away, along with `INPUT_LINE_MAX`, `Client::send_stream` and the codec features of `tokio-util`. `Client::input` takes `&[InputFrame]` and goes through `request`. The 204 response still carries `x-ship-revision`, so `automation.rs`'s `write` keeps reading the revision after its input.

Rejected: one frame per body. Batching would need a second shape later, and the CLI would send `ls⏎` as three requests.

### 5. The full-screen client queues input

`crates/ship-client/src/ui/outbox.rs` replaces `Local::input` and the `sending` future:

```text
push(frame)                                   called from a turn, synchronously
  if the back batch is unsent and client.last_order() is still its number,
     and it holds under ~1 MiB
    append frame to it
  else
    take the next number, start a new batch {n, frames}
  pump()

pump()
  while fewer than 4 are in flight and a batch is waiting
    start POST /attach/input for the front batch, with its number and frames

next()                                        polled by the select loop
  a request finished → pump(), and return its result
```

The back batch stays open only while nothing else has taken a number. An action turn's number closes it, so a key typed after the action can't join a batch numbered before it.

A batch's request is built with its own number. `Client::input` therefore takes the number from the batch rather than from `next_order`:

```rust
pub async fn input(&self, attachment: Option<IdOf<Attachment>>, order: Option<u64>, frames: &[InputFrame]) -> Result<()>;
```

Detaching waits for the outbox to empty, then returns. Losing the connection drops the outbox.

Rejected:
- **A request per frame with no cap.** The probe's 80 ms bursts, and about 1800 connections over HTTP/1.1.
- **One request in flight.** Over a network each key typed during a request would wait out its round trip.

### 6. Refusals reattach

`ErrorCode::OutOfOrder` (409, kind `out_of_order`). In the select loop, an `out_of_order` error from `lane.finished()` or from the outbox goes through the same path as a lost event stream. That path moves from the inline match arm into one function so both can call it.

Other input errors show on the status line. Today an input stream failure only logs a warning and leaves the client without input until it reattaches.

### 7. HTTP/2 prior knowledge, with a fallback

In `crates/ship/src/main.rs`, `client()` builds reqwest with `http2_prior_knowledge()`. reqwest gains the `http2` feature.

When `connect` gets an explicit target and its health check fails with a network error that isn't a refused connection or a timeout, it rebuilds the client with `http1_only()` and retries health once. On success it logs a warning. The local default target skips the fallback, because Ship's own server always speaks HTTP/2.

On HTTP/1.1 nothing else changes. A stale pooled connection that loses a request shows up as a gap, which becomes a refusal and a reattach.

### 8. Timings

`ORDER_WAIT` is 1 s, under the client's 2 s request timeout, so the client sees a gap's refusal before its own request times out.

The probe's longest wait was 0.3 s. That was in a 2000-key flood without batching, a pattern the outbox prevents. With batching, waits were too short to measure.

The cap of 4 in flight and the ~1 MiB batch size are constants in `outbox.rs`.

### 9. Trace and protocol

The request span in `trace_request` gains `version` (HTTP/1.1 or HTTP/2) and `order`, the header's value. That makes the scenarios checkable from the debug log.

`PROTOCOL_VERSION` goes to 18.

### Slices

1. **Ordered input from a script.** The ordering middleware, `ErrorCode::OutOfOrder`, the JSON input body, `ship pane send` on the new body, the trace fields and protocol 18. Verified with `curl` against a temporary server and with `ship pane send`.
2. **Ordered full-screen client.** Numbering in `Client`, the outbox, reattach on refusal and the detach drain. Verified through a throwaway latency proxy.
3. **HTTP/2.** Prior knowledge and the HTTP/1.1 fallback. Verified from the debug log and an HTTP/1.1-only proxy.

## Risks / Trade-offs

- [Over a future network link, a key typed while 4 input requests are in flight waits for one of them to finish] → Locally, requests finish in under a millisecond. Before remote servers, measure through a latency proxy and tune the cap.
- [A refusal costs a reattach, dropping input that hadn't been sent] → A refusal needs a lost request. With HTTP/2 that means the connection itself failed, which already drops input today.
- [A handler can't be cancelled once it starts] → Every handler finishes within the client's 2 s timeout today. A slow one delays only its own attachment's later requests.
- [HTTP/2 makes local requests a little slower] → The probe measured about 0.07 ms per request against HTTP/1.1 on loopback, too small to feel.
- [The batch's number is taken when its first frame is pushed, but the request may be built later] → The outbox holds the number with the batch and builds the request with it.

## Migration Plan

Update the client and server together, as for every protocol bump. The health check refuses a mismatched pair. A script that posts NDJSON to `/api/v0/attach/input` sends `{"frames": [...]}` instead.
