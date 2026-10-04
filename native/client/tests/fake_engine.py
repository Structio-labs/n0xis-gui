#!/usr/bin/env python3
# Copyright (c) 2026 Tymofii Kosovskyi
# SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
"""A stand-in for `n0xis serve`, for the client's protocol tests.

It speaks the session protocol and nothing else. The behaviour is chosen by the
name of the file it is opened with, so each test asks for exactly what it needs:
  text-only*  announce only the text request form
  refuse*     fail to open, the way an unreadable target does
Requests:
  echo ...    answer with the argv as received and the raw request line
  sleep S     wait S seconds, then answer like echo
  crash       print a panic line on stderr and exit
"""
import json
import sys
import time

argv = sys.argv[1:]
target = argv[argv.index("--file") + 1] if "--file" in argv else ""
mode = target.replace("\\", "/").rsplit("/", 1)[-1]

if mode.startswith("refuse"):
    print(json.dumps({"ok": False, "error": {"code": "not-a-binary", "message": "cannot read " + mode}}), flush=True)
    sys.exit(2)

forms = ["text"] if mode.startswith("text-only") else ["text", "json-argv"]
print(json.dumps({"ok": True, "data": {"ready": True, "label": "fake:" + mode, "request_formats": forms},
                  "meta": {"schema": "n0xis.serve.ready.v1"}}), flush=True)

for line in sys.stdin:
    line = line.rstrip("\n")
    if not line.strip():
        break
    args = json.loads(line) if line.startswith("[") else line.split(" ")
    if args and args[0] == "crash":
        sys.stderr.write("thread 'main' panicked at fake: boom\n")
        sys.stderr.flush()
        sys.exit(101)
    if args and args[0] == "sleep":
        time.sleep(float(args[1]))
    print(json.dumps({"ok": True, "data": {"argv": args, "raw": line}, "meta": {"schema": "fake.echo.v1"}}), flush=True)
