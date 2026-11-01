// The hold's mouse demo: drag a border, split from a pane's right-click menu, and drag a tab to a new place
// in the sidebar. VHS has no mouse, so this drives the same terminal VHS uses (ttyd running xterm.js) with
// Playwright's real mouse, and records the page. A drawn pointer follows the mouse, since a recording shows
// none. No agents: record it with `record.sh mouse.mjs --shell`, which runs this in place of VHS.
//
//   node mouse.mjs <out.webm>
//
// Expects what record.sh exports (DEMO_PROJECT, DEMO_WORLD, the demo server's name, XDG dirs) and ttyd,
// ffmpeg and Playwright's Chromium.
import { createRequire } from "node:module";
import { spawn } from "node:child_process";
import { mkdtempSync, readFileSync, readdirSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { execFileSync } from "node:child_process";

// required rather than imported, so a Playwright installed globally (NODE_PATH) works as well as this
// folder's own (package.json, which CI installs)
const { chromium } = createRequire(import.meta.url)("playwright");
const out = process.argv[2];
if (!out) throw new Error("usage: node mouse.mjs <out.webm>");
const here = dirname(fileURLToPath(import.meta.url));
const theme = JSON.parse(/Set Theme (\{.*\})/.exec(readFileSync(join(here, "ship16.tape"), "utf8"))[1]);
const port = 43333;
const W = 1920;
const H = 1080;

// the same terminal VHS records: ttyd and xterm.js, JetBrains Mono at 30px, the site's palette. Bound to
// loopback only: it is a writable shell
const ttyd = spawn(
  "ttyd",
  [
    "-i",
    "lo",
    "-p",
    String(port),
    "-W",
    "-t",
    "fontFamily=JetBrains Mono",
    "-t",
    "fontSize=30",
    "-t",
    `theme=${JSON.stringify(theme)}`,
    "-t",
    "disableLeaveAlert=true",
    "-t",
    "disableResizeOverlay=true",
    "bash",
    "--norc",
    "--noprofile",
  ],
  { stdio: "ignore" },
);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
await sleep(800);

const videoDir = mkdtempSync(join(tmpdir(), "ship-mouse-"));
const browser = await chromium.launch();
const context = await browser.newContext({
  viewport: { width: W, height: H },
  recordVideo: { dir: videoDir, size: { width: W, height: H } },
});
const page = await context.newPage();
const t0 = Date.now();
await page.goto(`http://127.0.0.1:${port}/`);
await page.waitForFunction(() => window.term && window.term.cols > 0);
await page.addStyleTag({
  content: `html, body { margin: 0; background: ${theme.background}; } #terminal-container { padding: 18px; box-sizing: border-box; }
  #pointer { position: fixed; z-index: 99999; width: 42px; height: 60px; pointer-events: none; left: -50px; top: -50px;
    background: url("data:image/svg+xml,${encodeURIComponent(
      '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 7 10" shape-rendering="crispEdges"><path fill="#1c1a27" d="M0 0h1v10H0zM1 1h1v1H1zM2 2h1v1H2zM3 3h1v1H3zM4 4h1v1H4zM5 5h1v1H5zM6 6h1v1H6zM4 7h3v1H4zM1 8h1v1H1zM2 7h1v1H2zM3 8h1v1H3zM4 8h1v2H4zM3 9h1v1H3z"/><path fill="#fbf8f1" d="M1 2h1v6H1zM2 3h1v4H2zM3 4h1v4H3zM4 5h1v2H4zM5 6h1v1H5z"/></svg>',
    )}") no-repeat; background-size: 100% 100%; }`,
});
await page.evaluate(() => {
  const p = Object.assign(document.createElement("div"), { id: "pointer" });
  document.body.append(p);
  addEventListener("mousemove", (e) => Object.assign(p.style, { left: e.clientX + "px", top: e.clientY + "px" }), true);
});
await page.evaluate(() => window.dispatchEvent(new Event("resize")));
await sleep(500);

const type = async (text) => {
  await page.keyboard.type(text, { delay: 5 });
  await page.keyboard.press("Enter");
};
const screen = () =>
  page.evaluate(() => {
    const b = window.term.buffer.active;
    const rows = [];
    for (let y = 0; y < window.term.rows; y++) rows.push(b.getLine(b.viewportY + y).translateToString(true));
    return rows;
  });
const waitFor = async (re, ms = 20000) => {
  const end = Date.now() + ms;
  while (Date.now() < end) {
    if ((await screen()).some((r) => re.test(r))) return;
    await sleep(100);
  }
  await shot("timeout");
  throw new Error(`timed out waiting for ${re}`);
};
// the middle of a cell, in page pixels
const cell = async (col, row) =>
  page.evaluate(
    ([c, r]) => {
      const s = document.querySelector(".xterm-screen").getBoundingClientRect();
      const w = s.width / window.term.cols;
      const h = s.height / window.term.rows;
      return { x: s.left + (c + 0.5) * w, y: s.top + (r + 0.5) * h };
    },
    [col, row],
  );
const find = async (text, from = 0) => {
  const rows = await screen();
  for (let y = 0; y < rows.length; y++) {
    const x = rows[y].indexOf(text, y === 0 ? from : 0);
    if (x >= 0) return { col: x, row: y };
  }
  throw new Error(`no "${text}" on screen`);
};
// MOUSE_DEBUG=<dir> saves a screenshot at each step, for tuning
let shots = 0;
const shot = async (name) =>
  process.env.MOUSE_DEBUG && page.screenshot({ path: join(process.env.MOUSE_DEBUG, `${++shots}-${name}.png`) });
const glide = async (to, steps = 24) => {
  await page.mouse.move(to.x, to.y, { steps });
};

// set up off camera: the shell, the world, and the window on the api tab's split
await page.click(".xterm-screen");
await type(`cd "$DEMO_PROJECT" && PS1="\\W $ " && set +m && source "$DEMO_WORLD" && clear`);
await type(
  `(until A=$(ship view list | jq -r 'keys[0] // empty') && [ -n "$A" ]; do sleep 0.1; done; ship view select --attachment "$A" "$API_PANE") >/dev/null 2>&1 &`,
);
await type("clear && ship");
await waitFor(/agents/);
await sleep(1200);
const shownAt = Date.now() - t0;
await page.mouse.move(W * 0.62, H * 0.7);
await sleep(900);

// 1. drag the border between the api tab's two panes to the right
const rows = await screen();
const mid = Math.floor(rows.length / 2);
// the row's vertical lines: the sidebar's edge, then each pane's sides (│ or ┃, the selected pane's). The
// split is the one pane edge that isn't the first or the last
const bars = [...rows[mid]].map((ch, i) => (/[│┃]/.test(ch) ? i : -1)).filter((i) => i >= 0);
const border = bars[bars.length - 2];
let at = await cell(border, mid);
await glide(at);
await sleep(300);
await page.mouse.down();
await glide(await cell(border + 14, mid), 30);
await shot("dragging-border");
await page.mouse.up();
await sleep(1200);
await shot("border-dropped");

// 2. right-click the right-hand pane and split it down from its menu
await glide(await cell(border + 30, mid + 4));
await sleep(300);
await page.mouse.click((await cell(border + 30, mid + 4)).x, (await cell(border + 30, mid + 4)).y, { button: "right" });
await sleep(500);
await shot("right-clicked");
await waitFor(/split down/i);
await sleep(700);
const item = await find("split down");
await glide(await cell(item.col + 3, item.row), 16);
await sleep(500);
await page.mouse.click((await cell(item.col + 3, item.row)).x, (await cell(item.col + 3, item.row)).y);
await sleep(1500);

// 3. drag the blog tab up in the sidebar, to just under ship
const blog = await find("blog");
const docs = await find("docs");
await glide(await cell(blog.col + 1, blog.row));
await sleep(300);
await page.mouse.down();
await glide(await cell(blog.col + 1, docs.row), 30);
await sleep(600);
await page.mouse.up();
await sleep(2000);
const endAt = Date.now() - t0;

await context.close();
await browser.close();
ttyd.kill();
const raw = join(videoDir, readdirSync(videoDir)[0]);
// keep the part after setup: the window, then the three moves
execFileSync("ffmpeg", [
  "-loglevel",
  "error",
  "-y",
  "-ss",
  String(shownAt / 1000),
  "-to",
  String(endAt / 1000),
  "-i",
  raw,
  "-c:v",
  "libvpx-vp9",
  "-b:v",
  "0",
  "-crf",
  "34",
  "-an",
  out,
]);
rmSync(videoDir, { recursive: true, force: true });
