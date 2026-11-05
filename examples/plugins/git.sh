#!/bin/sh
# Tags each pane in a git checkout with its branch as git.branch, in cyan,
# and with the branch's pull request as git.pr, like `○ #112 ✓`. Removes
# the tags where they no longer apply. Needs ship, jq and git, and gh signed
# in for pull requests.
#
#   sh examples/plugins/git.sh &
#   GIT_ICONS=nerd sh examples/plugins/git.sh &   # GitHub's icons, from a Nerd Font
#
# Branches follow `ship watch`, whose states already hold each pane's
# directory and tags, so an unchanged pane costs a git call and no write.
# Pull requests change on GitHub while a pane sits still, so they are also
# looked up every 30 seconds, with answers kept in a temporary folder until
# then.

ship=${SHIP_BIN:-ship}
protocol=22
every=30
icons=${GIT_ICONS:-plain}
us=$(printf '\037')
cache=$(mktemp -d)
trap 'kill "$(cat "$cache/ticker" 2>/dev/null)" 2>/dev/null; rm -rf "$cache"' EXIT
trap 'exit 1' HUP INT TERM

prs=no
if command -v gh >/dev/null && gh auth status >/dev/null 2>&1; then
  prs=yes
fi

# One line per pane with a known directory: id, dir, and its git.branch and
# git.pr tags as "VALUE FG", or empty. Panes sit in layout slots, which nest
# in splits and tabs. The jq programs are in single quotes on purpose.
# shellcheck disable=SC2016
panes='def tag($name): .tags[$name] | if . then "\(.value) \(.fg)" else "" end;
  .. | objects | select(.kind == "slot") | .panes[] | select(.dir != null)
  | [.id, .dir, tag("git.branch"), tag("git.pr")] | join("\u001f")'

# A pull request from `gh pr view --json number,state,isDraft,statusCheckRollup`
# as "COLOR ICON #NUMBER CHECKS".
# shellcheck disable=SC2016
summary='
  def mark($plain; $nerd): if $icons == "nerd" then $nerd else $plain end;
  def check:
    if .__typename == "StatusContext" then
      if .state == "SUCCESS" then "pass"
      elif .state == "PENDING" or .state == "EXPECTED" then "pending"
      else "fail" end
    elif .status != "COMPLETED" then "pending"
    elif .conclusion == "SUCCESS" or .conclusion == "NEUTRAL" or .conclusion == "SKIPPED" then "pass"
    else "fail" end;
  ([.statusCheckRollup[]? | check]) as $checks
  | (if any($checks[]; . == "fail") then "fail"
     elif any($checks[]; . == "pending") then "pending"
     elif ($checks | length) > 0 then "pass"
     else "none" end) as $checks
  | {pass: mark("✓"; ""), fail: mark("✗"; ""), pending: mark("●"; ""), none: ""}[$checks] as $mark
  | if .state == "MERGED" then "magenta \(mark("◆"; "")) #\(.number)"
    elif .state == "CLOSED" then "red \(mark("×"; "")) #\(.number)"
    elif .isDraft then "bright_black \(mark("◇"; "")) #\(.number) \($mark)"
    else "\({pass: "green", fail: "red", pending: "yellow", none: "green"}[$checks]) \(mark("○"; "")) #\(.number) \($mark)"
    end
  | rtrimstr(" ")'

# pull_request DIR BRANCH: prints the branch's pull request summary, or
# nothing when it has none. Asks GitHub at most once per $every seconds per
# branch; when GitHub can't be reached, the last answer stands.
pull_request() {
  repo=$(cd "$1" && git rev-parse --path-format=absolute --git-common-dir) || return
  file="$cache/$(printf '%s %s' "$repo" "$2" | cksum | cut -d' ' -f1)"
  now=$(date +%s)
  if [ -f "$file" ] && [ $((now - $(head -n 1 "$file"))) -lt "$every" ]; then
    tail -n +2 "$file"
    return
  fi
  if answer=$(cd "$1" && gh pr view "$2" --json number,state,isDraft,statusCheckRollup 2>"$file.err" |
    jq -r --arg icons "$icons" "$summary") && [ -n "$answer" ]; then
    :
  elif grep -q 'no pull requests found' "$file.err"; then
    answer=
  elif [ -f "$file" ]; then
    answer=$(tail -n +2 "$file")
  else
    return
  fi
  printf '%s\n%s\n' "$now" "$answer" >"$file.new" && mv "$file.new" "$file"
  printf '%s\n' "$answer"
}

# tag PANE NAME CURRENT VALUE COLOR: sets the tag unless CURRENT, the pane's
# "VALUE FG", already matches. An empty VALUE removes it.
tag() {
  if [ -z "$4" ]; then
    [ -z "$3" ] || "$ship" pane set --pane "$1" --tag "$2" >/dev/null
  elif [ "$4 $5" != "$3" ]; then
    "$ship" pane set --pane "$1" --tag "$2=$4" --tag-fg "$2=$5" >/dev/null
  fi
}

# Brings the tags of each pane line on stdin up to date.
sync() {
  while IFS=$us read -r pane dir branch_tag pr_tag; do
    pr=
    if branch=$(git -C "$dir" symbolic-ref --short -q HEAD 2>/dev/null); then
      [ "$prs" = no ] || pr=$(pull_request "$dir" "$branch")
    else
      # Detached, or outside a checkout when this fails too.
      branch=$(git -C "$dir" rev-parse --short HEAD 2>/dev/null)
    fi
    tag "$pane" git.branch "$branch_tag" "$branch" cyan
    tag "$pane" git.pr "$pr_tag" "${pr#* }" "${pr%% *}"
  done
}

"$ship" watch | {
  # The first line is what `ship server status` prints.
  read -r health || exit 1
  version=$(printf '%s\n' "$health" | jq -r .protocol_version)
  if [ "$version" != "$protocol" ]; then
    echo "git: written for protocol $protocol, the server speaks $version" >&2
  fi

  # The timed lookups connect to this server's socket only, so they never
  # start a server after it stops.
  if [ "$prs" = yes ]; then
    url="unix://$(printf '%s\n' "$health" | jq -r .socket)"
    while sleep "$every"; do
      "$ship" tab list --server-url "$url" 2>/dev/null | jq -r "$panes" | sync
    done &
    echo "$!" >"$cache/ticker"
  fi

  # The first state is at .data.replica and later ones at .data; other line
  # types are skipped.
  jq --unbuffered -r "(.data.replica // select(.type == \"state\").data) | $panes" | sync
}
