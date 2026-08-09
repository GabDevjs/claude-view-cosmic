#!/usr/bin/env python3
"""ClaudeView statusLine command.

Claude Code invokes this once per status-bar render with the statusLine JSON on
stdin. The subscription rate limits (5-hour / 7-day rolling windows) are ONLY
exposed here — no hook event carries them, and they are not persisted to disk by
Claude Code. So we read them from this payload, write them to
``~/.claude/claudeview_cache.json`` in the schema ``cache.rs`` parses, and print
a compact line to stdout for the status bar.

Notes:
- ``rate_limits`` is present only for Claude.ai Pro/Max subscribers, and only
  after the first API response of a session. Each window may be independently
  absent. We therefore *merge* into the existing cache instead of overwriting,
  so a momentarily-absent window does not wipe previously-good data.
- ``used_percentage`` is 0-100; we store it as ``used`` with ``total`` = 100 so
  ``used/total`` round-trips back to the same percentage in the Rust/UI side.
"""
import sys
import json
import time
import pathlib


def _num(x):
    try:
        return int(round(float(x)))
    except (TypeError, ValueError):
        return None


def _window(rate_limits, key):
    """Map one rate-limit window to the {used,total,resetAt} cache shape."""
    d = rate_limits.get(key) or {}
    # Tolerate a couple of plausible field spellings across CC versions.
    pct = _num(d.get("used_percentage"))
    if pct is None:
        pct = _num(d.get("utilization"))
    if pct is None:
        return None
    reset = d.get("resets_at", d.get("reset_at"))
    return {"used": pct, "total": 100, "resetAt": reset}


def main():
    try:
        data = json.load(sys.stdin)
    except Exception:
        data = {}
    if not isinstance(data, dict):
        data = {}

    cache_path = pathlib.Path.home() / ".claude" / "claudeview_cache.json"
    try:
        cache = json.loads(cache_path.read_text())
        if not isinstance(cache, dict):
            cache = {}
    except Exception:
        cache = {}

    cache["updatedAt"] = time.time()

    rate_limits = data.get("rate_limits") or {}
    if isinstance(rate_limits, dict):
        five = _window(rate_limits, "five_hour")
        seven = _window(rate_limits, "seven_day")
        if five or seven:
            sub = cache.get("subscription")
            if not isinstance(sub, dict):
                sub = {}
            if five:
                sub["fiveHourLimit"] = five
            if seven:
                sub["sevenDayLimit"] = seven
            cache["subscription"] = sub
    else:
        five = seven = None

    # Best-effort session info (model / context window). Shapes vary by version,
    # so every read is defensive and never fatal.
    model = data.get("model")
    model_name = None
    if isinstance(model, dict):
        model_name = model.get("display_name") or model.get("id")
    elif isinstance(model, str):
        model_name = model

    session = cache.get("session")
    if not isinstance(session, dict):
        session = {}
    if model_name:
        session["model"] = model_name
    cw = data.get("context_window")
    if isinstance(cw, dict):
        used = _num(cw.get("used_tokens"))
        total = _num(cw.get("total_tokens") or cw.get("max_tokens"))
        if used is not None:
            session["contextUsed"] = used
        if total is not None:
            session["contextTotal"] = total
    if session:
        cache["session"] = session

    cache_path.parent.mkdir(parents=True, exist_ok=True)
    cache_path.write_text(json.dumps(cache))

    # The string we print becomes the Claude Code status line.
    parts = []
    if isinstance(model_name, str):
        parts.append(model_name)
    if five:
        parts.append(f"5h {five['used']}%")
    if seven:
        parts.append(f"7d {seven['used']}%")
    sys.stdout.write("  ".join(parts))


if __name__ == "__main__":
    main()
