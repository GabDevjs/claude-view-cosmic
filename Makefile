.PHONY: dev build run install uninstall

dev:
	pnpm tauri dev

build:
	pnpm tauri build

run: build
	./src-tauri/target/release/claudeview

install: build
	sudo dpkg -i src-tauri/target/release/bundle/deb/ClaudeView_*.deb

uninstall:
	sudo dpkg -r claudeview || true
