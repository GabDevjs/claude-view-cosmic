.PHONY: dev build run install uninstall hook

dev:
	pnpm tauri dev

build:
	pnpm tauri build

run: build
	./src-tauri/target/release/claudeview

hook:
	bash scripts/install_hook.sh

install: build hook
	sudo dpkg -i src-tauri/target/release/bundle/deb/ClaudeView_*.deb

uninstall:
	sudo dpkg -r claudeview || true
