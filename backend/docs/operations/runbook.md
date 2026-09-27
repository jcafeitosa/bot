---
title: Runbook operacional do backend
description: Procedimentos de inicialização, operação, diagnóstico e recuperação do backend
tags:
  - operations
  - backend
  - runbook
---
# Runbook operacional do backend

## Escopo

O backend é um processo de terminal para leitura de dados públicos da Binance Spot Test Network, avaliação de estratégia e modo paper/observe. A criação de sinal não envia ordem. O fluxo de pausa e retomada está especificado em [SDD T-10](../sdd/monitor-pause-resume-sdd.md), e a persistência opcional em [SDD T-15](../sdd/monitor-persistence-policy-sdd.md).

## Pré-voo

1. Execute a partir de `backend/`.
2. Confirme que o arquivo de configuração está legível.
3. Use apenas credenciais por variáveis de ambiente; nunca registre ou comite segredos.
4. Para persistência, confirme `DATABASE_URL` apontando para o banco `trading_bot` e verifique as migrações ativas antes de iniciar.
5. Comece em `dev` com `observe` ou `paper`. O modo HFT é rejeitado pelo backend atual.

## Inicialização

Monitor:

```sh
cargo run -- --config src/core/config/bot.toml --environment dev --mode observe --operation day-trader --risk-profile conservative
```

Backtest:

```sh
cargo run -- backtest --config src/core/config/bot.toml
```

Persistência é opt-in. Para o backtest, use `--persist`. Para o monitor, defina `PERSIST_MARKET_DATA=1` e forneça `DATABASE_URL`; mantenha o timeframe em `1m`.

## Operação durante a execução

- Pressione Espaço para pausar ou retomar a avaliação.
- Pressione `q` ou Esc para encerrar.
- Observe a TUI, stderr e os logs JSON rotacionados.
- Em `1m`, o monitor combina WS e REST; perda do WS deve manter o caminho REST ativo.
- Uma pausa confirmada impede novas avaliações; a retomada exige backfill REST válido antes de avaliar.

As regras de redirects e validação da janela REST estão detalhadas no [SDD de redirects REST](../sdd/rest-redirect-sdd.md) e no [SDD de correções do backend](../sdd/backend-corrections-sdd.md).

## Diagnóstico rápido

| Sintoma | Ação |
|---|---|
| Configuração não encontrada ou inválida | Confirme o caminho de `--config`, TOML e combinações de modo/timeframe na [referência de CLI e configuração](../reference/cli-and-config.md). |
| HFT rejeitado | Use um modo suportado; o backend atual é baseado em polling REST e não oferece infraestrutura HFT. |
| WS indisponível | Verifique logs e conectividade; mantenha a execução se REST estiver ativo. |
| Persistência indisponível | Trate o aviso como degradação, valide `DATABASE_URL` e não interprete o arquivo como histórico completo. |
| Janela REST rejeitada | Corrija a origem dos dados ou aguarde nova janela válida; não persista a janela rejeitada. |
| Redirect externo rejeitado | Preserve o erro e investigue a origem configurada; não relaxe a política sem revisar o [SDD T-05](../sdd/rest-redirect-sdd.md). |

## Encerramento e recuperação

1. Encerre com `q` ou Esc.
2. Preserve stderr e os logs JSON da execução.
3. Se houver falha de persistência, mantenha o monitor em observe/paper e desative o opt-in até a causa ser corrigida.
4. Não habilite produção ou ordens como tentativa de recuperação.
5. Registre a ocorrência no plano de execução quando ela alterar um gate ou bloqueio.

## Verificação antes de aceitar uma alteração

Gate canônico (mesmo job `rust` da CI):

```sh
./scripts/verify-backend-gates.sh
```

Inclui `fmt`, `clippy --bin bot`, import-direction, testes do bin `bot` com `--test-threads=1` e cinco suítes em `tests/`. Com PostgreSQL descartável (`DATABASE_URL` → `trading_bot`):

```sh
./scripts/verify-backend-full.sh
```

Baseline esperada (2026-09-27): linha `OK:` do gate → **461** passed, **0** ignored no bin `bot`; com PG → `run-pg-integration-tests.sh` **21/21**. Baseline e detalhes: [auditoria de completude](../planning/modules-completeness-audit.md#verificação-local). O [plano de execução](../planning/backend-work-plan.md) registra gates T-03…T-15 e a trilha paralela de completude de módulos.
