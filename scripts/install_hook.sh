#!/usr/bin/env bash
# Install the ClaudeView statusline hook into ~/.claude/.
#
# - Copies scripts/claudeview_statusline.py to ~/.claude/hooks/
# - Registers it as a PostToolUse hook in ~/.claude/settings.json
# - Backs up settings.json to settings.json.bak before any change (PRA §10.9)
# - Merges defensively: preserves all existing settings and other hooks
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HOOK_SRC="$SCRIPT_DIR/claudeview_statusline.py"
CLAUDE_DIR="$HOME/.claude"
HOOKS_DIR="$CLAUDE_DIR/hooks"
HOOK_DST="$HOOKS_DIR/claudeview_statusline.py"
SETTINGS="$CLAUDE_DIR/settings.json"
SETTINGS_BAK="$CLAUDE_DIR/settings.json.bak"

if [[ ! -f "$HOOK_SRC" ]]; then
  echo "ERROR: hook source not found at $HOOK_SRC" >&2
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "ERROR: python3 is required (PRA §7.2)" >&2
  exit 1
fi

mkdir -p "$HOOKS_DIR"
install -m 0755 "$HOOK_SRC" "$HOOK_DST"
echo "✓ installed $HOOK_DST"

# Backup settings.json before any modification. Always overwrite the .bak
# so a stale backup never wins — the previous run's settings are the most
# recent known-good state.
if [[ -f "$SETTINGS" ]]; then
  cp "$SETTINGS" "$SETTINGS_BAK"
  echo "✓ backed up $SETTINGS -> $SETTINGS_BAK"
else
  echo "{}" > "$SETTINGS"
  echo "✓ created empty $SETTINGS"
fi

# Merge in the PostToolUse entry without clobbering anything else. The
# merge is idempotent: re-running install_hook.sh does not duplicate the
# entry.
python3 - "$SETTINGS" "$HOOK_DST" <<'PY'
import json, sys, pathlib

settings_path = pathlib.Path(sys.argv[1])
hook_cmd = f"python3 {sys.argv[2]}"

settings = json.loads(settings_path.read_text() or "{}")
hooks = settings.setdefault("hooks", {})
post_tool_use = hooks.setdefault("PostToolUse", [])

target_entry = None
for entry in post_tool_use:
    if entry.get("matcher", "") == "":
        target_entry = entry
        break
if target_entry is None:
    target_entry = {"matcher": "", "hooks": []}
    post_tool_use.append(target_entry)

inner = target_entry.setdefault("hooks", [])
already_installed = any(
    h.get("type") == "command" and h.get("command") == hook_cmd
    for h in inner
)
if not already_installed:
    inner.append({"type": "command", "command": hook_cmd})

settings_path.write_text(json.dumps(settings, indent=2) + "\n")
print(f"✓ registered PostToolUse hook in {settings_path}")
PY

echo
echo "Done. Run Claude Code once and confirm ~/.claude/claudeview_cache.json appears."
