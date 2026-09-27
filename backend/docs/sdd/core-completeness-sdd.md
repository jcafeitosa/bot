---
title: SDD — Completude transversal do core
description: Escopo de infra compartilhada em src/core (health, notifications) vs modules/
tags:
  - sdd
  - backend
  - architecture
  - core
status: draft
---

# SDD — Completude transversal do `core`

- **Estado:** draft (implementação mínima completa).
- **Referências:** [Proposta 0001](../proposals/0001-backend-core-modules-mvc.md), [Convenção MVC](./modules-mvc-convention-sdd.md), [Extração core F1](./core-extraction-phase1-sdd.md), [Política de persistência do monitor](./monitor-persistence-policy-sdd.md).

## Contexto

O `core` atual expõe `config`, `error`, `logging`, `persistence` e `providers`. A proposta 0001 e a convenção MVC definem `core` como **infraestrutura transversal sem política de domínio**. Health operacional de arquivo de mercado (`OFF` / `HEALTHY` / `DEGRADED` / `GAP`) permanece em `modules::monitor` (`PersistenceHealth`). Faltavam primitivas reutilizáveis para **liveness/readiness de processo** e **notificação estruturada** de eventos críticos.

## Objetivo

1. `core::health` — agregar probes de readiness (processo + PostgreSQL opcional).
2. `core::notifications` — trait `Notifier`, `LogNotifier` (tracing) e canal stub para integrações futuras.
3. Wire mínimo no monitor ao mudar estado de arquivo para `GAP` ou degradação relevante.
4. HTTP `/readyz` reutiliza `core::health` em vez de SQL inline.

## Escopo — o que entra no `core`

| Módulo | Entra | Responsabilidade |
|--------|-------|------------------|
| `health` | Sim | Liveness, readiness struct, probe de banco via `Database::ping`. |
| `notifications` | Sim | Contrato de envio, impl log, stub de canal externo. |
| `metrics` / `tracing` além de `logging` | Não (agora) | Roadmap de observabilidade completa fica em planning; sem Prometheus nesta fatia. |
| `time` | Não | `std::time` / `tokio::time` suficientes; sem relógio injetável global. |
| Política `PersistenceHealth` | Não | Permanece `modules::monitor`; core não importa `modules`. |

## Não-objetivos

- SLI/SLO, alertas, dashboards ou runbooks (cap. 8 do planning).
- Slack/email reais; apenas stub + logs.
- Alterar semântica de trading, persistência de mercado ou TUI.

## Seams públicos

| Seam | Tipo | Uso |
|------|------|-----|
| `health::liveness` | fn | Sempre `Up` (processo vivo). |
| `health::readiness` | async fn | `ReadinessReport` com componentes `process` e opcional `database`. |
| `persistence::Database::ping` | async fn | `SELECT 1` reutilizável por health e HTTP. |
| `notifications::Notifier` | trait | `notify(&Notification)`. |
| `notifications::LogNotifier` | struct | Emite `tracing` por severidade. |
| `notifications::StubChannelNotifier` | struct | Delega ao log e registra canal não configurado. |

## Matriz de dependência

```text
presentation/http → core::health, core::persistence (readyz)
modules/monitor/supervisor → core::notifications (eventos críticos de arquivo)
core::health → core::persistence (opcional)
core → nunca modules::* ou presentation::*
```

## Validação

```sh
cd backend
cargo fmt
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
./scripts/check-import-direction.sh
```

## Rollback

Remover `core/health`, `core/notifications`, revert de wire no supervisor e restaurar `readyz` inline.
