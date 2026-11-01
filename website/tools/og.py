"""The link preview image (public/og.png): the landing hero at 1200x630, with reduced motion so every loop
sits on frame 0. The header bar and the hero's lower half (second paragraph, install block, links, the sign's caption)
are hidden, leaving the heading, the first paragraph, the mast and the agents sign, which is what og:image:alt describes.
Run against a built site (pnpm build && pnpm preview), then commit the png.
usage: og.py [url] [png]"""
import os, sys, time
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from cdp import Page
here = os.path.dirname(os.path.abspath(__file__))
src = sys.argv[1] if len(sys.argv) > 1 else "http://127.0.0.1:4321/"
png = sys.argv[2] if len(sys.argv) > 2 else os.path.join(here, "..", "public", "og.png")
page = Page(1200, 630, reduced_motion=True)
try:
    page.goto(src, settle=3.0)
    page.eval("""(() => {
      const s = document.createElement('style');
      s.textContent = `.bar { display: none !important } :root { --bar-h: 0px !important }
        .hero-text > :not(h1, .lede), .hero figcaption { visibility: hidden }`;
      document.head.append(s);
      scrollTo(0, 60); // the sky band above the heading is spare; this brings the whole sign into frame
    })()""")
    time.sleep(0.5)
    page.viewport_png(png)
    print(png)
finally:
    page.close()
