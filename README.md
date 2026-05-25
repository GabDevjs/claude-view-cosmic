# ClaudeView (Linux / COSMIC)

Porta do app macOS [ClaudeView](https://github.com/) para Linux com
integração ao COSMIC Desktop. Monitora em tempo real:

- **Subscription** — barras de 5h e 7d com countdown de reset
- **Active sessions** — lista todos os processos `claude` rodando com PID/cwd
- **Token stats** — mensagens, tool calls e tokens por modelo (janela configurável)
- **Tray icon** — verde / laranja / vermelho conforme uso
- **Notificações** — desktop alerts ao cruzar 80% e 95%

## Stack

Tauri 2 (Rust) + TypeScript + React + Vite + recharts. Binário < 20 MB.

## Instalação (Pop!_OS / Ubuntu / Debian)

### Pré-requisitos

```bash
# toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh   # Rust 1.75+
nvm install 20                                                    # Node.js 20+
npm install -g pnpm@9

# deps de sistema (Tauri + tray)
sudo apt install -y \
  libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev \
  librsvg2-dev libxkbcommon-dev
```

### Install zero-touch

```bash
git clone <repo-url> claude-view-cosmic
cd claude-view-cosmic
pnpm install
make install
```

`make install` faz tudo:

1. Builda o `.deb`
2. `sudo dpkg -i` instala o binário em `/usr/bin/claudeview`
3. Instala o **statusline hook** (`~/.claude/hooks/claudeview_statusline.py`) e o
   registra como `PostToolUse` em `~/.claude/settings.json` (com backup `.bak`)
4. Habilita **autostart** via `~/.config/autostart/claudeview.desktop`

Depois disso o ícone aparece no panel do COSMIC e auto-inicia no próximo login.

### Outros bundles

```bash
make deb        # Pop!_OS / Ubuntu / Debian
make rpm        # Fedora
make appimage   # universal Linux
```

Os bundles ficam em `src-tauri/target/release/bundle/{deb,rpm,appimage}/`.

## Desinstalar

```bash
make uninstall
```

Remove o pacote via `dpkg -r`, deleta o `.desktop` de autostart. **Não** remove
o hook do Claude Code (`~/.claude/hooks/claudeview_statusline.py` e a entry em
`settings.json`) — faça manual se quiser:

```bash
rm ~/.claude/hooks/claudeview_statusline.py
# editar ~/.claude/settings.json e remover a entry de PostToolUse
mv ~/.claude/settings.json.bak ~/.claude/settings.json   # ou restaurar do backup
```

## Configuração

Preferências em `~/.config/claudeview/config.toml`. Editáveis tanto pelo arquivo
quanto pelo ícone ⚙ na UI do app:

```toml
[display]
show_percentage = true
alert_threshold = 80
history_window_days = 7
refresh_interval_seconds = 60

[behavior]
autostart = false
notifications_enabled = true
notification_thresholds = [80, 95]

[appearance]
theme = "auto"  # "auto" | "light" | "dark"
```

## Desenvolvimento

```bash
make dev      # hot-reload via vite + cargo
make build    # build release sem instalar
make run      # build + executa o binário direto
make restart  # mata processo rodando + reabre
```

Testes:

```bash
cd src-tauri && cargo test --lib              # 15 testes unitários (cache, sessions,
                                              # transcripts, tray, notifier, config)
pnpm exec tsc --noEmit                        # type-check do frontend
```

## Troubleshooting

**Ícone não aparece no panel COSMIC**: confirme que `libayatana-appindicator3-dev`
foi instalado. Logue, deslogue, ou faça `make restart`.

**Subscription bars ficam vazias**: o hook só popula `subscription` quando o
Claude Code disparar uma PostToolUse — abra `claude` num terminal e mande
uma mensagem; o cache deve ser escrito em `~/.claude/claudeview_cache.json`.

**Sessions list vazia**: o ClaudeView filtra processos cujo `comm == "claude"`.
Confira com `ps -eo comm | grep claude` se há algum rodando.

**Notificações não disparam**: precisam de uma **transição** de baixo pra cima
do threshold. Se você já abriu acima de 80% sem ter passado por baixo antes,
não notifica.

**Stats divergem**: o agregador soma `input + output + cache_creation + cache_read`
tokens por modelo. Compare manualmente com:

```bash
python3 -c "
import json, pathlib, collections
totals = collections.Counter()
for f in pathlib.Path.home().glob('.claude/projects/*/*.jsonl'):
    for line in f.open():
        try:
            d = json.loads(line)
        except: continue
        if d.get('type') != 'assistant': continue
        u = d.get('message', {}).get('usage', {}) or {}
        totals[d['message'].get('model','?')] += sum(int(u.get(k,0)or 0) for k in
            ('input_tokens','output_tokens','cache_creation_input_tokens','cache_read_input_tokens'))
print(dict(totals))
"
```

## Arquitetura

```
~/.claude/claudeview_cache.json  ← hook PostToolUse (python)
              │
              ▼
        watcher.rs (notify) ─ emit "cache-updated"
              │
              ├── tray.rs (color update)
              ├── notifier.rs (threshold notify)
              └── useCache.ts ─ UI refresh

~/.claude/projects/*/*.jsonl     ← transcripts.rs aggregate (poll 60s)
/proc/<pid>/cwd                  ← sessions.rs scan (poll 5s)
~/.config/claudeview/config.toml ← config.rs load/save
```

Detalhes do design em [PRA.md](./PRA.md).

## License

MIT
