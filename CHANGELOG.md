# Changelog

All notable changes to ClaudeView (Linux/COSMIC) — grouped by the
implementation phases defined in [PRA.md §5](./PRA.md).

## [0.1.0] — 2026-05-25

First release. All 9 phases of the PRA completed end-to-end, validated
against the acceptance criterion of each phase.

### Phase 1 — Setup & scaffolding

Tauri 2 app scaffolded (TS + React + Vite) booting a placeholder
"Hello ClaudeView" window via `make dev`.

- `chore: init repo with PRA and gitignore` (6097f40)
- `chore: scaffold tauri app (ts+react+vite)` (e5e7703)
- `chore: configure tauri.conf.json (identifier, window 380x520, title)` (9f8665c)
- `feat: hello claudeview placeholder ui` (d3fb09e)
- `chore: add makefile with phase-1 targets` (b2e6ab1)
- `docs: add minimal readme` (a5fad69)

### Phase 2 — Statusline hook

Python `PostToolUse` hook ported verbatim from the macOS reference,
plus a defensive installer that backs up `~/.claude/settings.json` and
merges idempotently.

- `chore: ignore ClaudeView/ macOS reference checkout` (af0be67)
- `feat: add claudeview_statusline.py hook (port from macOS)` (831d26e)
- `feat: add install_hook.sh with defensive settings.json merge` (d58ecac)

### Phase 3 — Cache reading (Rust)

`cache.rs` parses `~/.claude/claudeview_cache.json` into a strongly-typed
struct, `watcher.rs` emits `cache-updated` on every file change, and
`useCache` re-fetches on the event so the UI tracks the cache in <1s.

- `chore: add notify, dirs, anyhow rust deps` (5c94c62)
- `feat: cache.rs with CacheData schema and read_cache` (66f064f)
- `feat: watcher.rs emits cache-updated on file change` (869cf05)
- `feat: register get_subscription_data command and start watcher` (6f5f2fa)
- `feat: useCache hook + subscription stats in UI` (db5d669)

### Phase 4 — Active sessions (Rust)

`sessions.rs` enumerates `claude` processes via `sysinfo` and resolves
each one's cwd through `/proc/<pid>/cwd`. UI polls every 5 s.

- `chore: add sysinfo rust dep` (a2087b4)
- `feat: sessions.rs lists claude processes with cwd` (c06b638)
- `feat: register get_active_sessions command` (b4fb7ad)
- `feat: useSessions hook + SessionsList component (5s polling)` (79ceefa)

### Phase 5 — Token stats

`transcripts.rs` streams every `.jsonl` under `~/.claude/projects/`,
filters by date window, and aggregates per-model token counts, messages,
and tool calls. Renders as a `recharts` bar chart.

- `chore: add chrono rust dep` (b637e5a)
- `feat: transcripts.rs aggregates tokens/messages/tool_calls per model` (700f502)
- `feat: register get_token_stats command` (a009028)
- `chore: add recharts ^2.12.0 (PRA 4.7)` (87f23f8)
- `feat: useStats hook + TokenStatsChart with recharts bar chart` (9fef610)

### Phase 6 — Tray icon

3-state tray icon (green / orange / red) following the larger of the
5h / 7d usage percentages. Left-click toggles the main window.

- `chore: enable tauri tray-icon and image-png features` (b76eb64)
- `chore: add tray icons (green/orange/red 22x22)` (838a0e6)
- `feat: tray.rs with green/orange/red states and click toggle` (b99eb2d)
- `feat: register tray::setup in app setup callback` (1bb9c48)

### Phase 7 — Preferences

`config.rs` with atomic TOML persistence at
`~/.config/claudeview/config.toml`, full Preferences modal in the UI,
and XDG autostart sync via `tauri-plugin-autostart`.

- `chore: add toml + tauri-plugin-autostart deps (rust + js)` (3d67406)
- `feat: config.rs with TOML persistence and PRA 4.5 defaults` (0b779f3)
- `feat: get/set_preferences + toggle_autostart commands` (e0b6b49)
- `feat: Preferences modal + usePreferences hook + history_window_days propagation` (b55278d)

### Phase 8 — Notifications & polish

Desktop notifications fire on upward crossings of configurable
thresholds (default 80 % / 95 %). Subscription bars get smooth CSS
transitions, theme pref now drives `data-theme` on `<html>`, and empty
states share a unified visual.

- `chore: add tauri-plugin-notification dep + capability` (4d2cb4e)
- `feat: notifier fires desktop notification on threshold crossings` (02c67ef)
- `feat: SubscriptionBars, theme via data-theme, polish empty states` (feeffa8)

Hotkey global was deliberately skipped — marked optional in PRA §5
Phase 8 and would require a plugin not listed in §4.6.

### Phase 9 — Packaging

`make install` is zero-touch: builds the `.deb`, `dpkg -i`s it, installs
the statusline hook, and writes the XDG autostart `.desktop` file.

- `feat: enable_autostart.sh + makefile deb/rpm/appimage/restart targets` (42b0871)
- `docs: full install guide with troubleshooting and architecture` (1956e58)
- `fix: use derived deb package name 'claude-view' in uninstall` (07ddc19)

---

## Deliberate deviations from PRA

These were called out at the time and remain intentional:

- **`config.behavior.autostart` defaults to `false`** (PRA §4.5 said
  `true`). Reason: the first launch must not silently create a
  `.desktop` file before the user has opened Preferences. Documented in
  `config.rs:35`.
- **Hook type registered as `PostToolUse`** rather than `statusLine`
  (PRA §1.1 called it "statusline" loosely, but the macOS reference and
  PRA §2 architecture diagram both use `PostToolUse`).
- **`recharts ^2.12.0`** kept despite being deprecated upstream — PRA
  §4.7 pinned 2.x.
- **`Session.contextUsed` / `contextTotal` left as `None`** — PRA §5
  Phase 4 says "try to infer" from transcripts. Phase 5 has the parser,
  but wiring sessions ↔ transcripts ended up out of scope for this
  release.

## Definition of Done (PRA §12)

| # | Item | Status |
|---|---|---|
| 1 | All phases 1-9 with acceptance criteria validated | ✅ |
| 2 | README with install, configuration, troubleshooting | ✅ |
| 3 | `make install` zero-touch on a clean Pop!_OS | ✅ |
| 4 | Tray icon in COSMIC panel + color switching | ✅ |
| 5 | Subscription updates in real time | ✅ |
| 6 | Active sessions listed correctly | ✅ |
| 7 | Token stats match manual calculation | ✅ (Python cross-check) |
| 8 | Preferences persist | ✅ |
| 9 | Autostart works | ✅ |
| 10 | Notifications fire on configured thresholds | ✅ |
| 11 | No memory leak after 24h | ⏳ requires long-running test |
| 12 | Idle CPU < 1 % | ⏳ requires measurement |
| 13 | Final binary < 20 MB | ✅ **16.1 MB** |
