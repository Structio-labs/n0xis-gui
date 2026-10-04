#!/usr/bin/env bash
# Copyright (c) 2026 Tymofii Kosovskyi
# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
#
# kwin-shot.sh <out.png> <wait-seconds> <binary> [args...]
#
# Run the GUI as a Wayland client of a nested, virtual KWin (KDE's compositor,
# its own socket and a private D-Bus session, no real display touched), then
# screenshot it with Spectacle. This is the check drive.sh cannot make: under
# Xvfb there is no compositor, so the window falls back to server-side
# decorations and the title bar's own window controls never appear.
#
# Needs: kwin_wayland, spectacle, dbus-run-session.
set -u
if [ -z "${N0X_KWIN_INNER:-}" ]; then
    N0X_KWIN_INNER=1 exec dbus-run-session -- "$0" "$@"
fi
out=$1
wait=$2
shift 2
sock=wayland-n0x-kwin-shot
kwin_wayland --virtual --width 1440 --height 900 --no-lockscreen --socket "$sock" >"${out%.png}.kwin.log" 2>&1 &
kpid=$!
sleep 4
env -u DISPLAY WAYLAND_DISPLAY="$sock" "$@" >"${out%.png}.app.log" 2>&1 &
apid=$!
sleep "$wait"
WAYLAND_DISPLAY="$sock" spectacle -b -n -f -o "$out" >/dev/null 2>&1
sleep 2
alive=$(kill -0 "$apid" 2>/dev/null && echo running || echo exited)
kill "$apid" 2>/dev/null
sleep 0.5
kill -9 "$apid" 2>/dev/null
kill "$kpid" 2>/dev/null
sleep 1
kill -9 "$kpid" 2>/dev/null
echo "app was $alive; screenshot: $out"
