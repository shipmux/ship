# image-frames

Throwaway spike. Measures the per-frame cost of carrying a base64 image inside
every pane screen frame on the attach stream. Results: [results.md](results.md).

Standalone crate (own `[workspace]`), path-depends on `crates/ship-core` for the
real `Screen` and `SseEvent` types. Nothing in the root workspace changes.

```sh
cd experiments/image-frames
cargo build --release
./target/release/image-frames-bench > /tmp/run1.md      # ~45 s, prints markdown tables
./target/release/image-frames-bench > /tmp/run2.md
python3 -I summarize.py /tmp/run1.md /tmp/run2.md       # fold runs into compact tables
```

Env: `ONLY=medium` runs only cases whose label contains that text, `DUMP=1`
writes each test image to `/tmp/dump-<label>.png`, `SHIP_ROOT` overrides the repo
root used to find the screenshots (default `../..`).

Each case streams 300 frames through one zstd level 3 encoder, flushing after each
`data: ...\n\n` event, and decodes them with one streaming decoder. Every decoded
frame is asserted equal to its input (screen and image bytes).
