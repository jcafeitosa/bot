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

> Revisão: 2026-09-27. Estados abaixo distinguem comportamento presente, design aprovado e trabalho ainda bloqueado.

## O que já está feito

### Produto executável

- CLI com caminho de monitor e subcomando de backtest.
- Configuração TOML com overrides de CLI e validação de ambiente, operação, modo, risco e timeframe.
- Monitor terminal com TUI, pausa, retomada e saída.
- Feed híbrido REST/WS para candles; WS limitado a klines fechados de `1m`.
- Validação de janelas REST e deduplicação/ordenação no feed.
- Estratégia SMA com períodos por operação.
- Modos observe/paper; ordem de exchange permanece desabilitada.
- Backtest sintético determinístico com taxas, slippage e resumo JSON.
- Persistência PostgreSQL opt-in, migração automática e gravação idempotente de datasets.
- Logging estruturado para stderr e arquivos rotacionados.
- Jev/TypeSafe consultivo opcional sem autoridade operacional.

### Evidência existente

- Testes de configuração cobrem arquivo padrão, caminho explícito, arquivo ausente e arquivo ilegível.
- Teste de fixture cobre presets/timeframes suportados e fechamento de trade.
- Testes puros de origem cobrem mesma origem, downgrade, userinfo, histórico vazio e limite de redirects.
- Testes HTTP cobrem bloqueio entre origens e aceitação dentro da origem inicial quando loopback está disponível.
- Teste PostgreSQL existe, mas é ignorado por padrão e exige banco dedicado.
- O [mapa de módulos anterior](../sdd/backend-module-map-sdd.md) e a [arquitetura consolidada](../architecture/backend-module-reference.md) registram os módulos presentes.

## Designs e correções já registrados

| Item | Estado atual | Documento |
|---|---|---|
| T-03 — configuração, mercado e organização | Proposta; Gate 1 não comprovado nesta revisão. | [SDD T-03](../sdd/backend-corrections-sdd.md) |
| T-05 — redirects REST | Design e prova HTTP observável aprovados; loopback validado fora do sandbox. | [SDD T-05](../sdd/rest-redirect-sdd.md) |
| T-07 — fixture e slippage | Design e parte da implementação/testes registrados; revisão documental acompanha os gates. | [SDD T-07](../sdd/backtest-trades-and-slippage-sdd.md) |
| T-10 — pausa/retomada | Design técnico aprovado; acordo/sequência de implementação ainda pendentes. | [SDD T-10](../sdd/monitor-pause-resume-sdd.md) |
| T-13 — limpeza de arquivos legados | Design e implementação registrados como aprovados, sujeito à verificação do worktree. | [SDD T-13](../sdd/legacy-file-cleanup-sdd.md) |
| T-15 — persistência opcional | Design técnico aprovado; implementação/testes do fluxo completo ainda pendentes. | [SDD T-15](../sdd/monitor-persistence-policy-sdd.md) |
| T-16 — mapa de módulos | Documentação do README concluída conforme o SDD. | [SDD T-16](../sdd/backend-module-map-sdd.md) |

## Pendências e bloqueios

1. Fechar T-03 com revisão independente e acordo dos seams públicos.
3. Confirmar a implementação de T-10 com testes de pausa, retomada, cancelamento e geração.
4. Completar T-15 com testes de persistência opt-in, falha de escrita, gap e reconciliação.
5. Confirmar T-13 no estado atual do worktree e garantir que não há caminhos legados ativos.
6. Revisar o contrato de Jev, timeout, telemetria e comportamento quando o endpoint falha.
7. Definir observabilidade operacional: métricas de WS/REST, estado de persistência, idade do último candle, falhas de Jev e runbook de credenciais.
8. Ingerir localmente as fontes externas da pesquisa de agentes antes de promovê-la a conhecimento consolidado.

## Plano recomendado

### P0 — Fechar evidência de segurança

- Executar os testes HTTP de redirect em ambiente com loopback.
- Confirmar que o adapter usa a política instalada no `HttpClient`.
- Revalidar checksum, licença e proveniência do vendor.
- Atualizar o [runbook](../operations/runbook.md) com o resultado observável.

### P1 — Fechar contratos do monitor

- Confirmar os seams públicos de T-10 e T-15.
- Implementar testes red para pausa/retomada e persistência.
- Implementar o mínimo green.
- Testar cancelamento, stale results, gaps e degradação sem transformar falha em execução.

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
CARGO_TARGET_DIR=/private/tmp/bot-backend-target cargo test --locked
71 unitários + 1 fixture + 2 config CLI + 3 redirect-origin = passaram
1 teste PostgreSQL = ignorado; requer DATABASE_URL para trading_bot
2 redirect-policy HTTP = falharam antes do teste por PermissionDenied ao abrir listener local
```

A suíte de lógica e contratos passou. A prova HTTP observável de redirect continua pendente porque o sandbox não permite abrir listeners em loopback; isso mantém P0/T-05 bloqueado. A compilação em target temporário produziu warnings existentes de código não usado em `app.rs` e variantes/campos sem uso; eles não foram alterados nesta tarefa documental.

## Gates de aceitação

- **G0:** escopo, usuários e não objetivos registrados.
- **G1:** SDD proporcional aprovado por revisão independente.
- **G2:** plano com dependências e critérios de aceite.
- **G3:** testes red/green e implementação mínima.
- **G4:** verificações executadas com evidência observável.
- **G5:** deploy somente com autorização explícita e rollback operacional.

Nenhum documento desta pasta autoriza deploy. A aceitação final depende dos gates e das evidências listadas nos SDDs.
