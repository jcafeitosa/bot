---
title: SDD — Extração do core (fase 1)
description: Primeira fatia da proposta 0001 — agrupar config, error, logging e persistence em src/core
tags:
  - sdd
  - backend
  - architecture
  - core
status: draft
---
# SDD — Extração do `core` (fase 1)

- **Estado:** draft (implementação da fatia 1).
- **Referência:** [Proposta 0001 — core e módulos](../proposals/0001-backend-core-modules-mvc.md), fatia de migração equivalente ao passo inicial de reorganização física do núcleo compartilhado.
- **Gate G1:** este documento precede a implementação G3.

## Contexto

O backend Rust declara hoje `config`, `error`, `logging` e `persistence` como módulos raiz em `main.rs`. A proposta 0001 define um diretório `core/` para capacidades compartilhadas sem política de domínio de módulos de negócio, com direção de imports verificável (`core` não depende de `modules/*` nem `presentation/*`).

Esta fatia **não** altera comportamento de trading, monitor, TUI, exchanges ou estratégia — apenas reposiciona arquivos e atualiza paths/imports.

## Objetivo

1. Introduzir `src/core/mod.rs` com submódulos públicos `config`, `error`, `logging`, `persistence`.
2. Mover fisicamente os artefatos atuais para `src/core/`.
3. Atualizar imports do crate para `crate::core::{config, error, logging, persistence}` (sem reexports eternos na raiz).
4. Atualizar caminhos documentados e embutidos em código (TOML default, migrações SQLx, diretório de exchanges).

## Não-objetivos (fora desta fatia)

- Mover ou reorganizar `app`, `ui`, `exchanges`, `market`, `market_feed`, `strategy`, `backtest`, `modules/monitor`, `monitor_startup`, `persistence_health`.
- Extrair `OperationMode` para `modules::config` (decisão adiada — ver abaixo).
- Introduzir `presentation/terminal` ou contratos `MonitorCommand`/`MonitorEvent` da proposta.
- Alterar schema SQL, regras de validação de config ou semântica de persistência.
- Script de verificação de grafo de dependências / `cargo deny` (planejado em fatias posteriores).

## Seams públicos (após a fatia)

| Seam | Caminho | Responsabilidade |
|------|---------|------------------|
| `core::config` | `src/core/config/` | CLI/TOML, enums de ambiente/operação/risco, validação estrutural, presets via `strategy::periods_for_mode` (acoplamento pré-existente). |
| `core::error` | `src/core/error.rs` | `BotError`, `BotResult`. |
| `core::logging` | `src/core/logging.rs` | Inicialização de tracing (terminal + arquivo rotativo). |
| `core::persistence` | `src/core/persistence/` | Pool PostgreSQL, migrações, `persist_dataset` genérico. |

## Decisões

### `OperationMode` permanece em `core::config`

Alinhado à árvore alvo do owner para esta fase: o tipo continua definido e validado junto ao loader de configuração. A proposta 0001 discute eventual `modules::config::OperationMode` em fatia 2; **não** duplicar nem mover o enum nesta entrega.

### Paths TOML e migrações (após `git mv`)

| Recurso | Caminho ativo |
|---------|----------------|
| Monitor/backtest TOML | `src/core/config/bot.toml` |
| Contas exchange | `src/core/config/exchanges/*.toml` |
| Migrações SQLx | `src/core/persistence/migrations/` |

**Impacto:** defaults CLI (`--config`), `EXCHANGE_CONFIG_DIR`, `CARGO_MANIFEST_DIR`-joined paths em `persistence::migrate`, testes `include_str!`, README e docs operacionais devem usar os novos prefixos. O cwd de execução continua sendo `backend/` para paths relativos.

### Estratégia de imports

- Preferir **`crate::core::config`**, **`crate::core::error`**, etc., em todo o crate.
- **Não** adicionar `pub use core::*` na raiz de `main.rs` além do necessário em `main` (`use crate::core::config::Config`).
- Sem shims `mod config` na raiz após a fatia.

### Matriz de dependência (fase 1)

```text
main → core, app, exchanges, …
app / exchanges / strategy / … → core (quando necessário)
core::config → core::error, crate::strategy (validação de presets — legado)
core::logging → core::config
core::persistence → crate::market (tipo HistoricalDataset — legado)
core → NÃO importa modules::*, ui, app, exchanges, …
```

`modules/` e `presentation/` ainda não existem como árvore final; `core` não deve importá-los.

## Alternativas consideradas

| Alternativa | Motivo de rejeição nesta fatia |
|-------------|--------------------------------|
| Reexport `pub mod config` na raiz | Viola preferência do owner por imports explícitos `core::`. |
| Manter TOML em `src/config/` só os arquivos | Duplicaria ownership; TOML viaja com o módulo. |
| Crate separado `bot-core` | Custo de fronteira maior; proposta 0001 adia multi-crate. |

## Riscos e mitigação

| Risco | Mitigação |
|-------|-----------|
| Paths quebrados em docs/CI | Atualizar README, testes e constantes na mesma PR/fatia. |
| `config` → `strategy` no core | Documentado como débito; fatia 2 pode inverter via composition em `main`. |
| Churn em grep/docs | Lista de arquivos alterados na entrega G4. |

## Critérios de aceite

1. `src/core/mod.rs` declara os quatro submódulos; `main.rs` declara `mod core` e não declara `mod config|error|logging|persistence` na raiz.
2. Compilação e testes passam com:
   - `cargo fmt`
   - `cargo clippy --locked --all-targets -- -D warnings`
   - `cargo test --locked`
3. Nenhum `use crate::config` / `crate::error` / `crate::logging` / `crate::persistence` remanescente no `src/` e `tests/` do backend (exceto vendor).
4. Caminhos ativos de TOML e migrações apontam para `src/core/config/` e `src/core/persistence/migrations/`.
5. Comportamento observável inalterado (mesmos defaults relativos a `backend/`).

## Rollback

1. `git revert` ou `git mv` inverso: `core/*` de volta à raiz `src/`.
2. Restaurar imports `crate::config` e paths `src/config/`, `src/persistence/migrations/`.
3. Reexecutar a tríade fmt/clippy/test.

## Validação (G4)

```sh
cd backend
cargo fmt
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

Opcional smoke: `cargo run --locked -- backtest --config src/core/config/bot.toml`.

## Pendências para fatia 2

- Mover domínio restante para `modules/*` conforme proposta 0001.
- Resolver acoplamento `core::config` → `strategy` (validação de presets).
- Avaliar `OperationMode` em `modules::config` vs manter em `core::config`.
- Extrair `Signal` para contrato neutro; mover `market_feed`, `persistence_health`.
- Script de direção de imports e gates de reexport temporário da proposta.
