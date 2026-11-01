# The hold's agents demo: you're writing in the blog tab while a pi in the
# docs tab writes a README. The sidebar shows it working, then finished and
# unseen, and alt-a jumps to it. Real agents, like waterline.tape: record it
# with `record.sh agents.tape --openrouter` (or --dry to tune the timing, never
# shipped). Hidden waits cut the agent's think time.

Set Shell bash
Set FontFamily "JetBrains Mono"
Set FontSize 30
Set Width 1920
Set Height 1080
Set Padding 18
Set Framerate 30
Set TypingSpeed 90ms
Source ship16.tape

Hide
Type `cd "$DEMO_PROJECT" && PS1="\W $ " && set +m && source "$DEMO_WORLD" && clear` Enter
# The window opens on the blog post, away from the docs pi, which gets its task
# a moment after the window is up.
Type `(until A=$(ship view list | jq -r 'keys[0] // empty') && [ -n "$A" ]; do sleep 0.1; done; ship view select --attachment "$A" "$BLOG_PANE"; sleep 1.5; ship pane send --pane "$DOCS_PANE" "Write a short README.md for this project: one paragraph and a usage example." --key enter) >/dev/null 2>&1 &` Enter
Type "clear && ship" Enter
Wait+Screen@20s /Reading tide tables/

# Beat 1: the docs pi working in another tab, shown in the sidebar.
Wait+Screen@30s /● pi working/
Show
Sleep 3s

# Beat 2: it finished while you were elsewhere, so it shows unseen.
Hide
Wait+Screen@180s /◆ pi/
Show
Sleep 3s

# Beat 3: alt-a goes to it, and seeing it makes it idle.
Alt+a
Sleep 3.5s
