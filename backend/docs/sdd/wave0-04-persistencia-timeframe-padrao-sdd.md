---
title: SDD W0-04 — persistência do monitor com o timeframe padrão
description: Fatia Onda 0 que resolve o conflito entre bot.toml (15m) e a exigência de 1m da persistência, com validação clara no boot
tags:
  - sdd
  - backend
  - monitor
  - persistence
  - wave0
status: draft
---

# SDD W0-04 — persistência do monitor com o timeframe padrão

- **Estado:** draft, ajustes do follow-up do G1 ciclo 1 aplicados. Nenhum gate aprovado aqui. Precisa de acordo do Julio sobre os seams antes do primeiro teste.
- **Plano:** W0-04 em [master-plan](../planning/master-plan.md) §4.1 (recomendação lá: validar cedo agora; suportar `15m` depois, se houver demanda).
- **Contratos existentes:** [monitor-persistence-policy-sdd](./monitor-persistence-policy-sdd.md) (C16: requisito inválido encerra antes de rede/TUI; só candles `1m` entram na persistência).

## Contexto (evidência no código, HEAD `d42b71a5`)

- `backend/src/core/config/bot.toml:8`: `timeframe = "15m"` no arquivo distribuído.
- `backend/src/core/database/monitor_bootstrap.rs:52-54`: com opt-in ligado e timeframe ≠ `1m`, o boot falha com `UnsupportedTimeframe` ("PERSIST_MARKET_DATA requires market.timeframe=1m").
- O opt-in pode vir de env ou de `system.toml` `[monitor] persist_market_data` (`core/config/monitor/file.rs:23-35`). `persist_market_data_flag_raw` devolve só `Result<Option<String>, MonitorEnvError>`: a origem se perde (com `system.toml` ele devolve `"true"` sintético). `main.rs:18-20` (`read_persist_flag`) repassa esse valor para `bootstrap_monitor_postgres` (`main.rs:112-116, 146-150`).
- `backend/src/modules/market/controllers/feed.rs:11-13`: o WS só é usado com `1m`; com `15m` o feed é só REST de candles de 15m, que não viram `candles_1m`.
- A mesma regra existe duplicada em `backend/src/modules/monitor/controllers/startup.rs:26-51` (`bootstrap_monitor`, com `#![allow(dead_code)]` na linha 1). Os testes C16 exercitam essa cópia injetável; o caminho de produção (`main.rs:112-116, 146-150`) chama `core/database/monitor_bootstrap.rs`.
- `backend/tests/monitor_startup_cli.rs:8-14` troca o `bot.toml` para `1m` antes de testar; nenhum teste roda o binário com o `bot.toml` padrão e opt-in ligado.

Conclusão: o boot já falha cedo e de forma visível com a config padrão + opt-in, **antes do bind do `serve --with-monitor` e antes da TUI**. Não há perda silenciosa nem estado "rodando sem snapshot" por causa do timeframe: o processo não sobe. Falta: mensagem que diga o que fazer, teste pelo binário com o arquivo padrão e uma única implementação da regra.

## Contradições doc × código

- O C16 em [monitor-persistence-policy-sdd](./monitor-persistence-policy-sdd.md) descreve a validação por um "seam de bootstrap injetável" testado; esse seam (`startup.rs`) não é o que roda em produção.
- A auditoria de 2026-09-27 dizia "boot falha se habilitado" como defeito; pelo contrato C16, falhar é o comportamento correto. O defeito é o padrão distribuído ser incompatível sem aviso.

## Decisão

1. Manter a exigência de `1m` para persistência e a falha no boot. W0-04 **não** habilita persistência nem snapshot com `15m`; isso é a alternativa futura abaixo.
2. Mensagem acionável e estável, sem URL: dizer o valor atual e as duas saídas (`market.timeframe = "1m"` ou desligar `PERSIST_MARKET_DATA`), e de onde veio o opt-in (env ou `system.toml`).
3. Uma só implementação da regra: `core/database/monitor_bootstrap.rs` vira a fonte; `modules/monitor/controllers/startup.rs` passa a delegar (ou é removido, com os testes movidos para o seam de produção).
4. Comentário no `bot.toml` ao lado de `timeframe` avisando que persistência exige `1m`.

**Onde validar (decidido):** a regra fica no bootstrap do monitor (o critério do plano citava `Config::validate`; o master plan será ajustado para bater com isto). Motivo: o bootstrap já roda antes de rede e TUI, e `Config::validate` também roda para `serve` sem monitor e para os outros subcomandos, e o opt-in pode vir de env, que o `Config` não lê. **Alternativa registrada, reversível pelo owner:** mover para `Config::validate`, com o opt-in passado como parâmetro; os critérios A1–A3 não mudam.

**Alternativa considerada:** suportar `15m` com persistência, abrindo um feed `1m` (WS + janela REST) só para gravar, independente do timeframe da estratégia. Resolve o caso de uso, mas é feature (novo feed, nova reconciliação de janelas, mais carga de rede) e muda o contrato C16/C17. Fica para depois, se houver demanda.

## Seams públicos para acordo antes do TDD

| Seam | Proposta |
|---|---|
| Texto do erro | `PERSIST_MARKET_DATA requires market.timeframe=1m (current: <tf>, source: env\|system.toml); set market.timeframe = "1m" or disable PERSIST_MARKET_DATA` |
| Local da regra | `core::database::monitor_bootstrap::bootstrap_monitor_postgres` (única) |
| Origem do opt-in | `persist_market_data_flag_raw` passa a devolver `Result<Option<PersistFlag>, MonitorEnvError>`, com `PersistFlag { raw: String, source: PersistFlagSource }` e `PersistFlagSource::{Env, SystemToml}`. Muda a assinatura pública em `core/config/monitor/file.rs:23`, o reexport em `core/config/mod.rs:37` e `core/config/monitor/mod.rs:3`, e o chamador `main.rs:18-20`; `bootstrap_monitor_postgres` recebe a origem para montar a mensagem |
| `startup.rs` | remover e mover os testes C16 para o seam de produção. Mexe no `pub use controllers::startup::{…}` de `modules/monitor/mod.rs:16-19` (`bootstrap_monitor`, `bootstrap_monitor_database`, `connect_database`, `persistence_required`, `MonitorStore`, `StartupError`). `StartupError` é usado em produção (`main.rs:8, 11-13, 18`) e tem de mudar de lugar (ex.: `core::database::monitor_bootstrap`) antes da remoção. Alternativa: `startup.rs` delegar para o bootstrap de produção |

## Critérios de aceite

- A1. Teste CLI em `backend/tests/monitor_startup_cli.rs` com o `bot.toml` padrão (sem trocar `15m`) e `PERSIST_MARKET_DATA=1`: exit ≠ 0 antes de "Starting trading monitor"/"Exchange registry loaded"; saída contém o texto acordado; não contém a URL.
- A2. Mesmo cenário com o opt-in vindo de `system.toml` (sem env): mesma falha, com `source: system.toml`.
- A3. `serve --with-monitor` com a mesma config falha igual (mesmo caminho em `main.rs`).
- A4. Uma única função com a regra de timeframe; os testes C16 passam contra ela.

## Dependências

- W0-03 antes: depois dele, o opt-in é o único caminho de PG do monitor. Com `15m` + opt-in, o resultado continua sendo falha alta no boot (não "snapshot indisponível"); W0-04 só torna a mensagem acionável.
- Não precisa de PG; G4 não depende de W0-02.

## Riscos

- Mudar o texto do erro quebra quem compara a mensagem antiga (os testes CLI atuais usam outras mensagens).
- Remover `startup.rs` mexe na superfície `pub use` de `modules/monitor/mod.rs:16-19` e no import de `StartupError` em `main.rs:8`; conferir todos os usos com `rg` antes.

## Validação

- `backend/scripts/verify-backend-gates.sh` (inclui `monitor_startup_cli`).

## Rollout / rollback

- Rollout: próximo build; sem mudança para quem usa `1m` ou persistência desligada. Nenhum deploy autorizado.
- Rollback: reverter o commit; sem migração nem dado.
