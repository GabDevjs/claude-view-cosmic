.PHONY: dev build run hook deb rpm appimage install uninstall restart

# ── Dev ───────────────────────────────────────────────────────────────────────

dev:
	pnpm tauri dev

build:
	pnpm tauri build

run: build
	./src-tauri/target/release/claudeview

# ── Statusline hook ───────────────────────────────────────────────────────────

hook:
	bash scripts/install_hook.sh

# ── Bundles (PRA §6) ──────────────────────────────────────────────────────────

deb:
	pnpm tauri build --bundles deb

rpm:
	pnpm tauri build --bundles rpm

appimage:
	pnpm tauri build --bundles appimage

# ── System install (Pop!_OS / Ubuntu / Debian) ────────────────────────────────

install: deb hook
	sudo dpkg -i src-tauri/target/release/bundle/deb/ClaudeView_*.deb
	bash scripts/enable_autostart.sh

uninstall:
	sudo dpkg -r claudeview || true
	bash scripts/enable_autostart.sh --disable
	@echo "note: ~/.claude/hooks/claudeview_statusline.py and ~/.claude/settings.json untouched — remove manually if desired"

restart:
	pkill -x claudeview || true
	claudeview &
	@echo "✓ restarted"
