# Request ordering probe

Evidence for the ordered-requests change ([#20](https://github.com/shipmux/ship/issues/20)). Run on 2026-10-26 on macOS (Darwin arm64, rustc 1.98.0), loopback only. Nothing here was run on Linux, so every result is unverified there.

## Summary

Sending input as ordinary JSON POSTs through an ordering middleware is fast enough, as long as the client keeps one input request in flight and puts whatever piles up meanwhile into the next one.

- At human speed (typing every 30 ms, a mouse drag at 125 events a second) a POST per event cost about 0.1 to 0.15 ms more than today's stream: p50 0.2 to 0.3 ms against 0.1 to 0.2 ms. Every request arrived in order, and the middleware never had to wait.
- Firing a POST per event without waiting is the wrong client shape. With 2000 keys sent at once, requests reached the middleware out of order even over one HTTP/2 connection (about 800 of 2000), because each request runs in its own task on both sides. The middleware put them back in order, but keys took 80 ms (p50) to land. Over HTTP/1.1 the same burst opened about 1800 connections and took 250 ms.
- One request in flight with batching handled the same burst in 2 requests and about 0.6 ms, faster than today's stream (0.6 to 3.6 ms).
- Pastes of 64 KiB and 1 MiB took the same time as a request as they do in the stream (0.4 to 0.6 ms and 1.6 to 4.9 ms).
- The middleware never timed out and no request got a 409 in any run.

Not measured: real network round trips, CPU cost per request, and Linux.

## Details

### Setup

A temporary crate (`/tmp/ship-order-probe`, not checked in) held a server and a client in one process, each on its own multi-thread Tokio runtime, sharing one `Instant` clock so latency could be measured directly. It used Ship's versions: axum 0.8.9 with `http2`, reqwest 0.13.5 with Ship's features plus `http2`. The server set `TCP_NODELAY` on accepted connections, as Ship does (`crates/ship-server/src/lib.rs:189`).

Server routes:

- `POST /input/stream` reads NDJSON lines through `StreamReader` and `LinesCodec`, like `/api/v0/attach/input` today.
- `POST /input` takes `Json<{"frames": [...]}>` and answers 204.

The ordering middleware wraps both. It does nothing without `x-ship-order: {lane}/{n}`. With it, the middleware:

1. answers 409 if `n` already ran;
2. waits up to 2 s for `n - 1` to finish, and answers 409 on timeout;
3. runs the handler in a spawned task, so a client disconnect can't cancel it, and marks `n` done when the handler finishes.

Each lane's progress is a `watch::Sender<u64>` in a map keyed by lane. "Delivered" is the moment the handler hands a frame on, standing in for the pane's command channel.

Client modes:

- **stream**: today's shape. One POST through `reqwest::Body::wrap_stream`, a line per frame.
- **spawn**: one POST per frame, numbered in send order, each started with `tokio::spawn` the moment the frame exists.
- **batch**: one sender task, one request in flight. Frames that arrive while a request is in flight go into the next request together.

Workloads:

- typing: 200 one-character keys, 30 ms apart.
- drag: 500 mouse-sized frames, 8 ms apart.
- flood: 2000 keys with no gap, standing in for automation or key repeat.
- paste: 10 frames of 64 KiB, 100 ms apart, and one frame of 1 MiB.

Each run warms the connection first. Three full runs; latency is from frame creation on the client to delivery on the server, in ms.

### Results

Ranges cover the three runs. Every row delivered every frame in order, with no 409s and no client errors.

| Workload | Proto | Mode | p50 | p99 | Requests | Arrived out of order | Connections |
| --- | --- | --- | --- | --- | --- | --- | --- |
| typing | h2 | stream | 0.17 | 0.21 to 0.30 | 1 | 0 | 1 |
| typing | h2 | spawn | 0.32 to 0.33 | 0.44 to 0.49 | 200 | 0 | 1 |
| typing | h2 | batch | 0.31 to 0.32 | 0.44 to 0.51 | 200 | 0 | 1 |
| typing | 1.1 | stream | 0.14 | 0.17 to 0.21 | 1 | 0 | 1 |
| typing | 1.1 | spawn | 0.25 | 0.32 to 0.41 | 200 | 0 | 1 |
| typing | 1.1 | batch | 0.24 | 0.33 to 0.37 | 200 | 0 | 1 |
| drag | h2 | stream | 0.11 | 0.19 to 0.22 | 1 | 0 | 1 |
| drag | h2 | spawn | 0.20 to 0.21 | 0.40 to 0.41 | 500 | 0 | 1 |
| drag | h2 | batch | 0.20 | 0.35 to 0.39 | 500 | 0 | 1 |
| drag | 1.1 | stream | 0.09 to 0.10 | 0.17 | 1 | 0 | 1 |
| drag | 1.1 | spawn | 0.16 to 0.18 | 0.27 to 0.50 | 500 | 0 | 1 to 2 |
| drag | 1.1 | batch | 0.15 to 0.16 | 0.28 to 0.32 | 500 | 0 | 1 |
| flood | h2 | stream | 0.81 to 3.62 | 1.46 to 7.04 | 1 | 0 | 1 |
| flood | h2 | spawn | 80.6 to 85.9 | 155 to 157 | 2000 | 788 to 862 | 1 |
| flood | h2 | batch | 0.55 to 0.62 | 0.57 to 0.63 | 2 | 0 | 1 |
| flood | 1.1 | stream | 0.62 to 0.64 | 1.08 to 1.14 | 1 | 0 | 1 |
| flood | 1.1 | spawn | 222 to 255 | 308 to 341 | 2000 | 1868 to 1938 | 1757 to 1835 |
| flood | 1.1 | batch | 0.69 to 1.63 | 0.71 to 1.64 | 1 to 2 | 0 | 1 |
| paste 64 KiB | h2 | stream | 0.44 to 0.45 | 0.45 to 0.57 | 1 | 0 | 1 |
| paste 64 KiB | h2 | spawn / batch | 0.59 to 0.60 | 0.61 to 0.94 | 10 | 0 | 1 |
| paste 64 KiB | 1.1 | stream | 0.39 to 0.40 | 0.43 to 0.48 | 1 | 0 | 1 |
| paste 64 KiB | 1.1 | spawn / batch | 0.25 to 0.52 | 0.54 to 0.69 | 10 | 0 | 1 |
| paste 1 MiB | h2 | all | 1.93 to 4.87 | | 1 | 0 | 1 |
| paste 1 MiB | 1.1 | all | 1.60 to 2.15 | | 1 | 0 | 1 |

In the flood rows, the middleware made 1980 to 1998 requests wait for their predecessor. The longest wait was 19 to 28 ms over h2 and 285 to 318 ms over HTTP/1.1, well under the 2 s timeout.

### Findings

**Cost per request.** At human speed a request per event adds about 0.1 to 0.15 ms on loopback, against a 16 ms frame at 60 Hz. The stream's advantage is real but too small to feel.

**HTTP/2 doesn't guarantee arrival order.** One connection delivers the bytes in order, but reqwest sends each spawned request from its own task, and hyper's server runs each HTTP/2 stream in its own task too. Under a burst, the order the middleware sees them in is up to the schedulers. The middleware has to wait for missing numbers even over HTTP/2. It isn't only an HTTP/1.1 fallback concern.

**Batching makes ordering cheap.** With one input request in flight, input requests can't overtake each other, so the middleware almost never waits on input. A burst collapses into a couple of requests, which beats the stream. Numbered action requests (selects, splits) can still overtake an input request; the middleware orders those.

**HTTP/1.1 without batching opens a connection per concurrent request.** reqwest's pool opened about 1800 connections for the 2000-key flood. One request in flight keeps HTTP/1.1 on a single connection, which also makes gaps from a stale pooled connection less likely.

**Network round trips are unmeasured.** On loopback a request finishes in well under a millisecond, so batching never holds a key back for long. Over a real network, a key typed while the previous request is in flight waits for that request's round trip before it goes out. At 50 ms round trip and 30 ms between keys, that's up to one extra round trip of echo delay. The stream doesn't have that cost. This matters only once remote servers exist. Possible answers then are a small number of input requests in flight (2 to 4) before batching starts, or measuring before choosing. Simulating latency needs root (`dnctl`/`pfctl` on macOS, `tc netem` on Linux) and wasn't run.
