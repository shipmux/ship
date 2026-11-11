//! Throwaway spike: per-frame cost of carrying a base64 image inside every
//! pane screen frame on the attach stream. See README.md and results.md.

use std::{
    borrow::Cow,
    fs::File,
    io::{BufReader, Write},
    path::Path,
    sync::Arc,
    time::Instant,
};

use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use ship_core::{
    id::Id,
    model::Pane,
    protocol::{PaneScreen, SseEvent},
    screen::{Attr, Color, Cursor, CursorShape, Row, Run, Screen, Size},
};

const FRAMES: usize = 300;
const WARMUP: usize = 12;

// ---------------------------------------------------------------- wire types

/// `D` is `Arc<str>` on the server (shared, serialized without a copy) and
/// `String` on the client (what deserialization produces).
#[derive(Serialize, Deserialize)]
struct ImageB64<D> {
    id: u32,
    generation: u64,
    format: Cow<'static, str>,
    width: u32,
    height: u32,
    data: D,
}

#[derive(Serialize, Deserialize)]
struct Frame<D> {
    event: SseEvent,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    images: Vec<ImageB64<D>>,
}

// ------------------------------------------------------------------- screens

fn style_for(tok: &str, in_comment: bool, in_str: bool) -> (Color, Vec<Attr>) {
    if in_comment {
        return (Color::Rgb(0x6c, 0x70, 0x86), vec![Attr::Italic]);
    }
    if in_str {
        return (Color::Rgb(0xa6, 0xe3, 0xa1), vec![]);
    }
    match tok {
        "fn" | "let" | "pub" | "use" | "struct" | "impl" | "match" | "if" | "else" | "for"
        | "in" | "return" | "mut" | "enum" | "async" | "await" => {
            (Color::Rgb(0xcb, 0xa6, 0xf7), vec![Attr::Bold])
        }
        "String" | "Vec" | "Option" | "Result" | "Self" | "Arc" | "Screen" | "Row" | "Run" => {
            (Color::Rgb(0xf9, 0xe2, 0xaf), vec![])
        }
        t if t.chars().all(|c| c.is_ascii_digit()) && !t.is_empty() => {
            (Color::Rgb(0xfa, 0xb3, 0x87), vec![])
        }
        t if t.chars().all(|c| !c.is_alphanumeric() && c != '_') => {
            (Color::Rgb(0x89, 0xdc, 0xeb), vec![])
        }
        _ => (Color::Default, vec![]),
    }
}

fn runs_for(text: &str) -> Vec<Run> {
    // Tokenize: words, punctuation, whitespace. Track comment and string state.
    let mut spans: Vec<(usize, Color, Vec<Attr>)> = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut in_comment = false;
    let mut in_str = false;
    while i < chars.len() {
        let c = chars[i];
        let start = i;
        let tok: String;
        if c == '/' && chars.get(i + 1) == Some(&'/') {
            in_comment = true;
        }
        if c.is_alphanumeric() || c == '_' {
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
        } else {
            i += 1;
        }
        tok = chars[start..i].iter().collect();
        if c == '"' {
            in_str = !in_str;
            let (fg, attrs) = style_for(&tok, in_comment, true);
            spans.push((i - start, fg, attrs));
            continue;
        }
        let (fg, attrs) = style_for(&tok, in_comment, in_str);
        spans.push((i - start, fg, attrs));
    }
    // Merge adjacent equal styles.
    let mut runs: Vec<Run> = Vec::new();
    for (cells, fg, attrs) in spans {
        if let Some(last) = runs.last_mut()
            && last.fg == fg
            && last.attrs == attrs
        {
            last.cells += cells as u16;
            continue;
        }
        runs.push(Run {
            cells: cells as u16,
            fg,
            bg: Color::Default,
            underline_color: Color::Default,
            attrs,
        });
    }
    runs
}

const CODE: &[&str] = &[
    "use std::{collections::HashMap, sync::Arc};",
    "",
    "/// A pane's visible grid at one moment. Full snapshot; zstd on the stream",
    "/// supplies the savings between frames.",
    "#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]",
    "pub struct Screen {",
    "    pub size: Size,",
    "    pub rows: Vec<Row>,",
    "    pub cursor: Option<Cursor>,",
    "    pub mouse: bool,",
    "}",
    "",
    "impl Screen {",
    "    pub fn blank(size: Size) -> Self {",
    "        let rows = (0..size.rows).map(|_| Row::default()).collect();",
    "        Self { size, rows, cursor: None, mouse: false }",
    "    }",
    "",
    "    pub fn set_text(&mut self, y: usize, text: &str) -> Result<(), String> {",
    "        let Some(row) = self.rows.get_mut(y) else {",
    "            return Err(format!(\"row {} out of range for {} rows\", y, self.size.rows));",
    "        };",
    "        row.text = text.to_owned();",
    "        row.runs.clear(); // restyled by the caller",
    "        Ok(())",
    "    }",
    "",
    "    pub async fn publish(&self, tx: &Sender<Arc<Screen>>) {",
    "        for attempt in 0..3 {",
    "            match tx.send(Arc::new(self.clone())).await {",
    "                Ok(()) => return,",
    "                Err(error) => tracing::warn!(attempt, %error, \"screen dropped\"),",
    "            }",
    "        }",
    "    }",
    "}",
    "",
    "fn color_of(value: u32) -> Color {",
    "    if value < 16 { Color::Indexed(value as u8) } else { Color::Rgb(0, 0, 0) }",
    "}",
];

fn row_from(text: String, runs: Vec<Run>) -> Row {
    Row {
        text,
        runs,
        multi: vec![],
    }
}

/// Frame `n` of a pretend typing session: a character changes in one line, the
/// cursor moves, and the status line shows the position.
fn screen_at(n: usize) -> Screen {
    let (cols, rows) = (200usize, 50usize);
    let mut out: Vec<Row> = Vec::with_capacity(rows);
    let line_nr = Color::Rgb(0x58, 0x5b, 0x70);
    for y in 0..rows - 2 {
        let code = CODE[y % CODE.len()];
        let mut code = code.to_owned();
        // Typing: the line under the cursor grows by one char per couple of frames.
        let edit_row = 20usize;
        if y == edit_row {
            let typed = "let value = compute_something(a, b);";
            let k = (n / 2) % (typed.len() + 1);
            code = format!("    {}", &typed[..k]);
        }
        // A second spot flickers (diagnostic marker) on odd frames.
        let marker = if y == 30 && n % 2 == 1 { 'E' } else { ' ' };
        let gutter = format!("{marker}{:>4} ", y + 1);
        let text = format!("{gutter}{code}");
        let mut runs = vec![Run {
            cells: gutter.chars().count() as u16,
            fg: line_nr,
            bg: Color::Default,
            underline_color: Color::Default,
            attrs: vec![],
        }];
        runs.extend(runs_for(&code));
        out.push(row_from(text.trim_end().to_owned(), runs));
    }
    // Command line row and a full-width status line.
    let status_text = format!(
        " NORMAL  src/screen.rs [+]{}{:>3}:{:<3} {:>3}% ",
        " ".repeat(cols - 50),
        21,
        6 + (n / 2) % 36,
        42
    );
    let status_text: String = status_text.chars().take(cols).collect();
    out.push(row_from(
        status_text.clone(),
        vec![
            Run {
                cells: 8,
                fg: Color::Rgb(0x1e, 0x1e, 0x2e),
                bg: Color::Rgb(0x89, 0xb4, 0xfa),
                underline_color: Color::Default,
                attrs: vec![Attr::Bold],
            },
            Run {
                cells: (cols - 8) as u16,
                fg: Color::Rgb(0xcd, 0xd6, 0xf4),
                bg: Color::Rgb(0x31, 0x32, 0x44),
                underline_color: Color::Default,
                attrs: vec![],
            },
        ],
    ));
    out.push(row_from(format!(":w{}", if n % 5 == 0 { "q" } else { "" }), vec![]));
    Screen {
        size: Size {
            cols: cols as u16,
            rows: rows as u16,
        },
        rows: out,
        cursor: Some(Cursor {
            x: (11 + (n / 2) % 36) as u16,
            y: 20,
            shape: CursorShape::Block,
            blinking: false,
        }),
        scrolled: None,
        mouse: true,
    }
}

// -------------------------------------------------------------------- images

struct Img {
    w: u32,
    h: u32,
    rgba: Vec<u8>,
}

fn load_rgba(path: &Path) -> Img {
    let decoder = png::Decoder::new(BufReader::new(File::open(path).expect("open png")));
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    buf.truncate(info.buffer_size());
    let (w, h) = (info.width, info.height);
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::Rgb => buf
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        other => panic!("unsupported {other:?}"),
    };
    Img {
        w,
        h,
        rgba,
    }
}

fn crop(img: &Img, x: u32, y: u32, w: u32, h: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity((w * h * 4) as usize);
    for row in y..y + h {
        let s = ((row * img.w + x) * 4) as usize;
        out.extend_from_slice(&img.rgba[s..s + (w * 4) as usize]);
    }
    out
}

/// Smooth gradients plus per-pixel noise, roughly photo-like entropy.
fn noisy(w: u32, h: u32, seed: u64) -> Vec<u8> {
    let mut s = seed | 1;
    let mut next = move || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        s
    };
    let mut out = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let fx = x as f32 / w as f32;
            let fy = y as f32 / h as f32;
            let base = [
                80.0 + 120.0 * fx + 30.0 * (fy * 9.0).sin(),
                100.0 + 90.0 * fy + 25.0 * (fx * 7.0).cos(),
                140.0 + 60.0 * (fx + fy) * 0.5 + 20.0 * ((fx * 11.0 + fy * 5.0).sin()),
            ];
            let r = next();
            for (c, b) in base.iter().enumerate() {
                let n = ((r >> (c * 8)) & 0xff) as f32 / 255.0 * 24.0 - 12.0;
                out.push((b + n).clamp(0.0, 255.0) as u8);
            }
            out.push(255);
        }
    }
    out
}

fn encode_png(rgba: &[u8], w: u32, h: u32, compression: png::Compression) -> Vec<u8> {
    let mut out = Vec::new();
    let mut enc = png::Encoder::new(&mut out, w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.set_compression(compression);
    let mut w = enc.write_header().unwrap();
    w.write_image_data(rgba).unwrap();
    w.finish().unwrap();
    out
}

struct Case {
    label: String,
    desc: String,
    w: u32,
    h: u32,
    rgba: Vec<u8>,
}

fn pct(v: &mut [f64], p: f64) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let idx = ((v.len() as f64 - 1.0) * p).round() as usize;
    v[idx]
}
fn median(v: &[f64]) -> f64 {
    pct(&mut v.to_vec(), 0.5)
}
fn p99(v: &[f64]) -> f64 {
    pct(&mut v.to_vec(), 0.99)
}
fn us(a: Instant, b: Instant) -> f64 {
    b.duration_since(a).as_secs_f64() * 1e6
}

#[derive(Clone)]
enum Variant {
    A,
    B(Arc<str>, u32, u32),
    C(Arc<str>, u32, u32, &'static str),
    D(Arc<str>, u32, u32),
}

impl Variant {
    fn images(&self, frame: usize) -> Vec<ImageB64<Arc<str>>> {
        let mk = |data: &Arc<str>, w, h, fmt: &'static str| {
            vec![ImageB64 {
                id: 7,
                generation: 1,
                format: Cow::Borrowed(fmt),
                width: w,
                height: h,
                data: data.clone(),
            }]
        };
        match self {
            Variant::A => vec![],
            Variant::B(d, w, h) => mk(d, *w, *h, "rgba"),
            Variant::C(d, w, h, _) => mk(d, *w, *h, "png"),
            Variant::D(d, w, h) => {
                if frame == 0 {
                    mk(d, *w, *h, "png")
                } else {
                    vec![]
                }
            }
        }
    }
}

#[derive(Default)]
struct Stats {
    ser: Vec<f64>,
    zstd: Vec<f64>,
    dezstd: Vec<f64>,
    split: Vec<f64>,
    parse: Vec<f64>,
    total: Vec<f64>,
    wire: Vec<usize>,
    json: Vec<usize>,
}

fn run(
    variant: &Variant,
    screens: &[Arc<Screen>],
    pane: &<Pane as ship_core::id::Identified>::Id,
    window_log: Option<u32>,
    warm: bool,
) -> Stats {
    let level = 3;
    let mut enc = zstd::stream::write::Encoder::new(Vec::new(), level).unwrap();
    if let Some(wl) = window_log {
        enc.set_parameter(zstd::zstd_safe::CParameter::WindowLog(wl))
            .unwrap();
    }
    let mut dec = zstd::stream::write::Decoder::new(Vec::new()).unwrap();
    let mut st = Stats::default();
    let n = if warm { WARMUP } else { screens.len() };
    for i in 0..n {
        let screen = screens[i].clone();
        let frame = Frame {
            event: SseEvent::Screen(PaneScreen {
                pane: pane.clone(),
                screen,
            }),
            images: variant.images(i),
        };
        // Server.
        let t0 = Instant::now();
        let json = serde_json::to_string(&frame).unwrap();
        let t1 = Instant::now();
        enc.write_all(b"data: ").unwrap();
        enc.write_all(json.as_bytes()).unwrap();
        enc.write_all(b"\n\n").unwrap();
        enc.flush().unwrap();
        let t2 = Instant::now();
        let wire = std::mem::take(enc.get_mut());
        // Client.
        let t3 = Instant::now();
        dec.write_all(&wire).unwrap();
        dec.flush().unwrap();
        let t4 = Instant::now();
        let decoded = dec.get_ref();
        let end = memchr::memchr(b'\n', decoded).expect("newline");
        let line = &decoded[..end];
        let payload = std::str::from_utf8(line)
            .unwrap()
            .strip_prefix("data: ")
            .expect("data prefix");
        let t5 = Instant::now();
        let back: Frame<String> = serde_json::from_str(payload).unwrap();
        let t6 = Instant::now();

        // Round-trip check (untimed).
        let SseEvent::Screen(ps) = &back.event else {
            panic!("not a screen event")
        };
        assert_eq!(*ps.screen, *screens[i], "screen mismatch at frame {i}");
        let expect = variant.images(i);
        assert_eq!(back.images.len(), expect.len(), "image count at frame {i}");
        for (b, e) in back.images.iter().zip(&expect) {
            assert!(b.id == e.id && b.generation == e.generation && b.format == e.format);
            assert!(b.width == e.width && b.height == e.height);
            assert!(b.data.as_str() == &*e.data, "image bytes differ at frame {i}");
        }
        let json_len = json.len();
        drop(back);
        dec.get_mut().clear();

        if !warm {
            let (ser, zs, dz, sp, pa) = (us(t0, t1), us(t1, t2), us(t3, t4), us(t4, t5), us(t5, t6));
            st.ser.push(ser);
            st.zstd.push(zs);
            st.dezstd.push(dz);
            st.split.push(sp);
            st.parse.push(pa);
            st.total.push(ser + zs + dz + sp + pa);
            st.wire.push(wire.len());
            st.json.push(json_len);
        }
    }
    st
}

fn med_usize(v: &[usize]) -> usize {
    let mut f: Vec<f64> = v.iter().map(|x| *x as f64).collect();
    pct(&mut f, 0.5) as usize
}

struct Row2 {
    name: String,
    server: f64,
    client: f64,
    total: f64,
    p99: f64,
    stats: Stats,
}

fn measure(name: &str, v: &Variant, screens: &[Arc<Screen>], pane: &<Pane as ship_core::id::Identified>::Id, wl: Option<u32>) -> Row2 {
    // Warm up the process on a throwaway stream, then measure on a fresh one.
    run(v, screens, pane, wl, true);
    let st = run(v, screens, pane, wl, false);
    Row2 {
        name: name.to_owned(),
        server: median(&st.ser) + median(&st.zstd),
        client: median(&st.dezstd) + median(&st.split) + median(&st.parse),
        total: median(&st.total),
        p99: p99(&st.total),
        stats: st,
    }
}

fn print_table(title: &str, rows: &[Row2]) {
    let base = rows.first().map(|r| (r.total, r.p99));
    println!("\n### {title}\n");
    println!("| variant | server us (ser + zstd) | client us (unzstd + split + parse) | total median us | added vs A median us | total p99 us | added vs A p99 us | total mean us | wire B/frame median | wire B/frame mean | first frame total us | wire B first frame | wire B max after first | frames after first with wire > 1 KB | frames after first with wire > 100 KB | JSON B |");
    println!("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|");
    for r in rows {
        let (ab, ap) = base.unwrap();
        let s = &r.stats;
        println!(
            "| {} | {:.0} ({:.0} + {:.0}) | {:.0} ({:.0} + {:.0} + {:.0}) | {:.0} | {:+.0} | {:.0} | {:+.0} | {:.0} | {} | {} | {:.0} | {} | {} | {} | {} | {} |",
            r.name,
            r.server,
            median(&s.ser),
            median(&s.zstd),
            r.client,
            median(&s.dezstd),
            median(&s.split),
            median(&s.parse),
            r.total,
            r.total - ab,
            r.p99,
            r.p99 - ap,
            s.total.iter().sum::<f64>() / s.total.len() as f64,
            med_usize(&s.wire),
            s.wire.iter().sum::<usize>() / s.wire.len(),
            s.total[0],
            s.wire[0],
            s.wire[1..].iter().max().unwrap(),
            s.wire[1..].iter().filter(|w| **w > 1024).count(),
            s.wire[1..].iter().filter(|w| **w > 102400).count(),
            med_usize(&s.json),
        );
    }
}

fn time_median<F: FnMut()>(reps: usize, mut f: F) -> f64 {
    f(); // warm
    let mut v = Vec::new();
    for _ in 0..reps {
        let t = Instant::now();
        f();
        v.push(t.elapsed().as_secs_f64() * 1e6);
    }
    median(&v)
}

fn main() {
    let root = std::env::var("SHIP_ROOT").unwrap_or_else(|_| "../..".into());
    let root = Path::new(&root);
    let landing = load_rgba(&root.join("website/directions/ansi/screenshots/landing-1440.png"));
    let og = load_rgba(&root.join("website/public/og.png"));
    println!("# image-frames bench");
    println!("landing {}x{}, og {}x{}", landing.w, landing.h, og.w, og.h);

    // Crops chosen to avoid big flat-color areas where possible.
    let mut cases: Vec<Case> = Vec::new();
    cases.push(Case {
        label: "small".into(),
        desc: "64x64 crop of og.png (pixel-art sign)".into(),
        w: 64,
        h: 64,
        rgba: crop(&og, 790, 330, 64, 64),
    });
    cases.push(Case {
        label: "medium-shot".into(),
        desc: "800x600 crop of landing-1440.png at (320, 600)".into(),
        w: 800,
        h: 600,
        rgba: crop(&landing, 320, 600, 800, 600),
    });
    cases.push(Case {
        label: "medium-noise".into(),
        desc: "800x600 synthetic gradient + noise".into(),
        w: 800,
        h: 600,
        rgba: noisy(800, 600, 0x9e3779b97f4a7c15),
    });
    // Large: two screenshot crops side by side, 1440x1080 + 480x1080.
    {
        let a = crop(&landing, 0, 1200, 1440, 1080);
        let b = crop(&landing, 0, 3400, 480, 1080);
        let mut rgba = Vec::with_capacity(1920 * 1080 * 4);
        for y in 0..1080usize {
            rgba.extend_from_slice(&a[y * 1440 * 4..(y + 1) * 1440 * 4]);
            rgba.extend_from_slice(&b[y * 480 * 4..(y + 1) * 480 * 4]);
        }
        cases.push(Case {
            label: "large-shot".into(),
            desc: "1920x1080: landing-1440.png crops at y=1200 and y=3400 side by side".into(),
            w: 1920,
            h: 1080,
            rgba,
        });
    }
    cases.push(Case {
        label: "large-noise".into(),
        desc: "1920x1080 synthetic gradient + noise".into(),
        w: 1920,
        h: 1080,
        rgba: noisy(1920, 1080, 0xdeadbeefcafef00d),
    });

    if std::env::var("DUMP").is_ok() {
        for c in &cases {
            let p = encode_png(&c.rgba, c.w, c.h, png::Compression::Fast);
            std::fs::write(format!("/tmp/dump-{}.png", c.label), p).unwrap();
        }
    }

    let only = std::env::var("ONLY").ok();
    let screens: Vec<Arc<Screen>> = (0..FRAMES).map(|n| Arc::new(screen_at(n))).collect();
    let pane = Id::<Pane>::random();
    {
        let j = serde_json::to_string(&screens[0]).unwrap();
        println!("screen JSON (A, screen only): {} bytes, {} rows x {} cols", j.len(), screens[0].rows.len(), screens[0].size.cols);
    }

    // One-time costs.
    println!("\n## One-time encode costs (median of reps, us)\n");
    println!("| image | dims | RGBA B | PNG fast B | PNG default B | PNG fast us | PNG default us | b64(RGBA) us | b64(PNG fast) us | b64 PNG fast chars |");
    println!("|---|---|---|---|---|---|---|---|---|---|");
    struct Prepared {
        raw_b64: Arc<str>,
        fast_b64: Arc<str>,
        def_b64: Arc<str>,
    }
    let mut prepared = Vec::new();
    for c in &cases {
        let reps = if c.w * c.h > 1_000_000 { 5 } else { 15 };
        let fast = encode_png(&c.rgba, c.w, c.h, png::Compression::Fast);
        let def = encode_png(&c.rgba, c.w, c.h, png::Compression::Balanced);
        let t_fast = time_median(reps, || {
            std::hint::black_box(encode_png(&c.rgba, c.w, c.h, png::Compression::Fast));
        });
        let t_def = time_median(reps, || {
            std::hint::black_box(encode_png(&c.rgba, c.w, c.h, png::Compression::Balanced));
        });
        let t_b64_raw = time_median(reps, || {
            std::hint::black_box(STANDARD.encode(&c.rgba));
        });
        let t_b64_png = time_median(reps, || {
            std::hint::black_box(STANDARD.encode(&fast));
        });
        let fast_b64: Arc<str> = STANDARD.encode(&fast).into();
        let def_b64: Arc<str> = STANDARD.encode(&def).into();
        let raw_b64: Arc<str> = STANDARD.encode(&c.rgba).into();
        println!(
            "| {} | {}x{} | {} | {} | {} | {:.0} | {:.0} | {:.0} | {:.0} | {} |",
            c.label, c.w, c.h, c.rgba.len(), fast.len(), def.len(), t_fast, t_def, t_b64_raw, t_b64_png, fast_b64.len()
        );
        prepared.push(Prepared { raw_b64, fast_b64, def_b64 });
    }
    println!();
    for c in &cases {
        println!("- {}: {}", c.label, c.desc);
    }

    for (c, p) in cases.iter().zip(&prepared) {
        if let Some(o) = &only
            && !c.label.contains(o.as_str())
        {
            continue;
        }
        let (w, h) = (c.w, c.h);
        let rows = vec![
            measure("A baseline", &Variant::A, &screens, &pane, None),
            measure("B raw RGBA b64 every frame", &Variant::B(p.raw_b64.clone(), w, h), &screens, &pane, None),
            measure("C PNG(fast) b64 every frame", &Variant::C(p.fast_b64.clone(), w, h, "png"), &screens, &pane, None),
            measure("C' PNG(default) b64 every frame", &Variant::C(p.def_b64.clone(), w, h, "png"), &screens, &pane, None),
            measure("D image once (PNG fast)", &Variant::D(p.fast_b64.clone(), w, h), &screens, &pane, None),
        ];
        print_table(&format!("{} ({}x{}), zstd level 3, default window", c.label, c.w, c.h), &rows);

        // Window side note for the big ones.
        if c.w * c.h > 1_000_000 {
            let wl = 24;
            let rows = vec![
                measure("A baseline", &Variant::A, &screens, &pane, Some(wl)),
                measure("C PNG(fast) every frame", &Variant::C(p.fast_b64.clone(), w, h, "png"), &screens, &pane, Some(wl)),
            ];
            print_table(&format!("{} side note: level 3 with window_log {wl} (16 MiB)", c.label), &rows);
        }
    }
}
