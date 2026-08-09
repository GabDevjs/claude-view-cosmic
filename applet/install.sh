#!/usr/bin/env bash
#
# Install (or update) the ClaudeView COSMIC panel applet so it shows the local
# 5h / 7d Claude Code usage as text in the panel — and survives reboots.
#
#   bash install.sh          per-user install into ~/.local  (no root needed)
#   sudo bash install.sh     system install into /usr        (most robust)
#   bash install.sh --uninstall   remove everything this script installed
#
# Why two modes: the COSMIC panel only keeps applets it can rediscover at boot.
# A per-user install works as long as the .desktop lives in an XDG applications
# dir *and* the desktop database is refreshed; the system install matches how
# first-party / packaged applets are shipped and is the most reliable.
set -euo pipefail

APPID="dev.gabdevjs.ClaudeViewApplet"
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APPLET_SRC="$REPO/applet/target/release/claudeview-applet"
MAIN_SRC="$REPO/src-tauri/target/release/claudeview"
ICON_SRC="$REPO/src-tauri/icons/claudecode.png"

# --- resolve install locations + the target user's config -------------------
if [ "$(id -u)" -eq 0 ]; then
    BIN_DIR="/usr/bin"
    APP_DIR="/usr/share/applications"
    ICON_DIR="/usr/share/icons/hicolor/128x128/apps"
    TARGET_USER="${SUDO_USER:-root}"
else
    BIN_DIR="$HOME/.local/bin"
    APP_DIR="$HOME/.local/share/applications"
    ICON_DIR="$HOME/.local/share/icons/hicolor/128x128/apps"
    TARGET_USER="$(id -un)"
fi
USER_HOME="$(getent passwd "$TARGET_USER" | cut -d: -f6)"
PANEL_CFG="$USER_HOME/.config/cosmic/com.system76.CosmicPanel.Panel/v1/plugins_center"
DESKTOP="$APP_DIR/$APPID.desktop"

# --- registers/deregisters the applet id in the panel's center plugin list ---
# Idempotent + preserves the existing entries. Runs as the target user so the
# file ownership stays correct even under sudo.
panel_edit() {  # $1 = add|remove
    local action="$1"
    run_as_user() { if [ "$(id -u)" -eq 0 ]; then sudo -u "$TARGET_USER" "$@"; else "$@"; fi; }
    run_as_user python3 - "$PANEL_CFG" "$APPID" "$action" <<'PY'
import os, sys
cfg, appid, action = sys.argv[1], sys.argv[2], sys.argv[3]
os.makedirs(os.path.dirname(cfg), exist_ok=True)
try:
    txt = open(cfg).read()
except FileNotFoundError:
    txt = "Some([\n])\n"
present = f'"{appid}"' in txt
if action == "add" and not present:
    i = txt.rfind("]")
    txt = txt[:i] + f'    "{appid}",\n' + txt[i:]
    open(cfg, "w").write(txt)
    print("  registered in panel")
elif action == "remove" and present:
    lines = [l for l in txt.splitlines(keepends=True) if f'"{appid}"' not in l]
    open(cfg, "w").write("".join(lines))
    print("  removed from panel")
else:
    print("  panel already", "has" if action == "add" else "clear of", "the applet")
PY
}

if [ "${1:-}" = "--uninstall" ]; then
    echo "Uninstalling ClaudeView applet…"
    rm -f "$BIN_DIR/claudeview-applet" "$DESKTOP" "$ICON_DIR/$APPID.png"
    panel_edit remove
    command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$APP_DIR" 2>/dev/null || true
    echo "Done. Log out/in (or restart cosmic-panel) to fully clear it."
    exit 0
fi

# --- build applet if missing (the main window binary is optional) -----------
[ -x "$APPLET_SRC" ] || (echo "Building applet (release)…"; cd "$REPO/applet" && cargo build --release)

# --- install binaries + icon + .desktop -------------------------------------
mkdir -p "$BIN_DIR" "$APP_DIR" "$ICON_DIR"
install -m755 "$APPLET_SRC" "$BIN_DIR/claudeview-applet"
[ -f "$ICON_SRC" ] && install -m644 "$ICON_SRC" "$ICON_DIR/$APPID.png"

# The detail window (opened by the applet popup) is optional. Install it if a
# release build exists; otherwise leave any already-installed copy in place.
if [ -x "$MAIN_SRC" ]; then
    install -m755 "$MAIN_SRC" "$BIN_DIR/claudeview"
elif [ ! -x "$BIN_DIR/claudeview" ]; then
    echo "note: main window not built — run 'make build' if you want the popup's Open button"
fi
echo "Installed binaries in $BIN_DIR"

cat > "$DESKTOP" <<EOF
[Desktop Entry]
Name=ClaudeView
Comment=Claude Code 5h / 7d usage in the panel
Type=Application
Exec=$BIN_DIR/claudeview-applet
Terminal=false
Categories=COSMIC;
Keywords=COSMIC;Applet;Claude;Usage;
Icon=$APPID
StartupNotify=true
NoDisplay=true
X-CosmicApplet=true
X-CosmicHoverPopup=Start
X-OverflowPriority=10
EOF
echo "Installed $DESKTOP"

# Refresh the desktop database so the panel can discover the applet at boot.
command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$APP_DIR" 2>/dev/null || true

panel_edit add

echo
echo "✓ ClaudeView applet installed."
echo "  If it doesn't appear immediately, restart the panel or log out/in."
echo "  It also appears in Settings → Desktop → Panel → Add applet."
