# Optimal-ish

I was pretty sure Ship was fast. It sends your window the whole screen every time anything changes, which sounds wasteful, but it had beaten every diff I tried, and I remembered it leading on latency in every test I'd run. Then I had Claude build a proper benchmark against Herdr and Zellij, and a key you typed took more than five times longer to show up in Ship than in Herdr.

This post is the couple of weeks after that, in the order it happened, and the quotes are from my chats with Claude while we were in the middle of it. Some of the guesses were wrong, a few of them mine, and the biggest fix we built is the one I threw away. But first you need to know how a screen gets from Ship's server to your window, because almost everything after the race comes back to it.

## 1. Two programs, not one

Ship runs as two pieces. There's a server in the background that owns your terminals, and there's the window you see when you run `ship`, and you can close the window or open three of them and the work underneath keeps going.

```txt
shell, nvim, claude ──▶ server ──screens──▶ window ──▶ your terminal
                        (runs them)          (draws them)
```

The window never runs your programs itself. The server reads everything they print and feeds it into a terminal emulator, which is the part of a terminal app that understands colors and cursor movement and all the escape codes programs use to draw. Ship uses Ghostty's. What comes out is a screen, a grid of characters with colors, and the window's job is to get that screen and draw it, over and over, fast enough that typing feels instant and a pane full of logs doesn't drag everything else down with it.

> [!NOTE]
> This split is also why closing the window doesn't stop anything. [Windows and the server](/docs/windows-and-the-server/) has the rest.

So every question in this post comes down to one thing, which is how a screen gets from the server to the window, and how fast.

## 2. Diffs that kept losing

The obvious way to do that is to send only what changed, so if you type one letter you send one cell. That's what I started with, and it falls apart as soon as something scrolls, because when a new line shows up at the bottom every other line moves up by one.

```txt
before                           after one new line
12:04:01 info  GET /items/7      12:04:02 info  GET /items/14
12:04:02 info  GET /items/14     12:04:03 info  GET /items/21
12:04:03 info  GET /items/21     12:04:04 info  GET /items/28
12:04:04 info  GET /items/28     12:04:05 info  GET /items/35
```

Compare each row with the one next to it and none of them match, so a simple diff ends up sending the whole screen anyway and does extra work to get there. I tried JSON Patch first, and on a one-row scroll the patch came out bigger than the screen itself, about 46 KB of patch against 9.5 KB for the compressed screen, and it took 15 to 27 ms per frame to compute.

A smarter diff can notice that the lines just moved, and then it only sends the new one, so I tried that too. The messages got small, but figuring out what moved where took about half a second on a big screen, even when nothing had changed, and you can't wait half a second to see your own typing.

| a 160×50 screen          | size on the wire | time to make it |
| ------------------------ | ---------------- | --------------- |
| whole screen, compressed | 7.7 to 10.5 KB   | barely anything |
| JSON Patch, one scroll   | 46 KB            | 15 to 27 ms     |
| diff that tracks moves   | tiny             | about 520 ms    |

The whole screen with compression on top was smaller than one diff and faster than the other, which wasn't what I expected.

## 3. The compressor that remembers

Terminal screens repeat themselves a lot. Most of a screen is blank, most of the text is in one or two colors, and a log is the same shape of line over and over with different numbers in it. That's what compression is good at, the same thing a zip file does when it finds the repeated parts and writes them down once, and Ship's server already talks to the window over HTTP, which can compress everything it sends without Ship doing anything special. It's one line in the server.

```rust
CompressionLayer::new().zstd(true).gzip(true)
```

That's not just compressing each screen on its own, though, and this is the part I didn't get at first. The window opens one long stream to the server and every screen goes down that same stream, so there's one zstd compressor per window, and it keeps a window of everything it sent recently. When the next screen shows up, zstd finds that almost all of it matches the screen it just sent and writes "same as before" instead of the screen.

```txt
stream to your window
  screen 1   ████████████████████████  compressed on its own
  screen 2   ████████████████████████  "like screen 1, but row 30 has one more letter"
                                        sent as ~27 bytes
```

So when a coding agent's spinner ticks and nothing else moves, the server sends a whole screen and about 27 bytes go over the wire, which is about the size a diff would be anyway. zstd ends up doing the diffing, and Ship never has to keep track of what the window already has. There's a comment in Ship's screen type that says so.

```rust
/// Full snapshot; zstd on the stream supplies the savings between frames.
```

That memory isn't free. Each attached window costs about 3.7 MB of zstd state on the server and 2.5 MB in the window, and I think that's a fine trade for never having a diff go out of sync.

### Rows instead of cells

How the screen gets written down still mattered. Ship used to describe every cell on the screen as its own little object, blank ones included, and it changed to sending each row as its text plus a short list of which stretches have which style.

```diff lang="rust"
 pub struct Screen {
     pub size: Size,
-    /// `size.cols * size.rows` cells.
-    pub cells: Vec<Cell>,
+    pub rows: Vec<Row>,
     pub cursor: Option<Cursor>,
     // ...
 }
```

Here's the first line of a small log screen in both shapes, captured from a running server.

```diff lang="json"
-"cells": [
-  {"symbol":"1","attrs":["dim"]}, {"symbol":"2","attrs":["dim"]},
-  {"symbol":":","attrs":["dim"]}, {"symbol":"0","attrs":["dim"]},
-  ... one of these for every cell, and {} for every blank one
-]
+"rows": [
+  {"text": "12:04:01 info  GET /api/items/7  200  7ms",
+   "runs": [{"cells":8,"attrs":["dim"]}, {"cells":1},
+            {"cells":4,"fg":{"indexed":2}}, {"cells":20},
+            {"cells":3,"attrs":["bold"]}]},
+]
```

| that screen         | as sent      | compressed |
| ------------------- | ------------ | ---------- |
| one object per cell | 10,578 bytes | 387 bytes  |
| rows of text        | 1,385 bytes  | 294 bytes  |

Compression already did most of the work even on the clunky format, since 1,920 nearly identical little objects are about as repetitive as text gets. So why bother? I measured the time on both ends, and that's where the difference showed up.

| per screen, 103×47 | server writes it | window reads and draws it | on the wire |
| ------------------ | ---------------- | ------------------------- | ----------- |
| agent, cells       | 0.129 ms         | 0.215 ms                  | 80 B        |
| agent, rows        | 0.015 ms         | 0.023 ms                  | 27 B        |
| git log, cells     | 0.134 ms         | 0.218 ms                  | 1,695 B     |
| git log, rows      | 0.014 ms         | 0.021 ms                  | 75 B        |

The bytes on the wire follow how much changed, but the time to write and read a screen follows how big it is before compression, and rows made that about 9 times smaller. Compression makes a verbose format almost free on the wire, but it does nothing about the work on either end.

I also tried rows written as plain terminal escape codes, the same thing a program prints, since that's even smaller. It broke wide characters like CJK text and some emoji, which take up two cells, and 1,498 out of 1,499 screens in that test came out wrong. Rows with named fields won, partly because they're easy to add to later without breaking older windows.

> [!TIP]
> A nice side effect of rewriting this was noticing that Ship, like Zellij, was dropping the colored underlines Neovim uses for errors. Rows keep the underline color now. The squiggly style itself is still waiting on [ratatui #2734](https://github.com/ratatui/ratatui/issues/2734).

## 4. Only the newest screen

The other half is that your window never needs every screen, just the newest one. A pane dumping a big log can change thousands of times in a fraction of a second, and if Ship sent all of those, the window would fall further and further behind and your keystrokes would be waiting in line behind all of it.

So the server keeps one slot per pane, and each new screen just replaces whatever was in it. In the code that's a single line.

```rust
self.screen.send_replace(Arc::new(screen));
```

On the other side, the stream to your window waits for the slot to change and takes whatever is there right then.

```rust
receiver.changed().await?;
Ok(receiver.borrow_and_update().clone())
```

If ten screens went by while the window was busy, it never sees the nine in the middle, and it doesn't need to, because nobody can read a log scrolling that fast anyway. There's never a backlog to work through.

```txt
program prints  ████████████████████████████████████████
screens made    1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 ...
window takes    1         5            10             16
```

It also keeps the rest of Ship simple, which I only appreciated later.

> oh and that doesn't even begin to count the fact that it makes the architecture much easier to reason about since everyone just gets the latest state at all times

In the benchmark, I printed 16 MiB of logs into a pane, and about 15 KB went from the server to the window. With all that in place I figured Ship was done being slow, and the obvious next thing was to race it against the others.

## 5. The first race

I had Claude write a benchmark that runs Ship, Herdr and Zellij side by side in a terminal and measures things like CPU, memory, how fast a log flood gets through and how long a keystroke takes to show up. Getting it to measure the right thing took a few tries, because the harness kept tripping over something.

- Zellij ignores `XDG_CONFIG_HOME` on macOS, so it kept opening its first-run setup screen, which quietly ate every key the benchmark typed.
- Herdr moves the cursor around after non-ASCII characters, so the `⟦END⟧` marker the benchmark waited for never showed up as one piece, and every run timed out.
- The benchmark typed 16 different keys in a cycle, and the 16th one happened to be the quit key for the little echo program on the other end.

Once it was measuring the right thing, the first numbers came back, medians of three runs.

|               | Ship    | Herdr   | Zellij   |
| ------------- | ------- | ------- | -------- |
| log flood     | 72 ms   | 109 ms  | 265 ms   |
| agent CPU     | 2.2%    | 2.3%    | 3.1%     |
| memory        | 45 MB   | 41 MB   | 100 MB   |
| typing (idle) | 3.93 ms | 0.74 ms | 13.35 ms |

> not surprised that we win on flood, although i'm a bit confused... on the typing one... are we winning or losing?

Losing, by a lot. Floods were great, and while an agent was printing Zellij wrote 5.2 MB to the terminal against 64 KB for Ship, about 80 times more, because Zellij forwards a lot more of every change. But typing in a quiet pane took 3.93 ms in Ship and 0.74 ms in Herdr, more than five times slower, and that's the number you feel every time you press a key.

> interesting... that's very strange. i remember leading on latency in pretty much every test i did. what are you thinking the culprit is?

## 6. A millisecond that wasn't

Ship had a small wait before sending a screen, 1 ms, so a program that draws twice in a row (Neovim does this when it scrolls) lands as one screen instead of showing you the half-finished one. One millisecond shouldn't add up to three, so Claude dug into where the time went, and the answer was the clock itself.

Ship runs on tokio, and tokio's timers don't wake you at the exact time you ask. They tick in 1 ms slots and round up to the next one, so asking for 1 ms can easily mean 2.

```txt
time (ms)          10        11        12
tokio's ticks       |---------|---------|
screen changes      ▲ 10.3
asks to wake at               ▲ 11.3
actually wakes                          ▲ 12.0
```

Even asking to wake at `Instant::now()`, which should mean right now, waited for the next tick, about 1.3 ms on every echo. So the wait was costing more than its own length on every keypress.

> i find that rather odd/strange. why does it wait for ms rounding? is there no way around that?

Herdr has a rule for this, and it's a good one. Normally it draws a pane at most once every 16 ms, but a keypress opens a short window where that pane draws right away, so your echo never waits behind any timer. Ship borrowed the idea.

```txt
a pane's screen changed
  if you pressed a key in the last 32 ms
    send it now (at most 16 of these per 16 ms, so a flood still paces)
  otherwise
    wait 1 ms, so a program that draws twice lands as one screen
    and keep at least 4 ms since the last one sent
```

| version                          | typing (idle) |
| -------------------------------- | ------------- |
| rows                             | 3.90 ms       |
| keypress rule, still with timers | 2.77 ms       |
| sending inline, no timer at all  | 1.93 ms       |
| Herdr                            | 0.79 ms       |

Sending inline worked, but in that first version it was a publish dropped into the middle of the pane's loop, and when it came time to land it I didn't love it.

> i'm mostly happy with that, i just think it reads a bit dirty to just publish straight in there. am i the only one who's bothered/annoyed by that?

So the version that landed keeps all of it in a small `Pacing` struct, and the pane asks it whether a screen is due before it waits on anything else.

That got a lot closer, but there was still a millisecond missing, so Claude timed each hop of the trip.

| hop                       | ms   |
| ------------------------- | ---- |
| key reaches the server    | 0.36 |
| server starts publishing  | 0.22 |
| capturing the screen      | 0.21 |
| JSON and sending it       | 0.09 |
| screen reaches the window | 0.21 |
| window redraws            | 0.47 |

Nothing in there was huge on its own, and I parked it there, because the next day Ship had a bigger problem.

## 7. The flood that drew too much

Ship was winning floods on time, but when I looked at CPU per megabyte, Ship used about 20% more than Herdr to chew through the same output. Both use Ghostty to understand the output, so the parsing should cost the same, and the difference had to be somewhere in Ship's own code.

> for the flood case... what do you think is going on there? that feels something that could heavily benefit from profiling, no?

Getting a profile took some work. samply needed a release build with symbols and link-time optimization left on, plus a debugging permission macOS wants before it lets you look inside a process, and the first profile claimed Ship had allocated 40 GB in 9 seconds, which was the profiler being confused and not Ship. The second one made sense.

```txt
copying Ghostty's screen out    ███████████████████████▌  47%   (27% of it malloc)
reading from the terminal       █████████████████         34%
Ghostty parsing                 ███████▌                  15%
```

Almost half the time went to copying Ghostty's screen into a form Ship can draw, and it was doing that after every read from the program, while the server only publishes a screen every 4 ms. So most of those copies were thrown away without anyone looking at them, and during a flood that was about 1.4 CPU cores, including for panes you can't see.

```txt
reads      r r r r r r r r r r r r r r r r r r r r r r r r
copies     C C C C C C C C C C C C C C C C C C C C C C C C
published          ▲               ▲               ▲        (every 4 ms)
```

> wait WTF? how much more do you think we're paying for all of that? ... i had no idea dang. i wonder if that is impacting a bunch of other things

### Two fixes that did nothing

The first idea was to copy once per batch of reads instead of after every read. It did nothing, because the queue of reads was nearly always empty after one chunk, so a batch was almost always one read.

The second idea was to make the copy cheaper. Claude got rid of the allocation for every cell, and malloc dropped from 27% to about 0% of the profile, but CPU per megabyte went from 11.2 ms to 10.9 ms, which is basically nothing. The thread was flat out the whole time, so a cheaper copy just meant it got through each one faster and then did more of them.

At that point I asked the question that should've come first.

> isn't the right way to only render the frame when asked to by the publishing task?

So that's what Ship does now, the copy only happens when the server is about to publish and asks for it, and copying went from 47% of the profile to 0.2%. But CPU per megabyte only went from 10.7 ms to 10.4 ms, because something else had eaten the win, and the new profile showed waking and sleeping the session thread had jumped to 31%.

```txt
session thread, one flood:
  [c]zz[c]zz[c]zz[c]zz[c]zz[c]zz[c]zz[c]zz[c]zz[c]zz ...
   c = take a chunk of output, zz = go back to sleep, wake up again
```

Claude worked out it had to be waking up at least 34,000 times a second per pane, doing a bit of work and going back to sleep. Somewhere around here I noticed Ship was reading from the terminal 4 KB at a time and lost it a bit.

> wtf is the 4KB limit? i feel like i don't have enough context to understand anything...
>
> and why the hell would we pick such a small bucket? what's the benefit of that? would 128KB not be much smarter?

That 34,000 assumed every read got the full 4 KB, so Claude traced a flood to see how big the reads were, and it was worse.

| per pane, per second, during a flood | count   |
| ------------------------------------ | ------- |
| reads from the terminal              | 122,000 |
| average bytes per read               | 1,018   |
| buffers allocated and sent           | 122,000 |
| session thread wake-ups              | 91,000  |
| screens actually published           | ~165    |

macOS hands back at most about 1 KB from a terminal per read, no matter how big a buffer you give it, so 128 KB wouldn't have changed anything.

> wait what? so we have to wake every 1KB? or am i reading something very wrong? WTF? that seems so cursed? i have to be reading it wrong, right?

I wasn't reading it wrong, but the fix didn't need bigger reads, it needed fewer wake-ups. The reading thread now appends whatever it read to one shared buffer, and it only wakes the session thread when that buffer was empty, because if there were already bytes in it, the session thread is already on its way.

```rust
let was_empty = queue.bytes.is_empty();
queue.bytes.extend_from_slice(&buf[..n]);
if was_empty || queue.bytes.len() >= PENDING_CAP {
    pty_tx.try_send(()).ok();
}
```

The session thread takes everything that piled up at the top of each loop, all at once. There's a 1 MiB cap, so a program that floods faster than Ghostty can keep up just has to wait, instead of eating all your memory. Claude's first version had every command remember to catch up on output before running.

> is it just me or does that feel a bit hacky to have to remember? is there a cleaner way?

So there's a single catch-up at the top of the loop instead, one place that does it and nothing to forget.

| server CPU per MB of output | ms          |
| --------------------------- | ----------- |
| before                      | 10.7        |
| copy only when asked        | 10.4        |
| copy when asked + batching  | **6.1**     |
| Herdr                       | 9.1 to 10.5 |

> so this time it actually improved and the gains didn't just get eaten by some other crap? or no?

It did this time. Wake-ups went from about 91,000 a second to about 165, the same as the number of screens published, which is what it should be. What's left is Ghostty parsing at about 2 ms per MB and the reads themselves at about 4 ms per MB, roughly 4 µs per syscall, and there's nothing Ship can do about either one. I also tried building Ghostty for the exact CPU in my laptop, and that didn't change a thing.

## 8. Faster when busy

With the flood fixed, Claude ran a quick check on typing, and one number didn't make sense.

| typing p50      | before  | after keypress rule |
| --------------- | ------- | ------------------- |
| beside floods   | 2.85 ms | 0.49 ms             |
| in a quiet pane | 4.05 ms | 1.80 ms             |

Typing in a pane right next to a flood was more than three times faster than typing with nothing else going on.

> how is ship faster during a flood that without a flood? wtf?

Claude's guess was that a quiet machine lets its cores go to sleep, and waking a sleepy core costs time on every hop. To check, we ran `yes > /dev/null` in the background to keep the cores busy without touching Ship.

|       | quiet machine | with `yes` running |
| ----- | ------------- | ------------------ |
| Ship  | 1.80 ms       | 0.41 ms            |
| Herdr | 0.67 ms       | 0.13 ms            |

Both got four or five times faster just by having the CPU awake. So part of the idle number isn't Ship, it's the hardware napping between keystrokes, and Claude figured Ship pays it more often because a keystroke hops between more threads than in Herdr. I'll come back to that, because it turned out to be wrong.

### The wake-up that came too early

While tracing single keystrokes, Claude noticed something odd. On 149 out of 151 keys, the pane got woken up to redraw before the echo even existed.

```txt
 µs after keypress
   0   key arrives
 414   written to the program
 431   pane wakes up and redraws (nothing changed)
 462   publishes the same screen again
 479   program echoes the letter          ← the pane is busy
 539   pane finally sees the echo
 652   redraw done
 676   the screen with your letter goes out
```

Every key scrolls the view back to the bottom and clears any selection, which is what you'd want if you'd scrolled up. But the code asked for a redraw every time, even when the view was already at the bottom with nothing selected, which is pretty much always. So the pane spent its time redrawing an unchanged screen right as your letter showed up. The fix only asks for a redraw when something moved.

```diff lang="rust"
 fn input_bottom(&mut self, needs_render: &mut bool) {
+    // Usually the view is already at the bottom with nothing selected,
+    // so nothing changes and a render would only delay the echo.
+    let scrolled = !self.terminal.viewport_active().unwrap_or(false);
+    let selected = self.terminal.selection().map_or(true, |s| s.is_some());
     self.terminal.scroll_viewport(ScrollViewport::Bottom);
     self.terminal.set_selection(None).ok();
-    *needs_render = true;
+    *needs_render |= scrolled || selected;
 }
```

Typing beside a flood went from 0.49 ms to 0.38 ms at p50, and the slowest 1% went from about 3.5 ms to about 0.65 ms.

> holy crap... crazy good free optimization

Not everything panned out like that. Each key was also publishing two screens instead of one, and fixing that made no difference to latency. After that I didn't expect much from the window's drawing either.

> as for the client draw. you can take a look at it, but i'd be surprised if there's anything we can realistically shave off there without introducing a mountain of complexity

It had one more surprise, though. ratatui, the library Ship's window draws with, asked for the terminal size on every frame, and getting the size meant opening `/dev/tty`, asking, and closing it again. That was 101 µs per draw, and Ship now sizes the window from resize events instead.

```rust
// Fixed, so drawing doesn't reopen `/dev/tty` for the size every
// frame; `drive` sizes it from resize events.
TerminalOptions {
    viewport: Viewport::Fixed(Rect::default()),
},
```

That took a draw from 101 µs to 16 µs.

> does the size thing cost us anything? would there ever be a chance where we miss an event or something and it would get all wonky and weird?

To make sure a fixed size couldn't go stale, Claude made a build that ignored resizes on purpose and checked that the test caught it.

## 9. Where the last millisecond goes

Even after all that, a quiet keystroke took about twice as long in Ship as in Herdr, and when Claude broke it down it was about 400 µs each way between the window and the server.

> Why does it take nearly 400us each way? that seems absolutely nuts. if we could half that somehow, then that would get us pretty close to herdr i think?..

Claude's theory was thread wake-ups. A keypress goes from the window's UI thread to its network thread and then over to the server, and each handoff wakes up another thread, and if each wake costs 50 to 150 µs on a sleepy core, that's most of the gap. The window ran on tokio's multi-threaded runtime, so the fix would be to run it on a single thread and skip the handoffs.

> i guess it's worth a probe, but i don't really expect much out of it.

I was right not to expect much.

| median µs, sleepy cores | many threads (19) | one thread (3) |
| ----------------------- | ----------------- | -------------- |
| key handed off          | 122 / 123         | 141 / 138      |
| reaches the server      | 192 / 189         | 174 / 186      |
| screen back out         | 156 / 154         | 162 / 159      |
| full round trip         | 2196 / 2030       | 2128 / 2134    |

Nothing moved, and a handoff that took 120 µs with 19 threads took 120 µs with one, so it wasn't a wake-up.

> does compiling with debug symbols plus profiling not help at all? or tracing logs?

Claude explained why they can't see this kind of problem. A profiler only sees a thread while it's busy and a trace log only sees the moments you added a line for, so adding more trace points just narrows the gap until it falls between two lines with nothing to log, and neither one sees why a thread was sitting there doing nothing.

What does see it is a scheduler trace, which records every time the OS puts a thread to sleep, wakes it up or moves it to another core. macOS has one built into Instruments, and it works without sudo. The first try missed most threads because of how the trace file nests things, then everything turned out to be running on efficiency cores, and the automatic wake-chain walker followed a TCP acknowledgement back to the wrong place, so in the end Claude walked a couple of keystrokes by hand, and it showed the theory was wrong, because waking a thread takes about 3 to 10 µs, not 100 or more.

| median µs, one keystroke  | Ship | Herdr | gap |
| ------------------------- | ---- | ----- | --- |
| key reaches the server    | 210  | 125   | 85  |
| program answers           | 246  | 235   | ~0  |
| new screen reaches window | 398  | 189   | 209 |
| window draws it           | 429  | 46    | 383 |
| total                     | 1293 | 618   | 675 |

The "wake-ups" were threads busy running, and Herdr hands off between threads about as many times as Ship does. The biggest gap was the last step, where Ship's window takes 429 µs to draw a keystroke and Herdr's takes 46.

### What Herdr does differently

Around here I was running out of usage for the week.

> bro i'm about to run out of usage. can you wrap up soon? i have 1% weekly usage left lol

But I had one more question first.

> have you checked herdr's drawing logic to see what magical things they do?

It hadn't, and we'd also been benchmarking Herdr with its defaults without checking what those were, which was funny, because the day before I'd already gotten mad about exactly that.

> this is exactly WHY IM SAYING TO CHECK THE CODE for herdr and trace how it actually works end to end for input and rendering...

So Claude finally read it. Plain `herdr` turns out to be just like Ship, a server that sends screens and a client that draws them, and its full redraw looks a lot like Ship's. The difference is a fast path next to it.

When only one pane changed, Herdr's server sends just the rows that changed, and the client skips building the frame and writes those rows straight to your terminal. It only does that when nothing else could've changed, so there are about 23 conditions that kick it back to the full redraw, like an overlay being open, text being selected, copy mode, a notification, hovering a link, or a patch that arrived out of order. All of that takes about 5,800 lines.

```txt
Ship, every keystroke                      Herdr, fast path
  build the whole frame        ~250 µs       (skipped)
  diff it against the last one ~110 µs       (skipped, server named the rows)
  write the changed cells       ~50 µs       write the changed rows ~50 µs
```

> so it's basically like moving part of the diffing logic out of ratatui and into our logic?

Partly, but mostly it skips the rebuild, and doing the same in Ship would mean writing into ratatui's buffers behind its back.

> oh man that's really ugly... that feels like we're reaching into it's guts or something... although in theory... i'm confused as to why ratatui doesn't already do that? why is it wasting time diffing everything? if you compared it to the web world, is it using a model closer to react than to something like solid?

It's React. ratatui works like React without `memo`, so it redraws everything every frame and then compares the result with the last frame to find what to write, and it's the reason Ship does diff screens after all, just at the end, between two frames sitting in memory on your machine. That's why it never misses a change, since it looks at everything, but it means rebuilding a frame that's 99% the same as the last one just so the diff can find out the sidebar didn't move. Herdr's fast path is more like Solid, which tracks what depends on what, except written by hand for one case. On the web the browser does that tracking for you, but a terminal doesn't have a browser, so somebody has to write it.

And Herdr's issue tracker shows what that costs. A lot of its bugs come down to something falling off the fast path when it shouldn't, or staying on it when it shouldn't, like [234 KB going out per keystroke over SSH](https://github.com/herdrdev/herdr/issues/3745), [3.6 GB an hour from a spinner](https://github.com/herdrdev/herdr/issues/4264), and [rows that never repainted after a background color change](https://github.com/herdrdev/herdr/issues/4808).

> it's like a messy spider web that would rather shave off 1ms by handling every possible edge case manually?

And all that work buys Herdr a few hundred microseconds per keystroke.

> even after all of that effort, all we really save is 200 microseconds... honestly, i'm more impressed that herdr was willing to spend the effort to squeeze out all of those extra microseconds on latency. only to be beaten on cpu anyways.

### The fast renderer I threw away

I still wanted to know what Ship could get, so Claude built a version of the window that only redraws what changed. On its own it was about 30 times faster at drawing a keystroke, 325 µs down to 10, and a randomized checker ran it against the normal renderer for 2 million frames without a mismatch. Plugged into Ship, it looked like this.

|                           | normal  | only what changed |
| ------------------------- | ------- | ----------------- |
| window draw per keystroke | 334 µs  | 83 µs             |
| window CPU with an agent  |         | 51% less          |
| typing (idle)             | 1.36 ms | 1.09 ms           |

> holy crap. no fucking way... so this is basically matching herdr?

Then I read the code.

> i'm not gonna lie... that client code looks absolutely abysmal to read. like pure slop... especially with all of the layering logic...

And for what it bought, I couldn't justify it.

> it does seem to be better, but it's way too much effort and weirdness for like... shaving off 200us at best, no?

A frame on a 120 Hz screen is 8.3 ms, and nobody feels 1.3 ms versus 0.6 ms on a keystroke.

> you can't even notice the 1ms overhead. but you can definitely notice the bugs as a user and the maintenance burden as a dev.

It lives on as the `experiment/retained-render` branch.

## 10. The memory nobody asked for

There was one more thing that bugged me.

> wonder how herdr does less memory... that's interesting. because i'd figured that we're pretty efficient?

Sitting idle, Ship's server used 16 MB and its window 10 MB, against 11 MB and 6.3 MB for Herdr, and both of them use Ghostty, so Ship was spending an extra 9 MB on something.

Claude's first guess was that tokio starts too many threads, and capping them would've saved a few MB.

> honestly... it feels so small that i feel like i'd leave it. maybe the client can work fine with less, but i feel like it's not really a noticable cost? like who's fighting over 4-5MB of memory on a 16 core machine?

Its second guess was that Herdr uses an older Ghostty, which turned out to be backwards, because Herdr's Ghostty was newer. The answer was in how big each thread is. Every thread gets its own copy of the program's thread-local storage, and Ship's was a lot bigger.

| thread-local storage per thread | size   |
| ------------------------------- | ------ |
| Herdr                           | 584 B  |
| Ship                            | 263 KB |

The version of Ghostty that Ship used pulled in a piece of Zig's standard library that sets aside a 256 KiB stack per thread for handling signals, and every one of Ship's threads paid for it, Ghostty's or not. Upstream Ghostty had already dropped that piece of Zig back in August, so the fix was just a version bump, and the window shrank the most, which confused me.

> why the heck did the client go down so much? wtf? that's nuts?
>
> why did the client have zig stuff in it?... huh?

The window is the same program as the server, so it carries Ghostty too, and over half the window's memory had been 19 copies of a spare stack nobody ever used.

> well damn... that's pretty sick... so with 0 code changes at all, we're better off than herdr now?

It wasn't quite zero, there was the version bump and one deleted `Ok(())`.

| peak memory, everything included | Herdr   | Ship before | Ship after |
| -------------------------------- | ------- | ----------- | ---------- |
| idle                             | 39.7 MB | 41.8 MB     | 30.9 MB    |
| agent printing                   | 42.2 MB | 46.5 MB     | 35.6 MB    |
| log flood                        | 50.6 MB | 55.8 MB     | 43.8 MB    |
| full-screen animation            | 44.7 MB | 53.7 MB     | 43.8 MB    |

Ship went from losing every row to winning every row, with CPU and typing where they were. The one place Herdr can still come out ahead is a long-lived server with a lot of scrollback, since Ship keeps 10 MB of history per pane.

At that point the list felt done.

> so no more real levers to pull?

## 11. One more look

About ten days later I still had the idle keystroke on my mind, and a theory about my laptop's cores.

> when the cpu isn't under load it actually uses different cores than when it is. and i think that ship actually benefits from that way more.

Checking that meant following one keystroke all the way through, and Ship doesn't know anything about the program running in the pane.

> how the hell are we going to be able to trace a key through the input and back out the output in a way that keeps the correct dumb pipe architecture where ship works no matter what program is running in the tty?

So Claude built a trace that watches from the outside. The program in the pane stamps each read and write, the window and the server mark the edges they own, and a script stitches all three into one timeline. That took its own detours. The idea of tagging each key with an ID died right away because all input goes over one long request, and the trace tool writes every process as process 1, so the files had to be renumbered before they'd line up. Ship stays a dumb pipe with timing only at the edges it owns, so the tracing added three lines to the code.

| µs after keypress           |      |
| --------------------------- | ---- |
| window sends it             | 71   |
| server writes it to the pty | 326  |
| program reads it            | 404  |
| program writes the echo     | 456  |
| Ghostty has parsed it       | 499  |
| screen published            | 552  |
| window receives it          | 1141 |
| window starts drawing       | 1152 |
| window done drawing         | 1790 |
| your terminal has it        | 1859 |

The work in there is small, about 5 µs to parse the echo, 4 to publish, 13 to encode, 20 to decode and 7 to apply. Most of the rest is time spent waiting for the next thread to get to it, including 369 µs between publishing and the stream to the window noticing.

For the efficiency core theory, Claude wanted to tell macOS to treat Ship's threads as user-interactive.

> does that actually help at all? i guess we can give it a shot, but i don't expect much

That bumped their priority from 31 to 37. Idle typing went from 1.58 ms to 1.51 ms, which is noise, so that got reverted. Running the window on a single thread came up again too.

> doesn't help. i already tried it before

> so seems like we're optimal-ish?

Pretty much. What's left is the design itself, a separate server and window talking over HTTP. That's the part that lets you close the window and keep working, or attach from another machine, and I'll take that over half a millisecond.

## 12. The final race

The last benchmark ran all four, tmux included this time.

> never used tmux before so i had to install it just now. let's give it a go and see!

Five rounds, 180 runs in shuffled order, on an M4 Max.

| medians                   | Ship         | Herdr      | Zellij  | tmux        |
| ------------------------- | ------------ | ---------- | ------- | ----------- |
| log flood                 | **198 MB/s** | 126 MB/s   | 40 MB/s | 40 MB/s     |
| wide-text flood           | **139 MB/s** | 85 MB/s    | 56 MB/s | 9 MB/s      |
| animation CPU             | **29%**      | 41%        | 65%     | 45%         |
| animation fps             | **60**       | 53         | **60**  | 48          |
| CPU while typing in flood | **82%**      | 102%       | 206%    | 100%        |
| typing during a flood     | **0.36 ms**  | 0.57 ms    | 8.5 ms  | 0.53 ms     |
| typing during a flood p99 | 8.3 ms       | **1.4 ms** | 12.5 ms | 9.0 ms      |
| typing (idle)             | 1.66 ms      | 0.64 ms    | 13.4 ms | **0.33 ms** |
| agent screen CPU          | 1.7%         | 1.2%       | 1.3%    | **0.25%**   |
| memory (idle)             | 35 MB        | 48 MB      | 108 MB  | **10 MB**   |

When there's a lot going on, Ship gets through it faster and with less CPU than the rest. The animation is a full-screen truecolor gradient, which is about as hard on compression as a normal screen gets.

> when it came to the anim test, what was the methodology for that? was it with gradients and we STILL won by THAT much?

Herdr also uses Ghostty, so the gap in the animation row comes from what happens after parsing. When not much is happening, Ship uses a bit more CPU and a quiet keystroke takes a bit longer, and you can see where that goes if you split the keystroke in half.

| typing (idle), median | key in  | program | screen out |
| --------------------- | ------- | ------- | ---------- |
| Ship                  | 0.31 ms | 0.04 ms | 1.31 ms    |
| Herdr                 | 0.28 ms | 0.05 ms | 0.30 ms    |
| tmux                  | 0.13 ms | 0.05 ms | 0.15 ms    |

Getting the key in is about the same for everyone, and it's the way back out, through the server, the stream and the full redraw, that costs Ship its millisecond.

> the thing is that to me, this feels like the right trade off. because on light workloads, you won't notice 1-2% difference in cpu utilization, but on heavy ones, you will certainly notice the drastic performance drop off.

The slowest 1% while typing beside a flood is the one loss I can't explain yet.

### A screen that never repeats

All of the above leans on screens repeating themselves, so I added a workload that doesn't, every cell a random color 60 times a second.

| over the wire or to the terminal | Ship over the wire | tmux to the terminal |
| -------------------------------- | ------------------ | -------------------- |
| 16 MiB log flood                 | 15 KB total        | 520 KB total         |
| a keystroke in a quiet pane      | 18 B               | 11 B                 |
| normal animation                 | 110 KB/s           | 8.8 MB/s             |
| noise                            | 2.9 MB/s           | 8.8 MB/s             |

Ship sends a lot more for noise, but still about a third of what tmux writes, and that's the number that matters if you use either over ssh, because Ship's window can already connect to a server on another machine through an ssh tunnel, and with tmux everything it draws goes through ssh as is.

> [!TIP]
> To try that, forward the server's port and point `ship` at it from another terminal. [Using Ship on another machine](/docs/windows-and-the-server/#using-ship-on-another-machine) has the details.
>
> ```sh
> ssh -N -L 43179:127.0.0.1:43179 otherbox
> ship --server-url http://127.0.0.1:43179
> ```

## What it came down to

The part I was most worried about, sending the whole screen every time, held up the whole way and never needed fixing. Every win came from waste around it, a timer that rounded up, copies nobody looked at, a thread woken 91,000 times a second, a redraw that landed right as your letter did and 256 KiB of stack per thread that nobody used, and most of those fixes were a few lines. Ship is still about a millisecond behind Herdr on a quiet keystroke, and after seeing what it takes to close that, I'm fine with it.

These numbers are from my laptop, and the benchmark is in [`benchmarks/`](https://github.com/shipmux/ship/tree/main/benchmarks) if you want to run it on yours.

> [!NOTE]
> It needs [uv](https://docs.astral.sh/uv/) and a release build of Ship, and a full run takes about half an hour. Leave out anything you don't have installed with `--mux`.
>
> ```sh
> cargo build --release
> cd benchmarks && uv run bench.py --mux ship tmux
> ```
