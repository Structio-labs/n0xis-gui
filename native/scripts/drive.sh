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
#   drag <x1> <y1> <x2> <y2>  press at the first point, move in steps, release at the second
#   wheel <x> <y> <up|down> <n>  turn the mouse wheel n notches over a point
#   clip <name>           save the clipboard's text to <out-prefix>-<name>.txt (needs xclip)
#   shiftclick <x> <y>    left click with Shift held
#   run <command...>      run a shell command (e.g. signal a process under test)
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
        drag)
            read -r y1 x2 y2 <<<"$rest"
            DISPLAY="$disp" xdotool mousemove "$arg" "$y1" mousedown 1
            for i in 1 2 3 4 5 6 7 8; do
                DISPLAY="$disp" xdotool mousemove $((arg + (x2 - arg) * i / 8)) $((y1 + (y2 - y1) * i / 8))
                sleep 0.05
            done
            DISPLAY="$disp" xdotool mouseup 1
            ;;
        wheel)
            read -r y dir n <<<"$rest"
            button=4
            [ "$dir" = down ] && button=5
            DISPLAY="$disp" xdotool mousemove "$arg" "$y" click --repeat "$n" --delay 80 "$button"
            ;;
        clip) DISPLAY="$disp" xclip -o -selection clipboard >"$prefix-$arg.txt" 2>/dev/null ;;
        shiftclick) DISPLAY="$disp" xdotool mousemove "$arg" "$rest" keydown shift click 1 keyup shift ;;
        run) sh -c "$arg${rest:+ $rest}" ;;
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
