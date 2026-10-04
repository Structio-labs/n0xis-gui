#!/usr/bin/env bash
# Copyright (c) 2026 Tymofii Kosovskyi
# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
#
# drive.sh <out-prefix> <binary> [args...]
#
# Run the GUI on a private Xvfb display (software rendering, no GPU: the same
# conditions as a VM), drive it with xdotool, and screenshot where asked. Steps
# come from stdin, one per line:
#
#   wait <seconds>        sleep
#   click <x> <y>         left click at window coordinates
#   type <text>           type text
#   key <keysym>          press a key (e.g. Return, ctrl+q)
#   shot <name>           save <out-prefix>-<name>.png
#
# Example:
#   printf 'wait 8\nshot start\nclick 85 257\nwait 3\nshot selected\n' |
#     native/scripts/drive.sh /tmp/ui target/debug/n0xis-ui /path/to/binary
#
# Needs: Xvfb, xdotool, ImageMagick (import). The app's own output goes to
# <out-prefix>.log.
set -u
prefix=$1
shift
disp=${DRIVE_DISPLAY:-:96}
Xvfb "$disp" -screen 0 1440x900x24 -nolisten tcp >/dev/null 2>&1 &
xpid=$!
sleep 1
env -u WAYLAND_DISPLAY DISPLAY="$disp" "$@" >"$prefix.log" 2>&1 &
apid=$!
while read -r cmd arg rest; do
    case "$cmd" in
        wait) sleep "$arg" ;;
        click) DISPLAY="$disp" xdotool mousemove "$arg" "$rest" click 1 ;;
        type) DISPLAY="$disp" xdotool type --delay 60 "$arg${rest:+ $rest}" ;;
        key) DISPLAY="$disp" xdotool key "$arg" ;;
        shot)
            DISPLAY="$disp" import -window root "$prefix-$arg.png" 2>/dev/null
            echo "shot $prefix-$arg.png"
            ;;
    esac
done
alive=$(kill -0 "$apid" 2>/dev/null && echo running || echo exited)
kill "$apid" 2>/dev/null
sleep 0.5
kill -9 "$apid" 2>/dev/null
kill "$xpid" 2>/dev/null
echo "app was $alive at the end"
