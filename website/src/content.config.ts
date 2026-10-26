import { defineCollection } from "astro:content";
import { docsLoader } from "@astrojs/starlight/loaders";
import { docsSchema } from "@astrojs/starlight/schema";
import { blogSchema } from "starlight-blog/schema";

// src/content/docs/ is written by scripts/prepare-docs.mjs; each page's frontmatter `slug` carries its
// version prefix (docs, docs/main, docs/v1.2.0), so one build serves one version at its own path.
// Blog posts (scripts/prepare-blog.mjs) sit at blog/ and carry starlight-blog's fields, `date` among them.
export const collections = {
  docs: defineCollection({ loader: docsLoader(), schema: docsSchema({ extend: (context) => blogSchema(context) }) }),
};
