#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Run Cursor agent; SIGTERM when stdout is idle (CLI hang workaround)."""
from __future__ import annotations

import json
import os
import select
import signal
import subprocess
import sys
import time


def classify_line(line: bytes) -> str:
    """Return 'thinking_done' or 'activity'."""
    stripped = line.strip()
    if not stripped:
        return "activity"
    try:
        obj = json.loads(stripped)
    except json.JSONDecodeError:
        if b'"type":"thinking"' in stripped and b'"subtype":"completed"' in stripped:
            return "thinking_done"
        return "activity"
    if obj.get("type") == "thinking" and obj.get("subtype") == "completed":
        return "thinking_done"
    return "activity"


def feed_lines(buf: bytes, chunk: bytes) -> tuple[bytes, list[str]]:
    buf += chunk
    kinds: list[str] = []
    while b"\n" in buf:
        line, buf = buf.split(b"\n", 1)
        kinds.append(classify_line(line))
    return buf, kinds


def main() -> int:
    if len(sys.argv) < 5:
        print(
            "usage: gch-agent-idle-wrap.py <idle_secs> <after_thinking_secs> <max_secs> -- <cmd...>",
            file=sys.stderr,
        )
        return 2

    idle_long = int(sys.argv[1])
    idle_after_thinking = int(sys.argv[2])
    max_secs = int(sys.argv[3])
    rest = sys.argv[4:]
    if not rest or rest[0] != "--":
        print("missing -- before command", file=sys.stderr)
        return 2
    cmd = rest[1:]
    if not cmd:
        print("missing command", file=sys.stderr)
        return 2

    proc = subprocess.Popen(
        cmd,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    assert proc.stdout is not None

    deadline = time.monotonic() + max_secs
    last_output = time.monotonic()
    saw_output = False
    use_short_idle = False
    line_buf = b""
    fd = proc.stdout.fileno()

    while proc.poll() is None:
        now = time.monotonic()
        if now >= deadline:
            proc.kill()
            proc.wait(timeout=30)
            print("gch-agent-idle-wrap: max runtime exceeded", file=sys.stderr)
            return 124

        idle_limit = idle_after_thinking if use_short_idle else idle_long
        wait = min(5.0, idle_limit, deadline - now)
        ready, _, _ = select.select([fd], [], [], wait)
        if ready:
            chunk = os.read(fd, 65536)
            if chunk:
                line_buf, kinds = feed_lines(line_buf, chunk)
                if kinds:
                    if kinds[-1] == "thinking_done":
                        use_short_idle = True
                    else:
                        use_short_idle = False

                sys.stdout.buffer.write(chunk)
                sys.stdout.buffer.flush()
                last_output = time.monotonic()
                saw_output = True
            elif proc.poll() is not None:
                break
        elif saw_output and (time.monotonic() - last_output) >= idle_limit:
            label = "after thinking" if use_short_idle else "long idle"
            print(
                f"gch-agent-idle-wrap: no output for {idle_limit}s ({label}); stopping agent",
                file=sys.stderr,
            )
            proc.send_signal(signal.SIGTERM)
            try:
                proc.wait(timeout=30)
            except subprocess.TimeoutExpired:
                proc.kill()
                proc.wait(timeout=30)
            return 0

    return proc.wait() or 0


if __name__ == "__main__":
    sys.exit(main())
