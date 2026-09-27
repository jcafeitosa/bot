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

- **Estado:** draft. Nenhum gate aprovado. Precisa de Critic independente (G1) e de acordo do Julio sobre o seam antes do primeiro teste.
- **Plano:** W0-04 em [master-plan](../planning/master-plan.md) §4.1 (recomendação lá: validar cedo agora; suportar `15m` depois, se houver demanda).
- **Contratos existentes:** [monitor-persistence-policy-sdd](./monitor-persistence-policy-sdd.md) (C16: requisito inválido encerra antes de rede/TUI; só candles `1m` entram na persistência).

## Contexto (evidência no código, HEAD `72eb471d`)

- `backend/src/core/config/bot.toml:8`: `timeframe = "15m"` no arquivo distribuído.
- `backend/src/core/database/monitor_bootstrap.rs:52-54`: com opt-in ligado e timeframe ≠ `1m`, o boot falha com `UnsupportedTimeframe` ("PERSIST_MARKET_DATA requires market.timeframe=1m").
- O opt-in pode vir de env ou de `system.toml` `[monitor] persist_market_data` (`core/config/monitor/file.rs:23-35`).
- `backend/src/modules/market/controllers/feed.rs:11-13`: o WS só é usado com `1m`; com `15m` o feed é só REST de candles de 15m, que não viram `candles_1m`.
- A mesma regra existe duplicada em `backend/src/modules/monitor/controllers/startup.rs:26-51` (`bootstrap_monitor`, com `#![allow(dead_code)]` na linha 1). Os testes C16 exercitam essa cópia injetável; o caminho de produção (`main.rs:112-116, 146-150`) chama `core/database/monitor_bootstrap.rs`.
- `backend/tests/monitor_startup_cli.rs:8-14` troca o `bot.toml` para `1m` antes de testar; nenhum teste roda o binário com o `bot.toml` padrão e opt-in ligado.

Conclusão: o boot já falha cedo e de forma visível com a config padrão + opt-in. Não há perda silenciosa. Falta: mensagem que diga o que fazer, teste pelo binário com o arquivo padrão e uma única implementação da regra.

## Contradições doc × código

- O C16 em [monitor-persistence-policy-sdd](./monitor-persistence-policy-sdd.md) descreve a validação por um "seam de bootstrap injetável" testado; esse seam (`startup.rs`) não é o que roda em produção.
- A auditoria de 2026-09-27 dizia "boot falha se habilitado" como defeito; pelo contrato C16, falhar é o comportamento correto. O defeito é o padrão distribuído ser incompatível sem aviso.

## Decisão

1. Manter a exigência de `1m` para persistência e a falha no boot.
2. Mensagem acionável e estável, sem URL: dizer o valor atual e as duas saídas (`market.timeframe = "1m"` ou desligar `PERSIST_MARKET_DATA`), e de onde veio o opt-in (env ou `system.toml`).
3. Uma só implementação da regra: `core/database/monitor_bootstrap.rs` vira a fonte; `modules/monitor/controllers/startup.rs` passa a delegar (ou é removido, com os testes movidos para o seam de produção).
4. Comentário no `bot.toml` ao lado de `timeframe` avisando que persistência exige `1m`.

**Onde validar (diferença deliberada do plano):** o critério do plano cita `Config::validate`. Proponho manter a regra no bootstrap do monitor, que já roda antes de rede e TUI, porque `Config::validate` também roda para `serve` sem monitor e para os outros subcomandos, e o opt-in pode vir de env, que o `Config` não lê. Se o owner preferir `Config::validate`, a regra passa a receber o opt-in como parâmetro; os critérios A1–A3 não mudam.

**Alternativa considerada:** suportar `15m` com persistência, abrindo um feed `1m` (WS + janela REST) só para gravar, independente do timeframe da estratégia. Resolve o caso de uso, mas é feature (novo feed, nova reconciliação de janelas, mais carga de rede) e muda o contrato C16/C17. Fica para depois, se houver demanda.

## Seams públicos para acordo antes do TDD

| Seam | Proposta |
|---|---|
| Texto do erro | `PERSIST_MARKET_DATA requires market.timeframe=1m (current: <tf>, source: env\|system.toml); set market.timeframe = "1m" or disable PERSIST_MARKET_DATA` |
| Local da regra | `core::database::monitor_bootstrap::bootstrap_monitor_postgres` (única) |
| `startup.rs` | delegar ou remover (decisão do Builder com o Critic) |

## Critérios de aceite

- A1. Teste CLI em `backend/tests/monitor_startup_cli.rs` com o `bot.toml` padrão (sem trocar `15m`) e `PERSIST_MARKET_DATA=1`: exit ≠ 0 antes de "Starting trading monitor"/"Exchange registry loaded"; saída contém o texto acordado; não contém a URL.
- A2. Mesmo cenário com o opt-in vindo de `system.toml` (sem env): mesma falha, com `source: system.toml`.
- A3. `serve --with-monitor` com a mesma config falha igual (mesmo caminho em `main.rs`).
- A4. Uma única função com a regra de timeframe; os testes C16 passam contra ela.

## Dependências

- W0-03 antes: depois dele, o opt-in é o único caminho de PG do monitor, e esta mensagem passa a ser a única explicação para "snapshot indisponível".
- Não precisa de PG; G4 não depende de W0-02.

## Riscos

- Mudar o texto do erro quebra quem compara a mensagem antiga (os testes CLI atuais usam outras mensagens).
- Remover `startup.rs` mexe na superfície `pub use` de `modules/monitor/mod.rs:16`; conferir os usos antes.

## Validação

- `backend/scripts/verify-backend-gates.sh` (inclui `monitor_startup_cli`).

## Rollout / rollback

- Rollout: próximo build; sem mudança para quem usa `1m` ou persistência desligada. Nenhum deploy autorizado.
- Rollback: reverter o commit; sem migração nem dado.
