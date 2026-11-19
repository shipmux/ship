"""The link preview image for the Optimal-ish post (public/og-optimal-ish.png): tools/og-optimal-ish.html at 1200x630,
a boat race in the landing page's pixel art. The sprites come from tools/race.py (python3 tools/race.py) and the page's own art.
Needs the network once, for the pixel font the site loads from a CDN; it waits for the font and stops if it did not load.
usage: og_optimal_ish.py [png]"""
import os, sys, time
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from cdp import Page
here = os.path.dirname(os.path.abspath(__file__))
src = os.path.join(here, "og-optimal-ish.html")
png = os.path.abspath(sys.argv[1]) if len(sys.argv) > 1 else os.path.abspath(os.path.join(here, "..", "public", "og-optimal-ish.png"))
page = Page(1200, 630, reduced_motion=True)
try:
    page.goto(src, settle=1.0)
    if not page.eval("document.fonts.check('66px \"Departure Mono\"')"):
        sys.exit("Departure Mono did not load; check the network and run again")
    time.sleep(0.3)
    page.viewport_png(png)
    print(png)
finally:
    page.close()
