// The five agent-state glyphs, written as inline code in the docs (`▲`), take their state's terminal colour: a class
// from the same .s-* tokens the landing's sketches use, so the Agents page shows what the sidebar shows. The sources
// stay plain markdown that GitHub renders as grey code. A Sätteri mdast plugin, like alerts.mjs.
const STATE = { "▲": "blocked", "✖": "error", "◆": "unseen", "●": "working", "○": "idle" };

export const glyphs = {
  name: "ship-state-glyphs",
  inlineCode(node, ctx) {
    const state = STATE[node.value];
    if (state) ctx.setProperty(node, "data", { hProperties: { className: ["glyph", `s-${state}`] } });
  },
};
