// Starlight route middleware: blog pages are the captain's log, not docs, so their tab title and link
// previews name the blog. Starlight builds every page's head with the one site title, "ship docs".
// A post listed in `previews` also gets its own link preview picture in place of the site-wide og.png.
import { defineRouteMiddleware } from "@astrojs/starlight/route-data";
import { isBlogPage } from "./blog";

/** Posts with a preview picture of their own, by route id: the png in public/ and its alt text. */
const previews: Record<string, { png: string; alt: string }> = {
  "blog/optimal-ish": {
    png: "og-optimal-ish.png", // tools/og_optimal_ish.py
    alt: "a pixel-art boat race on a blue sea. a yellow sailboat marked ship follows a grey one toward a checkered finish flag. the title reads optimal-ish, and below it, 5x behind, then 0.8 ms behind.",
  },
};

export const onRequest = defineRouteMiddleware((context) => {
  const route = context.locals.starlightRoute;
  if (!isBlogPage(route.id)) return;
  const preview = previews[route.id];
  for (const tag of route.head) {
    if (tag.tag === "title" && tag.content) tag.content = tag.content.replace(/ship docs$/, "ship blog");
    if (tag.tag !== "meta" || !tag.attrs) continue;
    const key = tag.attrs.property ?? tag.attrs.name;
    if (key === "og:site_name") tag.attrs.content = "ship blog";
    if (!preview) continue;
    if (key === "og:image" || key === "twitter:image")
      tag.attrs.content = String(tag.attrs.content).replace(/\/og\.png$/, `/${preview.png}`);
    if (key === "og:image:alt") tag.attrs.content = preview.alt;
  }
});
