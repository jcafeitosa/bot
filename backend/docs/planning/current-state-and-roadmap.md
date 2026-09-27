---
title: Estado atual e planejamento do backend
description: Inventário do que foi feito, pendências, bloqueios, gates e plano de evolução
tags:
  - planning
  - backend
  - roadmap
  - status
---
# Estado atual e planejamento do backend

> Revisão: 2026-09-27. Estados abaixo distinguem comportamento presente, design aprovado e trabalho ainda bloqueado. Fatia **bots / orders / agents / HTTP:** [auditoria de completude](./modules-completeness-audit.md).

## O que já está feito

### Produto executável

- CLI com caminho de monitor e subcomando de backtest.
- Configuração TOML com overrides de CLI e validação de ambiente, operação, modo, risco e timeframe.
- Monitor terminal com TUI, pausa, retomada e saída.
- Feed híbrido REST/WS para candles; WS limitado a klines fechados de `1m`.
- Validação de janelas REST e deduplicação/ordenação no feed.
- Estratégia SMA (e crossover EMA no monitor/backtest via `MonitorEvaluatorKind`) com períodos por operação ou `[[strategy.monitor_registry]]`.
- Modos observe/paper no monitor; ordens reais **prod** REST bloqueadas; seams HTTP/monitor opt-in (`paper`, `recording`, testnet Spot) via `modules/orders` ([Gate 2](../sdd/orders-live-execution-gate2-sdd.md) parcial).
- Backtest sintético determinístico com taxas, slippage e resumo JSON.
- Persistência PostgreSQL opt-in, migração automática e gravação idempotente de datasets.
- Logging estruturado para stderr e arquivos rotacionados.
- Jev/TypeSafe consultivo opcional sem autoridade operacional.
- Módulos `agents` (IdentityOnly + espelhamento/hidratação PG), `bots` (catálogo/ranking + runtime `BotRuntimePort` + `evaluate_for_kind`) e `orders` (fail-closed + G2 parcial: idempotência/reconciliação PG, poll HTTP) com testes unitários (**387** no bin `bot`, gate `./scripts/verify-backend-gates.sh`).
- API HTTP Axum com OpenAPI/Scalar (**36** paths): agents, bots (catálogo `monitor_evaluator`, runtime promote/demote), risk, strategy, backtest, portfolio, exchanges; orders `execution-status`, `submit`, reconciliação GET/POST poll; `GET /meta` (`http_seams`); seam admin (`BOT_HTTP_ADMIN_TOKEN`, binds owner/agency) — [SDD HTTP admin](../sdd/http-admin-auth-seam-sdd.md); facades documentadas em [module-catalog §3d](../architecture/module-catalog.md#3d-facade-http_bridge-srcmoduleshttp_bridge). Completude: [auditoria](./modules-completeness-audit.md).

### Evidência existente

- Testes de configuração cobrem arquivo padrão, caminho explícito, arquivo ausente e arquivo ilegível.
- Teste de fixture cobre presets/timeframes suportados e fechamento de trade.
- Testes puros de origem cobrem mesma origem, downgrade, userinfo, histórico vazio e limite de redirects.
- Testes HTTP cobrem bloqueio entre origens e aceitação dentro da origem inicial; a execução foi validada fora do sandbox com loopback permitido.
- Teste PostgreSQL existe, mas é ignorado por padrão e exige banco dedicado.
- O [mapa de módulos anterior](../sdd/backend-module-map-sdd.md) e a [arquitetura consolidada](../architecture/backend-module-reference.md) registram os módulos presentes.

## Designs e correções já registrados

| Item | Estado atual | Documento |
|---|---|---|
| T-03 — configuração, mercado e organização | Proposta; Gate 1 não comprovado nesta revisão. | [SDD T-03](../sdd/backend-corrections-sdd.md) |
| T-05 — redirects REST | C9/C10 aprovados com follow-up; G4 ainda pendente. | [SDD T-05](../sdd/rest-redirect-sdd.md) |
| T-07 — fixture e slippage | C12/C13 aprovados com follow-up; G4 documental ainda pendente. | [SDD T-07](../sdd/backtest-trades-and-slippage-sdd.md) |
| T-10 — pausa/retomada | C14/C15 implementados e aprovados com follow-up; C17 ainda depende do sinal de overflow no estado de persistência. | [SDD T-10](../sdd/monitor-pause-resume-sdd.md) |
| T-13 — limpeza de arquivos legados | Design e implementação aprovados; manter verificação do worktree como controle de regressão. | [SDD T-13](../sdd/legacy-file-cleanup-sdd.md) |
| T-15 — persistência opcional | C16 aprovado com follow-up; C17, G4 e V18 continuam pendentes. | [SDD T-15](../sdd/monitor-persistence-policy-sdd.md) |
| T-16 — mapa de módulos | Documentação do README concluída conforme o SDD. | [SDD T-16](../sdd/backend-module-map-sdd.md) |

## Pendências e bloqueios

1. Fechar T-03 com revisão independente e acordo dos seams públicos.
2. Concluir C17 com o sinal de overflow do WS refletido no estado de persistência e sua revisão independente.
3. Executar V18 em PostgreSQL descartável e completar G4 de persistência.
4. Confirmar T-13 no estado atual do worktree e garantir que não há caminhos legados ativos.
5. Revisar o contrato de Jev, timeout, telemetria e comportamento quando o endpoint falha.
6. Definir observabilidade operacional: métricas de WS/REST, estado de persistência, idade do último candle, falhas de Jev e runbook de credenciais.
7. Ingerir localmente as fontes externas da pesquisa de agentes antes de promovê-la a conhecimento consolidado.
8. Acompanhar a [análise de módulos ainda não desenvolvidos](./unimplemented-modules-analysis.md) antes de abrir novos SDDs de runtime.

## Plano recomendado

### P0 — Fechado

- Testes HTTP de redirect executados fora do sandbox: origem externa bloqueada e redirect na mesma origem aceito.
- Confirmada a política instalada no `HttpClient`.
- Manter a proveniência, licença e checksum do vendor sob revisão quando a dependência mudar.

### P1 — Fechar runtime e persistência

- Concluir C17 preservando os estados `DEGRADED`, `HEALTHY` e `GAP`.
- Propagar overflow e descarte WS para o estado de persistência.
- Validar cancelamento, stale results, gaps e recuperação sem transformar falha em execução.
- Executar V18 e registrar a evidência real do PostgreSQL.

### P2 — Fechar integração e operação

- Executar o round-trip PostgreSQL em ambiente descartável.
- Definir logs/contadores para WS, REST, Jev e persistência.
- Documentar rotação de credenciais e recuperação de banco.
- Revisar a matriz de risco em [integrações](../architecture/integrations.md).

### P3 — Consolidar documentação

- Manter o [índice](../index.md), a [arquitetura](../architecture/backend-module-reference.md), as [integrações](../architecture/integrations.md), o [runbook](../operations/runbook.md) e a [referência de CLI](../reference/cli-and-config.md) como entradas canônicas.
- Atualizar SDDs quando o comportamento mudar; não registrar uma intenção como se fosse implementação.
- Ingerir e citar fontes da pesquisa de agentes antes de remover o status provisório.

## Verificação executada em 2026-09-27

```text
./scripts/verify-backend-gates.sh
```

O script executa `cargo fmt --check`, `cargo clippy --locked --bin bot -- -D warnings`, `./scripts/check-import-direction.sh`, `cargo test --locked --bin bot -- --test-threads=1` e cinco suítes em `tests/` (`backtest_fixture`, `config_cli`, `monitor_startup_cli`, `redirect_origin_test`, `redirect_policy_test`) — sem repetir `cargo test --locked` completo (evita flake do bin `bot` em paralelo).

Evidência observada: **387** testes unitários no binário `bot` (OpenAPI **36** paths; orders reconciliação GET + `POST …/poll` + poller testnet observe; PG `0004`/`0006`; paper/testnet/recording; bots runtime/`evaluate_for_kind`; agents promote capability), **13** ignorados (PG×11, Neo4j, testnet ccxt manual), `./scripts/verify-backend-gates.sh` **ok**; PG opcional **11/11** via `run-pg-integration-tests.sh`.

## Gates de aceitação

- **G0:** escopo, usuários e não objetivos registrados.
- **G1:** SDD proporcional aprovado por revisão independente.
- **G2:** plano com dependências e critérios de aceite.
- **G3:** testes red/green e implementação mínima.
- **G4:** verificações executadas com evidência observável.
- **G5:** deploy somente com autorização explícita e rollback operacional.

Nenhum documento desta pasta autoriza deploy. A aceitação final depende dos gates e das evidências listadas nos SDDs.
