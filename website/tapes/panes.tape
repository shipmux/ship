# The "tabs and panes" demo below deck: split, split again, stack a shell on
# top, then zoom one pane and back. No agents, so every take is the same:
# record it with `record.sh panes.tape --shell`.

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
# A fresh tab under ship with one shell, and the new window pointed at it.
Type `T=$(ship tab create --name panes "$SHIP_TAB" | jq -r .id); P=$(ship pane create --tab "$T" | jq -r .id)` Enter
Type `(until A=$(ship view list | jq -r 'keys[0] // empty') && [ -n "$A" ]; do sleep 0.1; done; ship view select --attachment "$A" "$P") >/dev/null 2>&1 &` Enter
Type "clear && ship" Enter
Wait+Screen@20s /panes/
Sleep 1s
Show

Sleep 1.2s
# split right, then down (ship.toml says why alt-| and alt-s are typed the way they are)
Alt+"|"
Sleep 1.4s
Alt+"-"
Sleep 1.4s
Type "ls" Enter
Sleep 1.2s
# a shell stacked in front of this pane: the pane keeps its size
Alt+"S"
Sleep 1.6s
Alt+","
Sleep 1.4s
# zoom to fill the tab, and back
Alt+f
Sleep 1.8s
Alt+f
Sleep 2s
