---
title: SDD W0-03 — monitor com persistência desligada não abre nem migra PG
description: Fatia Onda 0 que corrige a regressão da C17 fatia 1 (0011) em que o monitor conecta e migra PostgreSQL só porque DATABASE_URL existe
tags:
  - sdd
  - backend
  - monitor
  - persistence
  - security
  - wave0
status: draft
---

# SDD W0-03 — monitor com persistência desligada não abre nem migra PG [SEGURANÇA]

- **Estado:** draft. Nenhum gate aprovado. Precisa de Critic independente (G1) e acordo do Julio sobre o seam antes do primeiro teste.
- **Plano:** W0-03 em [master-plan](../planning/master-plan.md) §4.1; por decisão do Critic, vem logo depois de W0-01 na ordem de risco.
- **Contratos que esta fatia restaura:** [monitor-persistence-policy-sdd](./monitor-persistence-policy-sdd.md) (contrato 1 e "Inicialização e autoridade do estado": "Com opt-in desligado, `DATABASE_URL` não inicia conexão nem migração"; "A migration não roda só porque `DATABASE_URL` existe").
- **SDD que introduziu a regressão:** [monitor-persistence-c17-sdd](./monitor-persistence-c17-sdd.md) fatia 1.
- **Security:** não há threat model dedicado em `backend/docs/security/`; o risco é migração implícita em banco não pretendido. Critérios de segurança adicionais, se houver, vêm do bot Segurança.

## Contexto (evidência no código, HEAD `b2001a7c`)

- `backend/src/core/database/monitor_bootstrap.rs:44-64`: `bootstrap_monitor_postgres` respeita o opt-in — sem `PERSIST_MARKET_DATA`, devolve `None` sem ler URL.
- `backend/src/modules/monitor/controllers/supervisor.rs:894-899`: quando `database` é `None` (opt-in desligado), chama `AppDatabases::optional_postgres_for_monitor_supervisor_snapshot()`.
- `backend/src/core/database/bundle.rs:120-141`: essa função lê `DATABASE_URL`, conecta e roda **todas** as migrações (`db.migrate()`), com falhas só em `warn`.
- `Database::migrate` (`postgres.rs:94-98`) também recarrega `provider_credentials` para a memória do processo do monitor.
- Resultado: com só `DATABASE_URL` no `.env` (caso comum, porque o `serve` usa a mesma variável), o monitor TUI e o headless (`serve --with-monitor`, `main.rs:106-120`) conectam e migram `trading_bot` mesmo com persistência desligada.
- O guard de nome (`postgres.rs:58`, só `trading_bot`) não distingue banco de dev, de teste ou operacional.

## Contradições doc × doc × código

- [monitor-persistence-policy-sdd](./monitor-persistence-policy-sdd.md) proíbe conectar/migrar sem opt-in; [monitor-persistence-c17-sdd](./monitor-persistence-c17-sdd.md) fatia 1 diz "com `DATABASE_URL` → `trading_bot`, o monitor conecta/migra (best-effort)". O código segue a C17 fatia 1.
- O nome "C17" é usado em dois sentidos (fase C17 do T-15 e "C17 fatia 1/2" do snapshot); o master-plan pede nome sem colisão. Este SDD chama a mudança só de W0-03.

## Decisão

1. Remover o caminho `optional_postgres_for_monitor_supervisor_snapshot`. O snapshot do supervisor usa apenas o `database` já validado pelo bootstrap do monitor (existe só com `PERSIST_MARKET_DATA` ligado).
2. Com opt-in desligado: nenhuma leitura de `DATABASE_URL`, nenhuma conexão, nenhuma migração no processo do monitor. Snapshot fica desligado (log `info` único dizendo por quê).
3. O `serve` continua usando `DATABASE_URL` para os seus stores (contrato do `serve`, fora desta fatia); a mudança vale para o que o monitor abre por conta própria.

Consequência conhecida: com o `bot.toml` padrão (`15m`) o opt-in hoje é recusado no boot (`UnsupportedTimeframe`), então o snapshot fica indisponível até W0-04.

**Alternativa considerada:** flag própria para o snapshot (ex.: `BOT_MONITOR_SUPERVISOR_SNAPSHOT=1`, lida em `core/config`), que permitiria snapshot sem persistir candles e independente do timeframe. Mais flexível, mas cria segundo caminho de conexão/migração no monitor e mais uma variável; só vale se o owner quiser snapshot com `15m` antes de W0-04.

## Seam público para acordo antes do TDD

| Seam | Proposta |
|---|---|
| Fonte do PG do snapshot | só o `Option<Database>` vindo de `bootstrap_monitor_postgres` (sem env) |
| Flag | reusar `PERSIST_MARKET_DATA` (decisão) ou flag nova (alternativa) — precisa de escolha do owner |
| API removida | `AppDatabases::optional_postgres_for_monitor_supervisor_snapshot` (crate-interna) |

## Critérios de aceite

- A1. Com `PERSIST_MARKET_DATA` ausente/`0`/`false` e `DATABASE_URL` definida, o monitor não chama conexão nem migração: teste com seam de bootstrap injetável que conta chamadas (esperado 0), sem banco real, no padrão dos testes C16 de [monitor-persistence-policy-sdd](./monitor-persistence-policy-sdd.md).
- A2. Mesmo cenário com `DATABASE_URL` apontando para host inexistente: nenhum log `supervisor snapshot: …` de conexão/migração.
- A3. Com opt-in ligado e `1m`, o snapshot continua sendo gravado (teste PG `pg_monitor_supervisor_snapshot_round_trip` segue no manifesto e verde).
- A4. `rg optional_postgres_for_monitor_supervisor_snapshot backend/src` vazio.
- A5. [monitor-persistence-c17-sdd](./monitor-persistence-c17-sdd.md) fatia 1 corrigido para "só com opt-in" no mesmo entregável (hoje o arquivo tem alterações não commitadas de outra sessão; editar só quando estiver limpo).

## Dependências

- Nenhuma fatia W0 bloqueia. W0-04 depende desta (persistência com timeframe padrão).
- G4 depende de W0-02 (teste PG só vale se falhar sem PG).

## Riscos

- Quem usa snapshot hoje sem opt-in perde o snapshot (é o objetivo; registrado no rollout).
- `serve --with-monitor` continua com PG via `serve`; o Critic deve conferir que o supervisor não reaproveita o pool do `serve` por outro caminho.

## Validação

- `backend/scripts/verify-backend-gates.sh`; testes novos no bin `bot`, sem rede e sem PG; `monitor_startup_cli` segue verde.

## Rollout / rollback

- Rollout: próximo build local. Operador que queira snapshot liga `PERSIST_MARKET_DATA=1` (com `1m` até W0-04). Nenhum deploy autorizado.
- Rollback: reverter o commit restaura a conexão/migração implícita; sem migração nova nem dado a desfazer.
