# ClaudeView Linux — Requirements & Build Plan

> Documento de requisitos para Claude Code construir o **ClaudeView**, uma porta do app macOS original para Linux/COSMIC Desktop (Pop!_OS).

---

## 1. Contexto do projeto

### 1.1 Background

Já existe uma versão funcional do **ClaudeView** para macOS (Swift + AppKit), que vive na menu bar e monitora em tempo real:

- Uso de subscription do Claude Code (limites de 5h e 7 dias)
- Sessões ativas do Claude Code (% de context window)
- Estatísticas de tokens (mensagens, tool calls, tokens por modelo)

A versão macOS lê dados de `~/.claude/claudeview_cache.json`, que é preenchido por um **statusline hook** do próprio Claude Code (script Python registrado em `~/.claude/settings.json`).

### 1.2 Objetivo

Construir uma versão **Linux nativa** focada em **COSMIC Desktop** (Pop!_OS), mantendo paridade funcional com a versão macOS, com integração visual ao panel do COSMIC.

### 1.3 Stack escolhida

- **Tauri 2.x** (framework principal)
- **Rust** (backend — leitura de arquivos, parsing, monitoramento de processos)
- **TypeScript + React** (frontend do popup/janela)
- **tray-icon crate** (ícone no system tray do COSMIC panel)

**Justificativa da escolha:**
- Binário pequeno (~5-15 MB) vs Electron (~150+ MB)
- Aproveita stack web que o dev já domina (TS/React)
- Rust no backend força aprendizado relevante e dá performance
- Funciona em qualquer DE Linux (não fica preso ao COSMIC)
- A lógica de leitura do cache JSON é portável direto do macOS

---

## 2. Arquitetura

```
┌─────────────────────────────────────────────────────┐
│  Claude Code (CLI)                                   │
│  └─ PostToolUse hook                                 │
│     └─ statusline_hook.py                            │
│        └─ escreve ~/.claude/claudeview_cache.json    │
└─────────────────────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────┐
│  ClaudeView (Tauri app)                              │
│                                                       │
│  ┌──────────────────────────────────────────────┐   │
│  │  Rust Backend                                 │   │
│  │  ├─ File watcher (notify crate)              │   │
│  │  ├─ JSON parser (serde)                      │   │
│  │  ├─ Process scanner (sysinfo)                │   │
│  │  ├─ Tray icon (tray-icon)                    │   │
│  │  └─ IPC commands (Tauri commands)            │   │
│  └──────────────────────────────────────────────┘   │
│                       │                               │
│                       ▼ (Tauri IPC)                  │
│  ┌──────────────────────────────────────────────┐   │
│  │  React Frontend (popup window)                │   │
│  │  ├─ Subscription bars (5h / 7d)              │   │
│  │  ├─ Active sessions list                     │   │
│  │  ├─ Token stats por modelo                   │   │
│  │  └─ Preferences pane                         │   │
│  └──────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────┘
                       │
                       ▼
            ┌────────────────────────┐
            │  COSMIC Panel          │
            │  (Status Area applet)  │
            │  └─ ícone ClaudeView   │
            └────────────────────────┘
```

### 2.1 Componentes

| Componente | Responsabilidade |
|---|---|
| **Statusline hook** | Script Python que captura dados do Claude Code e escreve no cache |
| **File watcher (Rust)** | Detecta mudanças em `~/.claude/claudeview_cache.json` e emite eventos |
| **Process scanner (Rust)** | Lista processos `claude` ativos e lê seu uso de context window |
| **Tray icon (Rust)** | Ícone no panel; muda de cor (verde → laranja → vermelho) conforme uso |
| **Popup window (React)** | UI principal com gráficos, listas e configurações |
| **Settings storage** | Preferências do usuário em `~/.config/claudeview/config.toml` |

---

## 3. Funcionalidades (paridade com macOS)

### 3.1 Core features (P0 — obrigatórias)

- [ ] **Tray icon no panel COSMIC**
  - Ícone padrão (SVG)
  - Cor dinâmica: verde (<80%), laranja (80-95%), vermelho (>95%)
  - Texto opcional com % ao lado do ícone (toggle em config)

- [ ] **Subscription usage**
  - Barra de 5 horas (quota + countdown de reset)
  - Barra de 7 dias (quota + countdown de reset)
  - Lê de `~/.claude/claudeview_cache.json`
  - Atualiza automaticamente via file watcher

- [ ] **Active sessions**
  - Lista todos os processos `claude` rodando
  - Mostra: PID, project path (cwd), % do context window
  - Atualiza a cada N segundos (configurável)

- [ ] **Token stats**
  - Total de mensagens, tool calls, tokens
  - Agrupado por modelo (sonnet, opus, haiku)
  - Janela configurável (padrão: últimos 7 dias)
  - Lê histórico de `~/.claude/projects/**/*.jsonl` (transcripts)

- [ ] **Auto-start no login**
  - Cria entrada XDG autostart em `~/.config/autostart/claudeview.desktop`
  - Toggle nas Preferências

### 3.2 Nice-to-have (P1)

- [ ] Notificação desktop ao atingir 80% e 95%
- [ ] Histórico de uso (gráfico semanal)
- [ ] Dark/light mode automático (segue tema do COSMIC)
- [ ] Hotkey global pra abrir popup (ex: Super+Shift+C)
- [ ] Export de stats em CSV/JSON

### 3.3 Backlog (P2)

- [ ] Suporte multi-conta
- [ ] Integração com `cosmic-applets` como applet nativo (port em Rust puro)
- [ ] Comparação de uso entre dias/semanas
- [ ] Alerta sonoro opcional

---

## 4. Especificação técnica

### 4.1 Estrutura de pastas

```
claudeview/
├── Cargo.toml
├── package.json
├── tauri.conf.json
├── README.md
├── Makefile
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   ├── icons/
│   │   ├── icon.png
│   │   ├── tray-green.png
│   │   ├── tray-orange.png
│   │   └── tray-red.png
│   └── src/
│       ├── main.rs
│       ├── lib.rs
│       ├── cache.rs           # Leitura do claudeview_cache.json
│       ├── sessions.rs        # Scan de processos claude
│       ├── transcripts.rs     # Parsing dos .jsonl em ~/.claude/projects
│       ├── tray.rs            # Lógica do tray icon
│       ├── watcher.rs         # File watcher
│       ├── config.rs          # User preferences
│       └── commands.rs        # Tauri commands (bridge JS<->Rust)
├── src/
│   ├── main.tsx
│   ├── App.tsx
│   ├── components/
│   │   ├── SubscriptionBars.tsx
│   │   ├── SessionsList.tsx
│   │   ├── TokenStats.tsx
│   │   └── Preferences.tsx
│   ├── hooks/
│   │   ├── useCache.ts        # Subscribe a eventos do Rust
│   │   ├── useSessions.ts
│   │   └── useStats.ts
│   ├── types/
│   │   └── index.ts
│   └── styles/
│       └── global.css
├── scripts/
│   ├── install_hook.sh        # Instala o statusline hook
│   └── claudeview_statusline.py
└── public/
    └── tray-icons/
```

### 4.2 Schema do cache JSON

`~/.claude/claudeview_cache.json` (escrito pelo hook):

```json
{
  "updatedAt": 1716553200.123,
  "subscription": {
    "fiveHourLimit": {
      "used": 45,
      "total": 100,
      "resetAt": 1716570000
    },
    "sevenDayLimit": {
      "used": 320,
      "total": 1000,
      "resetAt": 1717160000
    }
  },
  "session": {
    "model": "claude-sonnet-4-7",
    "contextUsed": 85000,
    "contextTotal": 200000,
    "messagesCount": 42
  }
}
```

### 4.3 Tauri commands (Rust → JS)

```rust
#[tauri::command]
async fn get_subscription_data() -> Result<SubscriptionData, String>;

#[tauri::command]
async fn get_active_sessions() -> Result<Vec<Session>, String>;

#[tauri::command]
async fn get_token_stats(days: u32) -> Result<TokenStats, String>;

#[tauri::command]
async fn get_preferences() -> Result<Preferences, String>;

#[tauri::command]
async fn set_preferences(prefs: Preferences) -> Result<(), String>;

#[tauri::command]
async fn install_statusline_hook() -> Result<(), String>;

#[tauri::command]
async fn toggle_autostart(enable: bool) -> Result<(), String>;
```

### 4.4 Eventos (Rust → JS, via emit)

- `cache-updated` — quando o file watcher detecta mudança
- `sessions-updated` — refresh periódico de sessões
- `tray-clicked` — usuário clicou no ícone do tray

### 4.5 Preferências (`~/.config/claudeview/config.toml`)

```toml
[display]
show_percentage = true
alert_threshold = 80
history_window_days = 7
refresh_interval_seconds = 60

[behavior]
autostart = true
notifications_enabled = true
notification_thresholds = [80, 95]

[appearance]
theme = "auto"  # "auto" | "light" | "dark"
```

### 4.6 Crates Rust necessárias

```toml
[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-shell = "2"
tauri-plugin-notification = "2"
tauri-plugin-autostart = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
tokio = { version = "1", features = ["full"] }
notify = "6"          # File watcher
sysinfo = "0.30"      # Listar processos
dirs = "5"            # Paths ~/.claude, ~/.config
chrono = "0.4"        # Timestamps
anyhow = "1"          # Error handling
```

### 4.7 Dependências NPM (frontend)

```json
{
  "dependencies": {
    "@tauri-apps/api": "^2.0.0",
    "@tauri-apps/plugin-shell": "^2.0.0",
    "react": "^18.3.0",
    "react-dom": "^18.3.0",
    "recharts": "^2.12.0"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2.0.0",
    "@vitejs/plugin-react": "^4.3.0",
    "typescript": "^5.4.0",
    "vite": "^5.2.0"
  }
}
```

---

## 5. Plano de implementação (fases)

### Fase 1 — Setup & scaffolding
**Objetivo:** projeto rodando vazio, hello world.

1. Criar projeto Tauri com `cargo create-tauri-app` (TS + React + Vite)
2. Configurar `tauri.conf.json` (window size, icon, identifier)
3. Subir hello world e validar que builda no Pop!_OS
4. Configurar Makefile com targets: `build`, `run`, `dev`, `install`, `uninstall`

**Critério de aceitação:** `make dev` abre janela com "Hello ClaudeView".

---

### Fase 2 — Statusline hook
**Objetivo:** reaproveitar o hook do macOS no Linux.

1. Criar `scripts/claudeview_statusline.py` (idêntico ao macOS)
2. Criar `scripts/install_hook.sh`:
   - Cria `~/.claude/hooks/` se não existir
   - Copia o script Python
   - Adiciona entrada em `~/.claude/settings.json` (merge inteligente, sem sobrescrever hooks existentes)
3. Validar que após rodar Claude Code, o arquivo `~/.claude/claudeview_cache.json` é criado e atualizado

**Critério de aceitação:** rodar Claude Code uma vez → arquivo de cache existe com dados válidos.

---

### Fase 3 — Leitura do cache (Rust)
**Objetivo:** backend lê e disponibiliza dados do cache.

1. Implementar `src-tauri/src/cache.rs`:
   - Struct `CacheData` com serde
   - Função `read_cache() -> Result<CacheData>`
2. Implementar `src-tauri/src/watcher.rs`:
   - Usar `notify` para observar `~/.claude/claudeview_cache.json`
   - Emitir evento `cache-updated` no Tauri quando muda
3. Expor comando `get_subscription_data()`
4. Frontend: hook `useCache()` que escuta o evento e re-fetch

**Critério de aceitação:** modificar o cache JSON manualmente → UI atualiza em <1s.

---

### Fase 4 — Active sessions (Rust)
**Objetivo:** listar processos Claude Code rodando.

1. Implementar `src-tauri/src/sessions.rs`:
   - Usar `sysinfo` pra listar processos cujo nome contém `claude`
   - Pra cada um, ler cwd via `/proc/{pid}/cwd`
   - Tentar inferir context % dos transcripts mais recentes em `~/.claude/projects/{cwd-hash}/`
2. Expor comando `get_active_sessions()`
3. Polling a cada N segundos no frontend (configurável)

**Critério de aceitação:** rodar `claude` em 2 terminais diferentes → UI mostra ambos.

---

### Fase 5 — Token stats
**Objetivo:** estatísticas históricas de uso.

1. Implementar `src-tauri/src/transcripts.rs`:
   - Listar todos `.jsonl` em `~/.claude/projects/**/`
   - Filtrar por data (últimos N dias)
   - Agregar: mensagens, tool calls, tokens por modelo
2. Expor comando `get_token_stats(days: u32)`
3. Frontend: gráfico de barras com recharts agrupado por modelo

**Critério de aceitação:** stats batem com `claude --usage` ou similar.

---

### Fase 6 — Tray icon
**Objetivo:** ícone no panel do COSMIC.

1. Implementar `src-tauri/src/tray.rs`:
   - Criar tray icon com 3 estados (verde/laranja/vermelho)
   - Atualizar cor baseado no maior valor entre 5h% e 7d%
   - Texto opcional com % (lê de preferências)
   - Click → mostra/esconde janela popup
2. Validar que aparece no COSMIC panel (Status Area)

**Critério de aceitação:** ícone aparece no panel, troca de cor conforme uso, click abre popup.

---

### Fase 7 — Preferências
**Objetivo:** UI de configurações persistentes.

1. Implementar `src-tauri/src/config.rs`:
   - Ler/escrever `~/.config/claudeview/config.toml`
   - Defaults sensatos se arquivo não existe
2. Frontend: aba/modal de Preferences com todos os toggles
3. Implementar autostart via `tauri-plugin-autostart`

**Critério de aceitação:** mudanças persistem após restart; toggle de autostart funciona.

---

### Fase 8 — Notificações & polish
**Objetivo:** UX final.

1. Notificações desktop em 80% e 95% (com `tauri-plugin-notification`)
2. Animações suaves nas barras
3. Dark/light mode seguindo tema do sistema
4. Empty states (quando não há sessões ativas, quando cache não existe ainda)
5. Hotkey global (opcional)

**Critério de aceitação:** UX equivalente ou superior à versão macOS.

---

### Fase 9 — Packaging
**Objetivo:** distribuição.

1. Configurar `tauri build` pra gerar:
   - `.deb` (Pop!_OS / Ubuntu / Debian)
   - `.rpm` (Fedora)
   - `.AppImage` (universal)
2. Script `make install` que:
   - Builda o `.deb`
   - Instala via `sudo dpkg -i`
   - Instala o statusline hook
   - Habilita autostart
3. README com instruções de instalação

**Critério de aceitação:** usuário faz `make install` e tudo funciona end-to-end.

---

## 6. Makefile (referência)

```makefile
.PHONY: dev build app deb rpm appimage install uninstall hook restart

dev:
	cargo tauri dev

build:
	cargo tauri build

deb:
	cargo tauri build --bundles deb

rpm:
	cargo tauri build --bundles rpm

appimage:
	cargo tauri build --bundles appimage

hook:
	bash scripts/install_hook.sh

install: build hook
	sudo dpkg -i src-tauri/target/release/bundle/deb/claudeview_*.deb
	bash scripts/enable_autostart.sh

uninstall:
	sudo dpkg -r claudeview
	rm -f ~/.config/autostart/claudeview.desktop
	# Hook permanece (usuário remove manualmente se quiser)

restart:
	pkill claudeview || true
	claudeview &
```

---

## 7. Requisitos de ambiente

### 7.1 Desenvolvimento
- Pop!_OS 22.04+ ou Ubuntu 22.04+
- Rust 1.75+ (`rustup`)
- Node.js 20+ (`nvm`)
- `pnpm` ou `npm`
- Dependências de sistema:
  ```bash
  sudo apt install -y \
    libwebkit2gtk-4.1-dev \
    build-essential \
    curl wget file \
    libxdo-dev \
    libssl-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    libxkbcommon-dev
  ```

### 7.2 Runtime (usuário final)
- Pop!_OS 24.04+ (COSMIC) idealmente
- Claude Code CLI instalado
- `python3` (pro hook)

---

## 8. Estilo de código

### Rust
- `rustfmt` configurado com defaults
- `clippy` sem warnings antes de commitar
- Errors com `anyhow::Result` no top level, `thiserror` em libs internas
- Async/await com Tokio
- Logs com `tracing`

### TypeScript
- ESLint + Prettier
- Strict mode no `tsconfig.json`
- Componentes funcionais com hooks (sem class components)
- Tipos explícitos em retornos de funções públicas
- Sem `any` (usar `unknown` se precisar)

### Commits
- Conventional Commits (`feat:`, `fix:`, `chore:`, etc.)
- Mensagens em inglês

---

## 9. Diferenças vs versão macOS

| Aspecto | macOS | Linux/COSMIC |
|---|---|---|
| Linguagem principal | Swift | Rust |
| UI | AppKit (nativo) | React via Tauri webview |
| Menu bar | NSStatusItem | tray-icon crate |
| Autostart | Launch Agent (plist) | XDG autostart (.desktop) |
| Packaging | .app + .dmg | .deb / .rpm / .AppImage |
| Path do cache | `~/.claude/claudeview_cache.json` | **idem** (sem mudança) |
| Hook | Python | **idem** (sem mudança) |

A lógica do **hook é 100% portável** — esse foi o ponto-chave da decisão de arquitetura no macOS.

---

## 10. Como o Claude Code deve trabalhar este projeto

### Instruções diretas

1. **Trabalhe em fases.** Implemente Fase 1 completa antes de passar pra Fase 2. Não pule etapas.

2. **Valide cada fase** com o critério de aceitação antes de seguir adiante. Se não conseguir validar, pare e peça ajuda.

3. **Não invente requisitos.** Se algo não está neste documento, pergunte antes de assumir.

4. **Commits pequenos e frequentes** dentro de cada fase. Um commit por sub-task da checklist.

5. **README primeiro.** Crie um README minimalista no início, expanda em cada fase.

6. **Testes onde fizer sentido.** Não precisa cobertura 100%, mas a lógica de parsing do cache e dos transcripts merece testes unitários.

7. **Pergunte antes de adicionar dependências** que não estão listadas na seção 4.6/4.7.

8. **Erros silenciosos são proibidos.** Todo `Result` deve ser tratado (ou propagado com `?`). Use `tracing` pra logs estruturados.

9. **Não toque em `~/.claude/settings.json` sem fazer backup.** O script de install do hook deve criar `settings.json.bak` antes.

10. **Performance:** o app roda sempre em background. Watcher e polling devem ser leves. Refresh de stats não deve travar a UI.

---

## 11. Recursos de referência

- [Tauri 2.x docs](https://v2.tauri.app/)
- [tray-icon crate](https://docs.rs/tray-icon/)
- [notify crate](https://docs.rs/notify/)
- [sysinfo crate](https://docs.rs/sysinfo/)
- [Claude Code hooks docs](https://docs.claude.com/en/docs/claude-code/hooks)
- [COSMIC panel applets](https://pop-os.github.io/libcosmic-book/panel-applets.html)
- Versão macOS de referência: `~/projects/claudeview-macos/` (Swift)

---

## 12. Definition of Done (projeto inteiro)

- [ ] Todas as fases 1-9 completas com critérios de aceitação validados
- [ ] README completo com screenshots, instalação, troubleshooting
- [ ] `make install` funciona zero-touch em uma máquina Pop!_OS limpa
- [ ] Tray icon aparece no COSMIC panel e troca de cor
- [ ] Dados de subscription atualizam em tempo real após interação com Claude Code
- [ ] Active sessions listadas corretamente
- [ ] Token stats batem com o cálculo manual
- [ ] Preferences persistem
- [ ] Autostart funciona
- [ ] Notificações disparam nos thresholds configurados
- [ ] App não vaza memória após 24h rodando
- [ ] Uso de CPU em idle < 1%
- [ ] Binário final < 20 MB

---

**Versão:** 1.0
**Data:** 2026-05-24
**Autor:** Flavio Gabriel (Gabdevjs)
**Baseado em:** ClaudeView for macOS v1.x
