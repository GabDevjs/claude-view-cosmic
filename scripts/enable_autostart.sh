#!/usr/bin/env bash
# Write the XDG autostart .desktop entry that launches ClaudeView on login.
#
# Usage:
#   bash scripts/enable_autostart.sh           # enable
#   bash scripts/enable_autostart.sh --disable # remove
#
# Honored regardless of whether the user already enabled autostart via the
# in-app Preferences toggle (this is the same file that toggle writes to).
set -euo pipefail

AUTOSTART_DIR="$HOME/.config/autostart"
DESKTOP_FILE="$AUTOSTART_DIR/claudeview.desktop"

if [[ "${1:-}" == "--disable" ]]; then
  rm -f "$DESKTOP_FILE"
  echo "✓ removed $DESKTOP_FILE"
  exit 0
fi

# Resolve binary path. After `dpkg -i` it lands at /usr/bin/claudeview;
# fall back to the path that might be present for ad-hoc local installs.
if command -v claudeview >/dev/null 2>&1; then
  EXEC_PATH="$(command -v claudeview)"
else
  EXEC_PATH="/usr/bin/claudeview"
  echo "warning: 'claudeview' not in PATH yet — using $EXEC_PATH (will resolve after dpkg -i)" >&2
fi

mkdir -p "$AUTOSTART_DIR"
cat > "$DESKTOP_FILE" <<EOF
[Desktop Entry]
Type=Application
Name=ClaudeView
Comment=Monitor Claude Code usage from the COSMIC panel
Exec=$EXEC_PATH
Icon=claudeview
Terminal=false
Categories=Utility;Monitor;
X-GNOME-Autostart-enabled=true
EOF
chmod 0644 "$DESKTOP_FILE"
echo "✓ wrote $DESKTOP_FILE"
