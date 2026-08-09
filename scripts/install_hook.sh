#!/usr/bin/env bash
# Install the ClaudeView statusLine command into ~/.claude/.
#
# The 5h / 7d subscription rate limits are ONLY exposed to a `statusLine`
# command (via its stdin JSON `rate_limits` block) — no hook event carries
# them. So we register claudeview_statusline.py as the statusLine command, not
# as a PostToolUse hook.
#
# - Copies scripts/claudeview_statusline.py to ~/.claude/hooks/
# - Registers it as `statusLine` in ~/.claude/settings.json
# - Removes the legacy PostToolUse entry that pointed at this script (older
#   versions wired it there, where it only ever wrote the raw tool event)
# - Backs up settings.json to settings.json.bak before any change
# - Merges defensively: preserves all other settings and hooks
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HOOK_SRC="$SCRIPT_DIR/claudeview_statusline.py"
CLAUDE_DIR="$HOME/.claude"
HOOKS_DIR="$CLAUDE_DIR/hooks"
HOOK_DST="$HOOKS_DIR/claudeview_statusline.py"
SETTINGS="$CLAUDE_DIR/settings.json"
SETTINGS_BAK="$CLAUDE_DIR/settings.json.bak"

if [[ ! -f "$HOOK_SRC" ]]; then
  echo "ERROR: statusLine source not found at $HOOK_SRC" >&2
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "ERROR: python3 is required" >&2
  exit 1
fi

mkdir -p "$HOOKS_DIR"
install -m 0755 "$HOOK_SRC" "$HOOK_DST"
echo "✓ installed $HOOK_DST"

# Backup settings.json before any modification. Always overwrite the .bak so a
# stale backup never wins — the previous run's settings are the known-good state.
if [[ -f "$SETTINGS" ]]; then
  cp "$SETTINGS" "$SETTINGS_BAK"
  echo "✓ backed up $SETTINGS -> $SETTINGS_BAK"
else
  echo "{}" > "$SETTINGS"
  echo "✓ created empty $SETTINGS"
fi

# Register the statusLine command and drop the legacy PostToolUse entry. The
# merge is idempotent and preserves every other setting / hook.
python3 - "$SETTINGS" "$HOOK_DST" <<'PY'
import json, sys, pathlib

settings_path = pathlib.Path(sys.argv[1])
script = sys.argv[2]
hook_cmd = f"python3 {script}"

settings = json.loads(settings_path.read_text() or "{}")

# 1) statusLine -> our command. Warn (but proceed; the .bak preserves it) if a
#    different statusLine was configured, since the feature depends on ours.
existing = settings.get("statusLine")
if isinstance(existing, dict) and existing.get("command") not in (None, hook_cmd):
    print(f"! replacing existing statusLine: {existing.get('command')!r} "
          f"(restore from {settings_path.name}.bak if unwanted)")
settings["statusLine"] = {"type": "command", "command": hook_cmd, "padding": 0}

# 2) Remove the legacy PostToolUse entry that referenced this same script.
hooks = settings.get("hooks")
if isinstance(hooks, dict) and isinstance(hooks.get("PostToolUse"), list):
    for entry in hooks["PostToolUse"]:
        inner = entry.get("hooks")
        if isinstance(inner, list):
            entry["hooks"] = [
                h for h in inner
                if not (h.get("type") == "command"
                        and "claudeview_statusline.py" in (h.get("command") or ""))
            ]
    # Drop now-empty matcher groups, and an empty PostToolUse key.
    hooks["PostToolUse"] = [e for e in hooks["PostToolUse"] if e.get("hooks")]
    if not hooks["PostToolUse"]:
        del hooks["PostToolUse"]
    if not hooks:
        del settings["hooks"]

settings_path.write_text(json.dumps(settings, indent=2) + "\n")
print(f"✓ registered statusLine in {settings_path} (legacy PostToolUse entry removed)")
PY

echo
echo "Done. The subscription bars / tray color populate only for Pro/Max plans,"
echo "after the first model response in a session (that is when Claude Code"
echo "starts sending rate_limits to the statusLine)."
