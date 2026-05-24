# ClaudeView (Linux / COSMIC)

Porta do app macOS [ClaudeView](#) para Linux com integração ao COSMIC Desktop.
Monitora em tempo real o uso da subscription do Claude Code, sessões ativas e
estatísticas de tokens — direto da barra do sistema.

Status: **Fase 1 — scaffolding**. Veja `PRA.md` para o plano completo.

## Stack

- Tauri 2.x + Rust (backend)
- TypeScript + React + Vite (frontend)
- pnpm (package manager)

## Pré-requisitos

- Pop!_OS 22.04+ ou Ubuntu 22.04+
- Rust 1.75+ (`rustup`)
- Node.js 20+
- pnpm 9 (`npm install -g pnpm@9`)
- Deps de sistema:
  ```bash
  sudo apt install -y \
    libwebkit2gtk-4.1-dev build-essential curl wget file \
    libxdo-dev libssl-dev libayatana-appindicator3-dev \
    librsvg2-dev libxkbcommon-dev
  ```

## Rodar em modo dev

```bash
pnpm install
make dev
```

Abre uma janela com "Hello ClaudeView".
