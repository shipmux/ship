// @ts-check
// One Astro project for the whole site: the landing page and the 404 are plain Astro pages, the docs are
// Starlight. scripts/prepare-docs.mjs runs first and writes the docs pages for one version into
// src/content/docs/ plus .generated/site.json (this build's prefix, version and sidebar).
// scripts/build-versions.mjs runs prepare + build once per git ref and merges the outputs.
import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";
import starlightBlog from "starlight-blog";
import { readFileSync, existsSync } from "node:fs";
import { satteri } from "@astrojs/markdown-satteri";
import { alerts } from "./src/markdown/alerts.mjs";
import { rebaseLinks } from "./src/markdown/links.mjs";
import shipDocs from "./src/integrations/ship-docs.mjs";
import toml from "./src/markdown/toml.mjs";
import { siteEnv } from "./scripts/site-env.mjs";

const { base, site } = siteEnv(); // SHIP_BASE and SHIP_SITE_URL, see scripts/site-env.mjs
const meta = JSON.parse(readFileSync(new URL("./.generated/site.json", import.meta.url), "utf8"));
const ship16 = JSON.parse(readFileSync(new URL("./src/ship16.theme.json", import.meta.url), "utf8"));
// scripts/prepare-blog.mjs writes posts only in the main build; the other builds leave /blog/ alone
const hasBlog = existsSync(new URL("./src/content/docs/blog/", import.meta.url));
const css = (/** @type {string} */ href) => ({
  tag: /** @type {const} */ ("link"),
  attrs: { rel: "stylesheet", href },
});

export default defineConfig({
  // the origin only: Astro adds base itself, and starlight-blog's RSS link appends base to site, so a site
  // with /ship/ in it came out as /ship/ship/. With site set, Starlight writes a sitemap; in a versioned build
  // the latest docs' build is copied last, so its sitemap is the one kept
  site: new URL(site).origin,
  base,
  outDir: process.env.DOCS_OUT ?? "./dist",
  trailingSlash: "always",
  markdown: {
    processor: satteri({
      mdastPlugins: [alerts, rebaseLinks(base.replace(/\/$/, ""))],
      features: { smartPunctuation: false },
    }),
  }, // the guide's straight quotes stay straight
  integrations: [
    starlight({
      title: "ship docs",
      titleDelimiter: "·",
      favicon: "/marks/avatar.svg",
      disable404Route: true, // src/pages/404.astro is "lost at sea"
      head: [
        { tag: "link", attrs: { rel: "preconnect", href: "https://fonts.googleapis.com" } },
        css("https://fonts.googleapis.com/css2?family=Atkinson+Hyperlegible+Next:wght@400;700&display=swap"),
        // the ansi CSS, served as is from public/css, then the bridge that maps Starlight's markup onto it
        css(`${base}css/site.css`),
        css(`${base}css/docs.css`),
        css(`${base}css/starlight-bridge.css`),
        css(`${base}css/mode.css`), // day and night, last so its night rules win
      ],
      sidebar: meta.sidebar,
      // navigation none: Ship's own Header and NavList show the blog link, not the plugin's header overrides
      plugins: hasBlog ? [starlightBlog({ title: "ship blog", navigation: "none" })] : [],
      tableOfContents: { minHeadingLevel: 2, maxHeadingLevel: 2 },
      pagination: false,
      expressiveCode: {
        themes: [ship16],
        shiki: { langs: [toml] },
        // a one-line command and the agent prompt wrap, as ansi's .cmd pre.wrap does; scripts and config scroll sideways
        defaultProps: { overridesByLang: { "console,text": { wrap: true } } },
        useStarlightDarkModeSwitch: false,
        useStarlightUiThemeColors: false,
        styleOverrides: {
          borderRadius: "0",
          borderWidth: "2px",
          borderColor: "#1c1a27",
          codeBackground: "#1c1a27",
          codeForeground: "#d4ccb8",
          codeFontFamily: 'ui-monospace, "SF Mono", Menlo, "Cascadia Mono", "DejaVu Sans Mono", monospace',
          codeFontSize: "14px",
          codeLineHeight: "1.45",
          codePaddingBlock: "16px",
          codePaddingInline: "18px",
          uiFontFamily: '"Departure Mono", ui-monospace, monospace',
          frames: {
            frameBoxShadowCssValue: "none",
            editorBackground: "#1c1a27",
            terminalBackground: "#1c1a27",
            terminalTitlebarBackground: "#1c1a27",
            terminalTitlebarBorderBottomColor: "#1c1a27",
            terminalTitlebarDotsOpacity: "0",
            editorTabBarBackground: "#1c1a27",
            editorTabBarBorderBottomColor: "#1c1a27",
            inlineButtonBackground: "#25b2c0",
            inlineButtonBackgroundIdleOpacity: "1",
            inlineButtonBackgroundHoverOrFocusOpacity: "1",
            inlineButtonForeground: "#1c1a27",
            inlineButtonBorder: "#25b2c0",
          },
        },
      },
      components: {
        PageFrame: "./src/overrides/PageFrame.astro",
        TwoColumnContent: "./src/overrides/TwoColumnContent.astro",
        Header: "./src/overrides/Header.astro",
        Search: "./src/overrides/Search.astro",
        Sidebar: "./src/overrides/Sidebar.astro",
        PageSidebar: "./src/overrides/PageSidebar.astro",
        PageTitle: "./src/overrides/PageTitle.astro",
        SkipLink: "./src/overrides/SkipLink.astro",
        Banner: "./src/overrides/Banner.astro",
        MarkdownContent: hasBlog
          ? "./src/overrides/BlogMarkdownContent.astro"
          : "./src/overrides/MarkdownContent.astro",
        Footer: "./src/overrides/Empty.astro",
        ThemeProvider: "./src/overrides/ThemeProvider.astro",
        MobileMenuToggle: "./src/overrides/Empty.astro",
      },
    }),
    shipDocs(),
  ],
});
