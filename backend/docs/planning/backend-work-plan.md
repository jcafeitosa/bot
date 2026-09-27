---
title: Plano de execução das correções pendentes do backend
description: Plano de entregas, gates, dependências e bloqueios do backend
tags:
  - planning
  - backend
  - execution
---

# Plano de execução das correções pendentes do backend

- **Responsável:** Orquestrador `/root`
- **Data:** 2026-09-26
- **Estado:** plano de G2 revisado e aprovado por `/root/workplan_critic`; C9 recebeu parecer complementar independente após os testes HTTP passarem fora do sandbox em 2026-09-27
- **Base:** SDDs [T-05](../sdd/rest-redirect-sdd.md), [T-07](../sdd/backtest-trades-and-slippage-sdd.md), [T-10](../sdd/monitor-pause-resume-sdd.md) e [T-15](../sdd/monitor-persistence-policy-sdd.md), todos com G1 técnico aprovado por críticos independentes

## Escopo e gates

O objetivo é corrigir o risco de redirect REST, a fixture e o custo de venda do backtest, a pausa/retomada do monitor e a semântica da persistência opcional. As entregas [T-13](../sdd/legacy-file-cleanup-sdd.md) e [T-16](../sdd/backend-module-map-sdd.md) já passaram por revisão independente; seus arquivos permanecem no worktree e não são refeitos aqui.

O `AGENTS.md` exige acordo do usuário com os seams públicos antes de escrever cada teste. O usuário aprovou explicitamente as interfaces de T-05, T-07, T-10 e T-15 em 2026-09-26. G2 está fechado quanto a esse requisito. C9 foi revalidado fora do sandbox: os dois testes HTTP obrigatórios e os testes puros de origem passaram. O parecer original de `/root/c9_critic` foi `REPROVADO` pela falta dessa prova; `/root/c16_critic`, instância independente distinta do Builder C9, reexaminou código, vendor e testes e emitiu parecer complementar **APROVADO COM FOLLOW-UP**. C10 verificou o contrato/fallback do produto com fonte simulada e recebeu **APROVADO COM FOLLOW-UP**. C17 está em execução com seu próprio par. O autor nunca aprova o próprio artefato. Cada CL segue testes, revisão e documentação proporcionais; desvios de TDD histórico em C10/C14 estão registrados nas linhas correspondentes.

| CL | Entrega e evidência mínima | Dependência | Builder / Critic |
|---|---|---|---|
| C9 | Política de redirect no `ccxt-core` local: servidores HTTP locais verificam bloqueio antes de contato com outra origem e redirect na mesma origem; testes puros verificam limite de saltos, origem e downgrade; árvore Cargo prova dependência única. | Interface T-05 aprovada | `/root/c9_builder` / `/root/c9_critic` (original), `/root/c16_critic` (reexame) — **G3 APROVADO COM FOLLOW-UP**: repetir testes em futuras atualizações do vendor |
| C10 | Contrato do adaptador e documentação: endpoint testnet e fallback após erro REST verificados; README e risco residual atualizados conforme testes reais. | C9 aprovado com follow-up | `/root/c10_builder` / `/root/c10_critic` — **G3 APROVADO COM FOLLOW-UP**: teste de regressão passou no baseline, sem red histórico; G4 pendente |
| C12 | Fixture CLI por timeframe produz ao menos um trade fechado por sinal; teste exercita binário e `run_sma_crossover`, sem banco. | Interface T-07 aprovada | `/root/c12_builder` / `/root/c12_critic` — **G3 APROVADO**; 8 pares e next-open revisados |
| C13 | Sell aplica slippage configurado no próximo open; equação de custo e regressão de stop/take profit verificadas. | C12 aprovado | `/root/c13_builder` / `/root/c13_critic` — **G3 APROVADO COM FOLLOW-UP**; corrigir link T-07 do README antes de G4 |
| C14 | Pause/Resume responde sem esperar REST/JEV; drena WS, reconcilia REST, descarta resultados obsoletos e confirma estado na TUI, com fontes/relógio falsos. | Interface T-10 aprovada | `/root/c14_builder` / `/root/c14_critic` — **G3 APROVADO COM FOLLOW-UP** após reexame; red histórico incompleto para as fatias adicionais |
| C15 | Produtor WS não bloqueia com canal cheio; overflow e canal fechado têm testes; README descreve perda recuperável. | C14 aprovado com follow-up | `/root/c15_builder` / `/root/c15_critic` — **G3 APROVADO COM FOLLOW-UP**: C17 deve transmitir timestamp de `Full` ao estado de persistência |
| C16 | Opt-in PostgreSQL inválido falha antes do monitor; opt-out não abre banco; `backtest --persist` permanece independente. | Interface T-15 aprovada | `/root/c16_builder` / `/root/c16_critic` — **G3 APROVADO COM FOLLOW-UP**; runtime C17 e PostgreSQL V18 pendentes |
| C17 | Estado de persistência e recuperação REST na TUI; falha/commit incerto, lacuna, pausa e overflow exercitados com armazenamento falso. Adicionar em `live.rs` sinal interno com timestamp descartado em `Full` e consumi-lo no monitor: log/contador de C15 sozinho não atualiza o estado de persistência. | C14, C15 e C16 aprovados com follow-ups | `/root/c17_builder` / `/root/c17_critic` — em andamento |
| Documentação | Índice, arquitetura, integrações, runbook, referência de CLI, catálogo de módulos, matriz de testes e análise de lacunas consolidados no OpenKnowledge. | C13 follow-up documental | Verificado por auditoria de links; 26 documentos, zero links quebrados |
| V18 | Integração PostgreSQL em database `trading_bot` descartável: migração, commit, rollback após erro e idempotência observados; setup, host, resultado e limpeza registrados. | C16 e C17 aprovados; `run-pg-integration-tests.sh` + CI cobrem round-trips G1 | QA/Dados: evidência formal V18 (rollback/limpeza auditada) além dos 5 testes ignorados |

C9/C10, C12/C13, C14/C15 e C16 foram executados com pares independentes e vereditos registrados acima. C17 depende dos estados de pausa, do commit de janelas REST, do descarte WS e do bootstrap de persistência dessas entregas. Alterações simultâneas no README são sequenciadas para evitar sobrescrita. Achado bloqueante ou importante volta ao Builder; após até três ciclos sem acordo, o Orquestrador arbitra sem substituir aprovação obrigatória.

## Verificação e limites

Cada CL registra testes relevantes, `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked`, `git diff --check`, diff revisado e veredito do Critic. G4 reúne regressão de monitor/backtest, revisão de segurança do redirect e revisão de dados da persistência. Não há deploy ou habilitação de ordens neste plano.

O teste PostgreSQL ignorado no `cargo test` padrão e V18 requerem database `trading_bot` **descartável e isolado**. Evidência opcional reproduzível: `./scripts/run-pg-integration-tests.sh` (**22** testes) com `DATABASE_URL` → `trading_bot` (Timescale + pgvector); job CI `postgres-integration` em `.github/workflows/backend-ci.yml` executa o mesmo script. Execução local: [postgres-and-graph-dev](../operations/postgres-and-graph-dev.md). V18 formal (rollback após erro, limpeza auditada) permanece pendente além dos round-trips automatizados.

## Próximas ações do Orquestrador

1. Acompanhar C17 e sua revisão independente; resolver achados de integridade e atualizar README/SDD T-15.
2. Manter o índice, catálogo, matriz de testes e análise de lacunas sincronizados quando os contratos mudarem.
3. Preparar V18 em banco PostgreSQL descartável e isolado quando houver ambiente acessível; registrar setup e execução reais.
4. Consolidar G4 após C17, D19 e V18, sem antecipar lançamento.

## Trilha paralela — completude bots / orders / agents / HTTP

Rastreada em [modules-completeness-audit.md](./modules-completeness-audit.md) (goal ativo, distinto do plano T-05…T-15 acima).

| Fatia | Estado (2026-09-27) | Próximo passo |
|-------|---------------------|---------------|
| HTTP integration (`http_bridge`, `/meta`, execution-status, admin bearer) | Facades [module-catalog §3d](../architecture/module-catalog.md#3d-facade-http_bridge-srcmoduleshttp_bridge); testes `meta_and_*` | Auth owner produto |
| Bots runtime G2 | Parcial: EMA/SMA + catálogo `monitor_evaluator`; `shared_bot_runtime` + testes paridade ([test-matrix § G2](../reference/test-matrix.md#bot-runtime-no-serve-vs-testes-http-g2-parcial)); HTTP promote com capability | Auth owner; orders live no monitor; E2E `BOT_RUNTIME_ENABLED` opcional |
| Orders G2 | Parcial: paper/recording/testnet; reconciliação GET/POST poll + `observe_testnet_spot_order_by_client_id`; idempotência PG `0004`/`0006`; retenção ops em [cli-and-config](../reference/cli-and-config.md#pg-orders-retention-gate-2) | LGTM **Critic** + purge PG automatizado (opcional); prod REST bloqueado |
| Agents G1 | Registry + PG; bootstrap `0010` + `VerifiedProductOwner`; promote capability; checklist [agents G1](../sdd/agents-module-sdd.md#critérios-de-fechamento-g1-checklist) | IdP owner humano; Critic G1 |
| Neo4j F3 read-only | `GraphQueryPort` + CLI `graph query` ([graph-query-port-f3-sdd](../sdd/graph-query-port-f3-sdd.md)); queries `agents`, `supervision-chain`, `bots-for-agent`, `code-impact`; E2E `neo4j_*` (incl. `neo4j_code_impact_for_module_after_seed`) | HTTP read-only gated ([graph-query-port-f3-sdd](../sdd/graph-query-port-f3-sdd.md) § roadmap) |
| Evidência | `./scripts/verify-backend-gates.sh` verde; **470** testes bin `bot`, **0** ignorados; `http_integration` **48**; PG **22/22** via CI `postgres-integration` ou local `./scripts/verify-backend-full.sh` | Revisão Critic AGENTS.md (instância separada) |

Esta trilha não substitui C17/V18; compartilha gates (`verify-backend-gates.sh`) e verificação PG opcional (`verify-backend-full.sh`).
