// Below 900px a wide table scrolls sideways inside the column (starlight-bridge.css), so it takes a tab stop:
// without one, keyboard users can't reach the columns past the edge. A Sätteri mdast plugin, like alerts.mjs.
export const tables = {
  name: "ship-scrollable-tables",
  table(node, ctx) {
    ctx.setProperty(node, "data", { hProperties: { tabIndex: 0 } });
  },
};
