// Turns docs/blog into blog posts for Starlight. Runs after prepare-docs.mjs, which clears src/content/docs/.
//
// Sources stay plain GitHub markdown named YYYY-MM-DD-slug.md: the date comes from the filename and the
// title from the `# Title` line, and the paragraph after the title is the post's preview in the post list, so
// nothing in the source needs frontmatter or an excerpt marker. Posts are written to
// src/content/docs/blog/<slug>.md, outside the versioned docs prefix.
//
// The date is written as noon UTC: starlight-blog formats it in the build machine's time zone, and noon
// stays the same calendar day from UTC-12 to UTC+12.
//
// Only the main build writes posts (DOCS_CHANNEL main, the default); the blog is not versioned.
// usage: node scripts/prepare-blog.mjs [blog-dir]
import { readFileSync, writeFileSync, mkdirSync, readdirSync, existsSync } from "node:fs";
import { join, resolve } from "node:path";

const site = resolve(import.meta.dirname, "..");
const src = resolve(process.argv[2] ?? join(site, "../docs/blog"));
const out = join(site, "src/content/docs/blog");
const NAME = /^(\d{4}-\d{2}-\d{2})-([a-z0-9-]+)\.md$/;

if ((process.env.DOCS_CHANNEL ?? "main") !== "main" || !existsSync(src)) process.exit(0);

const yaml = (s) => JSON.stringify(s);
for (const name of readdirSync(src).sort()) {
  if (!name.endsWith(".md")) continue;
  const m = NAME.exec(name);
  if (!m) throw new Error(`docs/blog/${name}: posts are named YYYY-MM-DD-slug.md`);
  const [, date, slug] = m;
  const text = readFileSync(join(src, name), "utf8");
  const h1 = /^# (.+)\n+/m.exec(text);
  if (!h1) throw new Error(`docs/blog/${name}: a post starts with a "# Title" heading`);
  const body = text.slice(0, h1.index) + text.slice(h1.index + h1[0].length);
  // starlight-blog shows a post in full in the list unless an excerpt marker says where its preview ends
  const first = /^[^\n#>`|*<-][^\n]*(?:\n[^\n]+)*/.exec(body);
  if (!first) throw new Error(`docs/blog/${name}: the title is followed by a paragraph, the post's preview`);
  const end = first.index + first[0].length;
  const withExcerpt = `${body.slice(0, end)}\n\n<!-- excerpt -->${body.slice(end)}`;
  mkdirSync(out, { recursive: true });
  writeFileSync(
    join(out, `${slug}.md`),
    `---\ntitle: ${yaml(h1[1].trim())}\ndate: ${date}T12:00:00Z\n---\n\n${withExcerpt}`,
  );
}
