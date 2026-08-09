#!/usr/bin/env python3
"""ClaudeView status collector.

Runs identically on the local machine (invoked as a subprocess) or on a
remote host (piped over SSH into `python3 -`). Emits a single JSON object
on stdout describing subscription quota, active `claude` processes, and
token/message/tool-call stats aggregated by model over a time window.

Only uses the standard library so it needs nothing installed remotely.
"""
import argparse
import json
import os
import socket
import subprocess
import sys
import time


def parse_iso(ts):
    if not ts:
        return None
    try:
        if ts.endswith("Z"):
            ts = ts[:-1] + "+00:00"
        import datetime
        return datetime.datetime.fromisoformat(ts).timestamp()
    except Exception:
        return None


def normalize_quota(raw):
    """Accepts either the flat ClaudeView cache shape, or a raw Claude Code
    statusLine hook payload (rate_limits/context_window/model nested), and
    returns the flat shape. Older/misconfigured hook installs (e.g. wired to
    PostToolUse instead of statusLine) may write unrelated tool-call
    payloads with none of this data; that case yields None."""
    if raw is None:
        return None
    if "fiveHourPct" in raw:
        return raw

    rate_limits = raw.get("rate_limits")
    context_window = raw.get("context_window")
    model = raw.get("model")
    if not rate_limits and not context_window:
        return None

    five_h = (rate_limits or {}).get("five_hour", {}) or {}
    seven_d = (rate_limits or {}).get("seven_day", {}) or {}
    ctx = context_window or {}
    if isinstance(model, dict):
        model_id, model_display = model.get("id", ""), model.get("display_name", "")
    else:
        model_id, model_display = (model or ""), ""

    return {
        "model": model_id,
        "modelDisplay": model_display,
        "fiveHourPct": five_h.get("used_percentage"),
        "fiveHourResetsAt": five_h.get("resets_at"),
        "sevenDayPct": seven_d.get("used_percentage"),
        "sevenDayResetsAt": seven_d.get("resets_at"),
        "contextWindowPct": ctx.get("used_percentage"),
        "updatedAt": raw.get("updatedAt"),
    }


def read_quota_cache():
    path = os.path.expanduser("~/.claude/claudeview_cache.json")
    try:
        with open(path) as f:
            return normalize_quota(json.load(f))
    except Exception:
        return None


def running_claude_pids():
    # `etime` (not `etimes`) is the portable keyword across BSD (macOS) and
    # GNU (Linux) ps; BSD ps rejects `etimes` outright.
    try:
        out = subprocess.run(
            ["ps", "-eo", "pid,etime,comm"],
            capture_output=True, text=True, timeout=5,
        ).stdout
    except Exception:
        return []
    pids = []
    for line in out.splitlines()[1:]:
        parts = line.split(None, 2)
        if len(parts) < 3:
            continue
        pid, etime, comm = parts[0], parts[1], parts[2]
        if os.path.basename(comm) != "claude":
            continue
        try:
            pids.append((int(pid), etime))
        except ValueError:
            continue
    return pids


def process_cwd(pid):
    proc_cwd = f"/proc/{pid}/cwd"
    if os.path.exists(proc_cwd):
        try:
            return os.readlink(proc_cwd)
        except Exception:
            return None
    try:
        out = subprocess.run(
            ["lsof", "-a", "-p", str(pid), "-d", "cwd", "-Fn"],
            capture_output=True, text=True, timeout=5,
        ).stdout
        for line in out.splitlines():
            if line.startswith("n"):
                return line[1:]
    except Exception:
        return None
    return None


CONTEXT_WINDOW_TOKENS = 200_000


def latest_session_for_cwd(cwd):
    project_dir = os.path.expanduser(
        "~/.claude/projects/" + cwd.replace("/", "-")
    )
    if not os.path.isdir(project_dir):
        return None
    candidates = []
    try:
        for name in os.listdir(project_dir):
            if name.endswith(".jsonl"):
                full = os.path.join(project_dir, name)
                candidates.append((os.path.getmtime(full), full, name[:-6]))
    except Exception:
        return None
    if not candidates:
        return None
    candidates.sort(reverse=True)
    mtime, path, session_id = candidates[0]

    last_usage = None
    try:
        with open(path, "rb") as f:
            f.seek(0, os.SEEK_END)
            size = f.tell()
            chunk = min(size, 200_000)
            f.seek(size - chunk)
            tail = f.read().decode("utf-8", errors="ignore")
        for line in reversed(tail.splitlines()):
            try:
                entry = json.loads(line)
            except Exception:
                continue
            usage = entry.get("message", {}).get("usage")
            if usage:
                last_usage = usage
                break
    except Exception:
        pass

    ctx_pct = None
    if last_usage:
        used = (
            last_usage.get("input_tokens", 0)
            + last_usage.get("cache_read_input_tokens", 0)
            + last_usage.get("cache_creation_input_tokens", 0)
        )
        ctx_pct = round(min(used, CONTEXT_WINDOW_TOKENS) / CONTEXT_WINDOW_TOKENS * 100, 1)

    return {
        "sessionId": session_id,
        "lastModified": mtime,
        "contextWindowPct": ctx_pct,
    }


def collect_active_sessions():
    sessions = []
    for pid, etime in running_claude_pids():
        cwd = process_cwd(pid)
        entry = {
            "pid": pid,
            "cwd": cwd,
            "project": os.path.basename(cwd) if cwd else None,
            "runningFor": etime,
            "sessionId": None,
            "lastModified": None,
            "contextWindowPct": None,
        }
        if cwd:
            session = latest_session_for_cwd(cwd)
            if session:
                entry.update(session)
        sessions.append(entry)
    return sessions


def iter_transcript_files(cutoff):
    root = os.path.expanduser("~/.claude/projects")
    if not os.path.isdir(root):
        return
    for dirpath, _dirnames, filenames in os.walk(root):
        for name in filenames:
            if not name.endswith(".jsonl"):
                continue
            full = os.path.join(dirpath, name)
            try:
                if os.path.getmtime(full) < cutoff:
                    continue
            except OSError:
                continue
            yield full


def collect_token_stats(window_days):
    cutoff = time.time() - window_days * 86400
    by_model = {}
    total_messages = 0
    total_tool_calls = 0

    for path in iter_transcript_files(cutoff):
        try:
            with open(path, "r", errors="ignore") as f:
                lines = f.readlines()
        except Exception:
            continue
        for line in lines:
            line = line.strip()
            if not line:
                continue
            try:
                entry = json.loads(line)
            except Exception:
                continue
            if entry.get("type") != "assistant":
                continue
            ts = parse_iso(entry.get("timestamp"))
            if ts is not None and ts < cutoff:
                continue
            message = entry.get("message") or {}
            usage = message.get("usage")
            if not usage:
                continue
            model = message.get("model", "unknown")
            stats = by_model.setdefault(model, {
                "messages": 0, "toolCalls": 0,
                "inputTokens": 0, "outputTokens": 0,
                "cacheReadTokens": 0, "cacheCreationTokens": 0,
            })
            stats["messages"] += 1
            stats["inputTokens"] += usage.get("input_tokens", 0) or 0
            stats["outputTokens"] += usage.get("output_tokens", 0) or 0
            stats["cacheReadTokens"] += usage.get("cache_read_input_tokens", 0) or 0
            stats["cacheCreationTokens"] += usage.get("cache_creation_input_tokens", 0) or 0
            total_messages += 1

            tool_calls = sum(
                1 for block in (message.get("content") or [])
                if isinstance(block, dict) and block.get("type") == "tool_use"
            )
            stats["toolCalls"] += tool_calls
            total_tool_calls += tool_calls

    return {
        "windowDays": window_days,
        "totalMessages": total_messages,
        "totalToolCalls": total_tool_calls,
        "byModel": by_model,
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--window-days", type=int, default=7)
    args = parser.parse_args()

    result = {
        "hostname": socket.gethostname(),
        "generatedAt": time.time(),
        "quota": read_quota_cache(),
        "activeSessions": collect_active_sessions(),
        "tokenStats": collect_token_stats(args.window_days),
    }
    json.dump(result, sys.stdout)


if __name__ == "__main__":
    main()
