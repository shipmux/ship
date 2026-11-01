# The hold's config demo: the config open in the editor beside the sidebar,
# one value changed and saved, and the open window follows it straight away.
# No agents: record it with `record.sh config.tape --shell`.

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
# A short config of the viewer's own, rather than the tapes' recording settings: the window reads it at
# start and again on every save.
Type `printf '# ~/.config/ship/config.toml\n\n[client]\nsidebar_width = 23\n' >"$SHIP_CONFIG"` Enter
Type `T=$(ship tab create --name config "$SHIP_TAB" | jq -r .id); P=$(ship pane create --tab "$T" -- nvim "$SHIP_CONFIG" | jq -r .id)` Enter
Type `(until A=$(ship view list | jq -r 'keys[0] // empty') && [ -n "$A" ]; do sleep 0.1; done; ship view select --attachment "$A" "$P") >/dev/null 2>&1 &` Enter
Type "clear && ship" Enter
Wait+Screen@20s /sidebar_width/
Sleep 1s
Show

Sleep 1.2s
Type "/23" Enter
Sleep 0.8s
Type "ciw34"
Escape
Sleep 0.8s
Type ":w" Enter
Sleep 3s
