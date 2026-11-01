# The hold's saved tabs demo: stop the whole server, start it again, and the
# tabs, splits and directories come back with fresh shells. No agents: record
# it with `record.sh saved-tabs.tape --shell`.

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
Type `(until A=$(ship view list | jq -r 'keys[0] // empty') && [ -n "$A" ]; do sleep 0.1; done; ship view select --attachment "$A" "$API_PANE") >/dev/null 2>&1 &` Enter
Type "clear && ship" Enter
Wait+Screen@20s /api/
Sleep 1s
Show

Sleep 1.5s
Alt+q
Sleep 1s
# stopping the server ends every program in it; the layout is saved
Type "ship server stop" Enter
Sleep 1.5s
Type "ship" Enter
Sleep 2s
# the window opens on the first tab, ship, which only holds others: tabs mode steps down to api
Alt+t
Sleep 400ms
Type "j"
Sleep 400ms
Escape
Sleep 3s
