# The hold's "first minute" demo: a long output, its scrollback opened in the
# editor and searched, then a detach and a reattach with everything still
# there. No agents: record it with `record.sh keys.tape --shell`.

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
Type `T=$(ship tab create --name tides "$SHIP_TAB" | jq -r .id); P=$(ship pane create --tab "$T" | jq -r .id)` Enter
# Points every window that attaches at the new pane, now and after the reattach.
Type `(while :; do for A in $(ship view list | jq -r 'keys[]'); do ship view select --attachment "$A" "$P"; done; sleep 0.2; done) >/dev/null 2>&1 & SEL=$!` Enter
Type "clear && ship" Enter
Wait+Screen@20s /tides/
Sleep 1s
Show

Sleep 1s
Type "seq 1 400" Enter
Sleep 1.2s
# the whole history in the editor, searched with its own keys
Alt+e
Sleep 1.5s
Type "/250" Enter
Sleep 1.8s
Type ":q" Enter
Sleep 1.2s
# detach: the server keeps everything
Alt+q
Sleep 1.5s
Type "ship" Enter
Sleep 2.5s
Hide
Type "kill $SEL" Enter
