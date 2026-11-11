# Images inside every screen frame: measured cost

Short answer: yes for UI-style images up to about 800x600, no for photo-like
ones. Variant C (pre-encoded base64 PNG in every frame) adds about 65 us per frame
for an 800x600 UI screenshot, well under the 1 ms target. For an 800x600 noisy,
photo-like image it adds about 0.85 ms median (1.1 to 1.4 ms at p99), which is right
at the target, and at 1920x1080 noisy it adds about 14 ms and sends the whole image
on every frame. The cost is almost entirely proportional to the base64 length
(about 0.5 us per KB of base64, server and client together), and the stream's 2 MiB zstd window
stops de-duplicating the repeated image once a frame's JSON gets near 1 MB.
Variant D (image only in the first frame) adds nothing in steady state at any size.

Measured on an Apple M4 Max, macOS (Darwin 25.6.0, arm64), release build. Linux is
unverified.

## What was run

- 50 rows x 200 cols Neovim-like `ship_core::screen::Screen` (16.4 KB of JSON as an
  `SseEvent::Screen`), a different frame each time (typing in one line, moving cursor,
  status line, a flickering marker). 300 frames per variant, after a 12-frame warmup
  on a throwaway stream.
- One zstd level 3 stream per run (`zstd` crate), `flush()` after every
  `data: {json}\n\n` event; one streaming decoder kept across frames.
- Server time: `serde_json::to_string` plus zstd write and flush. Client time: zstd
  decode, scan for the newline and strip `data: `, `serde_json::from_str`. "Added" is
  the median of per-frame total time minus variant A's.
- Frame type is `struct Frame<D> { event: SseEvent, images: Vec<ImageB64<D>> }`, no
  `flatten`. `D` is `Arc<str>` on the server and `String` on the client.
- Variants: A screen only; B raw RGBA as base64 every frame; C PNG as base64 every
  frame (C is `png` 0.18 `Compression::Fast`; C' is `Compression::Balanced`, the
  crate's default); D PNG(fast) only in frame 0.
- Three full runs. "Added" cells show the median over runs and the min..max.
- Round trip: every decoded frame is asserted equal to its input (full `Screen`
  equality, plus image id, generation, format, dimensions and base64 bytes). No
  assertion failed in any run.

Images (RGBA in memory; sizes are the PNG, then the base64 that goes in the frame):

| image | dims | RGBA B | PNG fast B | PNG default B | base64 of PNG fast |
|---|---|---|---|---|---|
| small | 64x64 | 16,384 | 2,675 | 2,241 | 3,568 |
| medium-shot | 800x600 | 1,920,000 | 94,369 | 54,701 | 125,828 |
| medium-noise | 800x600 | 1,920,000 | 1,231,842 | 1,204,909 | 1,642,456 |
| large-shot | 1920x1080 | 8,294,400 | 485,071 | 235,040 | 646,764 |
| large-noise | 1920x1080 | 8,294,400 | 5,314,178 | 5,193,735 | 7,085,572 |

- small: 64x64 crop of `website/public/og.png` (pixel-art sign).
- medium-shot: 800x600 crop of `landing-1440.png` at (320, 600). This is a UI
  screenshot with flat colors, not a photo.
- large-shot: two `landing-1440.png` crops (1440x1080 and 480x1080) side by side.
- medium-noise, large-noise: synthetic smooth gradients plus uniform +/-12 noise per
  channel. PNG only gets it to 64% of raw size, which is in the range of real photos.

Columns below: "server" and "client" are median microseconds per frame; "added" is
vs A; "first frame" is the total time and compressed bytes for frame 0.

## small, 64x64

| variant | server us | client us | added us | wire B/frame (median / mean) | first frame us / B |
|---|---|---|---|---|---|
| A screen only | 17 | 36 | 0 | 28 / 33 | 76 / 1,877 |
| B raw RGBA b64 | 25 | 40 | +11 | 30 / 43 | 106 / 4,109 |
| C PNG fast b64 | 18 | 37 | +2 | 29 / 43 | 83 / 4,708 |
| C' PNG default b64 | 18 | 37 | +2 | 29 / 42 | 80 / 4,362 |
| D image once | 17 | 37 | 0 | 28 / 42 | 83 / 4,708 |

Everything is in the noise at this size, even B.

## medium, 800x600 UI screenshot

| variant | server us | client us | added us (range) | added p99 us | wire B/frame (median / mean) | first frame us / B |
|---|---|---|---|---|---|---|
| A screen only | 17 | 37 | 0 | 0 | 28 / 33 | 75 / 1,877 |
| B raw RGBA b64 | 1,435 | 673 | +2,055 (+1,979..+2,098) | +2,518 | 50,614 / 50,508 | 2,704 / 50,515 |
| C PNG fast b64 | 61 | 57 | +64 (+63..+65) | +89 | 52 / 323 | 541 / 77,980 |
| C' PNG default b64 | 42 | 48 | +36 (+35..+37) | +45 | 31 / 212 | 259 / 54,462 |
| D image once | 17 | 36 | 0 | -2 | 28 / 287 | 522 / 77,980 |

C meets the target by more than 10x. B misses it by 2x and sends 50 KB on every
frame, because the 1.9 MB of raw pixels is beyond the zstd window.

## medium, 800x600 noisy (photo-like)

| variant | server us | client us | added us (range) | added p99 us | wire B/frame (median / mean) | first frame us / B |
|---|---|---|---|---|---|---|
| A screen only | 17 | 37 | 0 | 0 | 28 / 33 | 74 / 1,877 |
| B raw RGBA b64 | 10,632 | 2,478 | +13,079 (+12,301..+13,399) | +14,689 | 1,709,498 / 1,709,530 | 12,194 / 1,708,363 |
| C PNG fast b64 | 635 | 286 | +871 (+849..+876) | +1,386 | 179 / 36,270 | 3,067 / 1,232,564 |
| C' PNG default b64 | 617 | 278 | +848 (+816..+851) | +1,102 | 179 / 36,079 | 2,795 / 1,207,922 |
| D image once | 17 | 37 | 0 | -11 | 28 / 4,136 | 2,998 / 1,232,564 |

C lands at 0.85 ms median, so it is on the line, not comfortably under it. The
median wire size (179 B) hides that the window is struggling: 142 of 299 later frames
exceeded 1 KB, 2 of them exceeded 100 KB, and the mean is 36 KB per frame (the first
frame's 1.2 MB is a few KB of that mean).

## large, 1920x1080 UI screenshot

| variant | server us | client us | added us (range) | added p99 us | wire B/frame (median / mean) | first frame us / B |
|---|---|---|---|---|---|---|
| A screen only | 17 | 36 | 0 | 0 | 28 / 33 | 74 / 1,877 |
| B raw RGBA b64 | 5,990 | 2,567 | +8,537 (+8,083..+8,688) | +10,858 | 180,708 / 180,734 | 9,771 / 183,102 |
| C PNG fast b64 | 260 | 137 | +343 (+330..+344) | +3,708 | 98 / 17,174 | 2,439 / 314,275 |
| C' PNG default b64 | 127 | 86 | +160 (+157..+160) | +181 | 65 / 864 | 999 / 229,370 |
| D image once | 17 | 37 | +1 | +3 | 28 / 1,075 | 2,393 / 314,275 |

Median cost is fine (0.34 ms), but the 664 KB JSON frame is big enough that zstd
sometimes loses the match: 55 of 299 later frames exceeded 1 KB and 14 exceeded
100 KB (the image re-sent), which is where the 3.7 ms p99 and the 17 KB mean come
from. The smaller PNG (C', 330 KB JSON) has no such frames: 9 over 1 KB, none over
100 KB, p99 +181 us. Slower PNG compression bought a better wire profile here.

## large, 1920x1080 noisy (photo-like)

| variant | server us | client us | added us (range) | added p99 us | wire B/frame (median / mean) | first frame us / B |
|---|---|---|---|---|---|---|
| A screen only | 17 | 37 | 0 | 0 | 28 / 33 | 78 / 1,877 |
| B raw RGBA b64 | 44,790 | 10,670 | +55,450 (+55,335..+60,218) | +65,106 | 7,327,882 / 7,327,895 | 61,616 / 7,329,022 |
| C PNG fast b64 | 7,870 | 6,091 | +13,912 (+13,890..+14,003) | +15,071 | 5,308,560 / 5,308,573 | 14,586 / 5,308,956 |
| C' PNG default b64 | 7,582 | 5,074 | +12,596 (+12,585..+13,572) | +13,692 | 5,197,200 / 5,197,198 | 13,546 / 5,197,626 |
| D image once | 17 | 35 | -3 | +1 | 28 / 17,730 | 14,276 / 5,308,956 |

C breaks completely. A 7.1 MB frame is far past the 2 MiB window, so zstd finds no
match against the previous frame: all 299 later frames cost about 5.3 MB on the wire
(zstd just removes base64's 25% padding), and 14 ms of CPU per frame.
Variant D's mean of 17.7 KB per frame is the first frame's 5.3 MB spread over 300.

## Window effect

Default level 3 has a 2 MiB window. Observed on the wire, frames after the first:

| JSON per frame | case | frames > 1 KB | frames > 100 KB | median wire B |
|---|---|---|---|---|
| 89 KB | medium-shot C' | 0 | 0 | 31 |
| 143 KB | medium-shot C | 3 | 0 | 52 |
| 330 KB | large-shot C' | 9 | 0 | 65 |
| 664 KB | large-shot C | 55 | 14 | 98 |
| 1.66 MB | medium-noise C | 142 | 2 | 179 |
| 2.58 MB | medium-shot B | 299 | 0 | 50,614 (same as its first frame, 50,515: no cross-frame match, the image only compresses on its own) |
| 7.1 MB | large-noise C | 299 | 299 | 5,308,560 |

So repeated images stay cheap on the wire up to roughly 300 KB of JSON per frame,
leak intermittently between 600 KB and 1.7 MB, and are not de-duplicated at all at
several MB. I did not dig into why the leaks start well below 2 MiB; it looks like
a streaming-buffer effect, but that is a guess.

Side note, level 3 with `window_log` 24 (16 MiB), C only:

| case | added us (range) | wire B/frame (median / mean) | frames > 1 KB / > 100 KB |
|---|---|---|---|
| large-shot | +343 (+325..+346) | 101 / 1,159 | 3 / 0 |
| large-noise | +4,488 (+4,430..+4,883) | 686 / 26,207 | 46 / 0 |

A larger window mostly fixes the wire size (the 7.1 MB frame drops from 5.3 MB to a
median of 686 B, max 88 KB) and removes the p99 spikes for large-shot. It does not
fix CPU: large-noise still adds 4.5 ms per frame, since serializing, compressing and
parsing a 7 MB string every frame is the cost. It also needs a bigger window on
both sides of every attachment.

## One-time encode costs

Median of 5 (large) or 15 (smaller) reps per run, then the median over the three runs; run-to-run spread was within about 5%. Microseconds.

| image | PNG fast | PNG default | base64 of RGBA | base64 of PNG fast |
|---|---|---|---|---|
| small | 8 | 102 | 4 | 1 |
| medium-shot | 570 | 5,421 | 453 | 25 |
| medium-noise | 1,953 | 37,193 | 464 | 299 |
| large-shot | 3,706 | 24,649 | 2,041 | 119 |
| large-noise | 8,931 | 160,999 | 1,989 | 1,302 |

These are once per image
version, so they matter for how fast a changed image appears, not for steady-state
frame cost. PNG default compression costs roughly 7 to 19x as much as fast and saves 40 to 50% on
screenshots, almost nothing on noise.

## Reading the numbers

- Added cost scales with base64 length at about 0.5 us per KB, server plus client
  together: 64 us for 126 KB (medium-shot), 343 us for 647 KB (large-shot), 871 us
  for 1.6 MB (medium-noise). A 1 ms budget is roughly 2 MB of base64 if the window
  holds, but the window stops holding well before that.
- Variant D costs nothing per frame at any size. Its only cost is the one big first
  frame (0.5 ms to 14 ms CPU here, 78 KB to 5.3 MB on the wire), which is the same
  cost C pays on its first frame.
- These numbers are per attachment. Ship serializes and compresses each event
  independently per attachment, so with N attachments viewing the same pane every
  added figure is paid N times on the server. I did not model that.

## Not measured

- Linux (unverified). Only Apple M4 Max, macOS.
- Real network or HTTP/2 framing, tower-http's actual chunking, the SSE parser used by
  `ship-client`, tokio scheduling. The bench is single-threaded and in-process, so
  it models the CPU cost and the zstd stream, not transport.
- Several images in one pane, or a pane whose image changes (a generation bump would
  re-send one image like a first frame).
- Memory: B and C hold the base64 string once as `Arc<str>` on the server; the
  client's `String` is a fresh copy per frame (large-noise: 7 MB allocated each frame).
