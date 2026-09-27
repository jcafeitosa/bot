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

- **Estado:** draft, ciclo 3 do Builder depois do Critic G1 ciclo 2 (APROVADO COM FOLLOW-UP). Nenhum gate aprovado. Precisa de Critic independente (G1) e acordo do Julio sobre o seam antes do primeiro teste (seam **a acordar com o owner**).
- **Plano:** W0-03 em [master-plan](../planning/master-plan.md) §4.1; vem logo depois de W0-01 na ordem de risco.
- **Contratos que esta fatia restaura:** [monitor-persistence-policy-sdd](./monitor-persistence-policy-sdd.md) (contrato 1 e "Inicialização e autoridade do estado": "Com opt-in desligado, `DATABASE_URL` não inicia conexão nem migração"; "A migration não roda só porque `DATABASE_URL` existe").
- **SDD que introduziu a regressão:** [monitor-persistence-c17-sdd](./monitor-persistence-c17-sdd.md) fatia 1.
- **Security:** não há threat model dedicado em `backend/docs/security/`; o risco é migração implícita em banco não pretendido. Critérios de segurança adicionais, se houver, vêm do bot Segurança.

## Contexto (evidência no código, HEAD `d42b71a5`)

Entre `b8370a75` e `d42b71a5`, `postgres.rs` perdeu 2 linhas (o revert tirou `DatasetManifestConflict`); as referências abaixo foram conferidas de novo em `d42b71a5`.

- Caminho de produção do opt-in: `main.rs:112-117` (`serve --with-monitor`) e `main.rs:146-151` (monitor TUI) chamam `AppDatabases::bootstrap_monitor_postgres` (`core/database/bundle.rs:109-114`), que delega a `core/database/monitor_bootstrap.rs:45-65`. Sem `PERSIST_MARKET_DATA`, devolve `None` sem ler URL.
- `backend/src/modules/monitor/controllers/supervisor.rs:895-899` (em `run_with_agent_hook_inner`, `:874`): quando `database` é `None` (opt-in desligado), chama `AppDatabases::optional_postgres_for_monitor_supervisor_snapshot()`.
- `core/database/bundle.rs:120-142`: essa função lê `DATABASE_URL`, conecta e roda **todas** as migrações (`db.migrate()`), com falhas só em `warn` (mensagens `supervisor snapshot: …`).
- `Database::migrate` (`core/database/postgres.rs:94-104`) também recarrega `provider_credentials` para a memória do processo do monitor (`:99-102`).
- Resultado: com só `DATABASE_URL` no `.env` (caso comum, porque o `serve` usa a mesma variável), o monitor TUI e o headless (`serve --with-monitor`, `main.rs:106-136`) conectam e migram `trading_bot` mesmo com persistência desligada.
- O guard de nome (`postgres.rs:57-58`, só `trading_bot`) não distingue banco de dev, de teste ou operacional.
- Ponto de sincronização útil para teste: `spawn_headless_for_api` (`supervisor.rs:1140-1153`) só devolve o `MonitorHandle` quando `run_with_agent_hook_inner` envia o handle (`supervisor.rs:990-992`), **depois** da decisão do snapshot (`:895-899`) e da hidratação (`:962`).

## Contradições doc × doc × código

- [monitor-persistence-policy-sdd](./monitor-persistence-policy-sdd.md) proíbe conectar/migrar sem opt-in; [monitor-persistence-c17-sdd](./monitor-persistence-c17-sdd.md) fatia 1 diz "com `DATABASE_URL` → `trading_bot`, o monitor conecta/migra (best-effort)". O código segue a C17 fatia 1.
- **Cópia do bootstrap fora do caminho de produção:** `modules/monitor/controllers/startup.rs` tem `#![allow(dead_code)]` (`:1`) e uma segunda implementação do bootstrap (`bootstrap_monitor`, `:26-51`; `bootstrap_monitor_database`, `:57`), reexportada em `modules/monitor/mod.rs:16-19`. Os testes C16 (`startup.rs:86-227`, ex. `off_does_not_read_url_or_connect`) exercitam essa cópia, não `core/database/monitor_bootstrap.rs`, que é o que `main.rs` usa. Por isso os testes C16 verdes não provam nada sobre o caminho real, e nenhum deles passa pelo `supervisor`. A remoção da cópia fica com [W0-04](./wave0-04-persistencia-timeframe-padrao-sdd.md), que já propõe uma implementação só; W0-03 não mexe em `startup.rs`, mas também não usa a cópia como evidência.
- O nome "C17" é usado em dois sentidos (fase C17 do T-15 e "C17 fatia 1/2" do snapshot); o master-plan pede nome sem colisão. Este SDD chama a mudança só de W0-03.

## Decisão

1. Remover o caminho `optional_postgres_for_monitor_supervisor_snapshot`. O snapshot do supervisor usa apenas o `database` já validado pelo bootstrap do monitor (existe só com `PERSIST_MARKET_DATA` ligado).
2. Com opt-in desligado: nenhuma leitura de `DATABASE_URL`, nenhuma conexão, nenhuma migração no processo do monitor. Snapshot fica desligado (log `info` único dizendo por quê).
3. O `serve` continua usando `DATABASE_URL` para os seus stores (contrato do `serve`, fora desta fatia); a mudança vale para o que o monitor abre por conta própria.
4. **Flag: reusar `PERSIST_MARKET_DATA`** (decisão recomendada, fechada por recomendação e não bloqueia G1), conforme o contrato 1 de [monitor-persistence-policy-sdd](./monitor-persistence-policy-sdd.md): um único opt-in governa todo acesso do monitor ao PG.

Consequência conhecida: com o `bot.toml` padrão (`15m`) e o opt-in ligado, o boot falha alto antes do bind e da TUI (`UnsupportedTimeframe`, `monitor_bootstrap.rs:52-54`); W0-04 mantém `1m` obrigatório e só torna a mensagem acionável, então snapshot com `15m` continua sem suporte (alternativa futura de W0-04).

**Alternativa considerada:** flag própria para o snapshot (ex.: `BOT_MONITOR_SUPERVISOR_SNAPSHOT=1`, lida em `core/config`), que permitiria snapshot sem persistir candles e independente do timeframe. Mais flexível, mas cria segundo caminho de conexão/migração no monitor e mais uma variável, e contraria o contrato 1 da policy. Rejeitada. O owner pode reabrir se quiser snapshot com `15m`: nem esta fatia nem [W0-04](./wave0-04-persistencia-timeframe-padrao-sdd.md) habilitam `15m` (W0-04 mantém `1m` obrigatório), então sem esta alternativa, ou sem a alternativa futura de W0-04, snapshot com `15m` continua sem suporte.

## Seam público (a acordar com o owner antes do TDD)

| Seam | Proposta |
|---|---|
| Fonte do PG do snapshot | só o `Option<Database>` vindo de `bootstrap_monitor_postgres` (sem env) |
| Flag | reusar `PERSIST_MARKET_DATA` (decidido por recomendação) |
| API removida | `AppDatabases::optional_postgres_for_monitor_supervisor_snapshot` (crate-interna) |

## Critérios de aceite

- A1 (RED primeiro, caminho real de produção). Teste `#[tokio::test]` no bin `bot`, sem PG real: sob o lock de env do repo, `PERSIST_MARKET_DATA` ausente e `DATABASE_URL=postgres://u:p@127.0.0.1:<porta>/trading_bot`, onde `<porta>` é de um `TcpListener` do próprio teste que conta `accept`s e fecha cada socket na hora. O teste chama `spawn_headless_for_api(config, None, hook_noop)` — o `None` é exatamente o que `bootstrap_monitor_postgres` devolve com opt-in desligado — e, quando o handle volta (depois de `supervisor.rs:895-899`), confere `accepts == 0` e aborta a task. Hoje o caminho `supervisor` → `bundle.rs:121-142` conecta, então o teste falha (accepts ≥ 1); depois da correção passa. O teste **não** usa `modules/monitor/controllers/startup.rs` nem os seams injetáveis dos testes C16, e não depende de nenhum seam que a correção remove. Config com timeframe padrão `15m` (sem WS de 1m); se algo antes de `:990` fizer rede, o Builder injeta a fonte de mercado fake já usada nos testes de `supervisor.rs`.
- A2. Mesmo cenário do A1: nenhum log `supervisor snapshot: …` (conexão/migração/URL inválida) é emitido; o log `info` de snapshot desligado aparece uma vez.
- A3 (**guarda de regressão, não RED**). Com opt-in ligado e `1m`, o snapshot usa o **mesmo** `database` do bootstrap. No HEAD este teste já passa: no caminho `Some(db)` (`supervisor.rs:895-896`) o supervisor usa o `database` recebido e não lê env. O teste existe para impedir que a correção, ou mudança futura, faça o supervisor abrir conexão própria. Forma: teste PG (no manifesto de `run-pg-integration-tests.sh`) obtém `db` pelo helper de integração, pré-grava uma linha de snapshot nele, aponta `DATABASE_URL` para o listener contador do A1 e chama `spawn_headless_for_api(config_1m, Some(db), hook_noop)`; espera `accepts == 0` (o supervisor não abriu conexão própria) e a linha hidratada (`supervisor.rs:962-982`). `pg_monitor_supervisor_snapshot_round_trip` (`core/database/monitor_supervisor_snapshot.rs:87`) segue no manifesto e verde, mas só cobre save/load do store, não a origem do `database`.
- A4. `rg optional_postgres_for_monitor_supervisor_snapshot backend/src` vazio.
- A5. [monitor-persistence-c17-sdd](./monitor-persistence-c17-sdd.md) fatia 1 corrigido para "só com opt-in" no mesmo entregável.

## Dependências

- Nenhuma fatia W0 bloqueia. W0-04 depende desta (mantém `1m` obrigatório, torna a mensagem de erro acionável e remove a cópia de `startup.rs`); W0-04 não habilita `15m`.
- G4 depende de W0-02 (teste PG só vale se falhar sem PG).

## Riscos

- Quem usa snapshot hoje sem opt-in perde o snapshot (é o objetivo; registrado no rollout).
- `serve --with-monitor` continua com PG via `serve`; o Critic deve conferir que o supervisor não reaproveita o pool do `serve` por outro caminho (A3 cobre a origem do `database`).
- A1 depende de `spawn_headless_for_api` subir sem rede até `supervisor.rs:990`; se não subir, o teste usa a fonte fake (ver A1), sem trocar o caminho de produção testado.

## Validação

- `backend/scripts/verify-backend-gates.sh`; A1/A2 no bin `bot`, sem rede e sem PG; A3 no manifesto PG; `monitor_startup_cli` segue verde.

## Rollout / rollback

- Rollout: próximo build local. Operador que queira snapshot liga `PERSIST_MARKET_DATA=1` com `market.timeframe = "1m"`; W0-04 mantém essa exigência e não habilita `15m`. Nenhum deploy autorizado.
- Rollback: reverter o commit restaura a conexão/migração implícita; sem migração nova nem dado a desfazer.
