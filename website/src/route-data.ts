// Starlight route middleware: blog pages are the captain's log, not docs, so their tab title and link
// previews name the blog. Starlight builds every page's head with the one site title, "ship docs".
import { defineRouteMiddleware } from "@astrojs/starlight/route-data";
import { isBlogPage } from "./blog";

export const onRequest = defineRouteMiddleware((context) => {
  const route = context.locals.starlightRoute;
  if (!isBlogPage(route.id)) return;
  for (const tag of route.head) {
    if (tag.tag === "title" && tag.content) tag.content = tag.content.replace(/ship docs$/, "ship blog");
    if (tag.tag === "meta" && tag.attrs?.property === "og:site_name") tag.attrs.content = "ship blog";
  }
});
