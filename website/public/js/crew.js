/* Crew reactions: small, event-driven and one shot. Pressing a crewmate or a room plays its beat once (the host gets
   .react, and landing.css plays its sheet), and a room also plays once when it first scrolls into view. Nothing here
   loops, nothing becomes a tab stop (it is all aria-hidden art), and with reduced motion none of it runs. The page
   reads and works the same without this file. */
(() => {
  if (matchMedia("(prefers-reduced-motion: reduce)").matches) return;

  // the sheet that says when the beat is over: the room or crewmate's own, the prop over a deck crewmate, or the
  // nest's spyglass. A host with data-ms is timed instead (the diver's bubbles are many small elements)
  const sheet = (host) => host.querySelector(":scope > .act1") || host.querySelector(":scope > .act");
  const play = (host) => {
    if (host.classList.contains("react")) return;
    host.classList.add("react");
    const ms = Number(host.dataset.ms);
    const own = ms ? null : sheet(host);
    let timer;
    const end = () => {
      clearTimeout(timer);
      host.classList.remove("react");
    };
    own?.addEventListener("animationend", end, { once: true });
    timer = setTimeout(end, ms || 4000);
  };

  // a deck crewmate stands in for an agent, so a press also lights its plate the way alt-a does (see shipKeys)
  const light = (member) => {
    member.classList.add("called");
    clearTimeout(member.lit);
    member.lit = setTimeout(() => member.classList.remove("called"), 1200);
  };

  document.addEventListener("click", (e) => {
    const host = e.target.closest?.("[data-react], .member");
    if (!host) return;
    if (host.matches(".member")) light(host);
    play(host);
  });

  // a room plays its beat once when it is mostly in view, a moment after it arrives
  if ("IntersectionObserver" in window) {
    const seen = new IntersectionObserver(
      (entries) =>
        entries.forEach((en) => {
          if (!en.isIntersecting) return;
          seen.unobserve(en.target);
          setTimeout(() => play(en.target), 400);
        }),
      { threshold: 0.7 },
    );
    document.querySelectorAll("[data-play]").forEach((el) => seen.observe(el));
  }
})();
