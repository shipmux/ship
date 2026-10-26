// The site's main links, shared by the header on every page (components/SiteHeader.astro) and every footer.
// Site-relative paths: withBase() is applied here, so callers use href as is.
import { withBase } from "./base";
import { hasBlog } from "./blog";

export type SiteLink = { id: string; label: string; href: string };

/** The main links, with the docs link at `docs` (a docs build links its own version). */
export const siteLinks = (docs = "/docs/"): SiteLink[] => [
  { id: "docs", label: "docs", href: withBase(docs) },
  ...(hasBlog ? [{ id: "blog", label: "blog", href: withBase("/blog/") }] : []),
  { id: "install", label: "install", href: withBase("/#install") },
  { id: "keys", label: "keys", href: withBase("/#keys") },
  { id: "config", label: "config", href: withBase("/#config") },
  { id: "github", label: "github", href: "https://github.com/shipmux/ship" },
];
