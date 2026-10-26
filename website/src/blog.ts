// The blog, from docs/blog through scripts/prepare-blog.mjs and starlight-blog. Unversioned: only the main
// build writes posts, so blog pages carry no docs version, and every build links to them once posts exist.
import { existsSync, readdirSync } from "node:fs";
import { resolve } from "node:path";

const posts = resolve(process.cwd(), "../docs/blog"); // astro runs in website/; import.meta.url moves when bundled

/** Whether this checkout has any posts, so the header can link to /blog/ from every docs version. */
export const hasBlog = existsSync(posts) && readdirSync(posts).some((name) => name.endsWith(".md"));

/** Whether a Starlight page is one of starlight-blog's (the post list, a post, tags, authors). */
export const isBlogPage = (id: string): boolean => id === "blog" || id.startsWith("blog/");

/** Whether it is a post, as starlight-blog decides in isAnyBlogPostPage: not a list, tag or author page. */
export const isBlogPost = (id: string): boolean => /^blog\/(?!(\d+\/?|tags\/.+|authors\/.+)$).+$/.test(id);
