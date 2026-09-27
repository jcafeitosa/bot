---
title: SDD — Módulo agents (fundação IdentityOnly)
description: Identidade administrativa de agentes, registro em memória, hierarquia e seam consultivo via Jev
tags:
  - sdd
  - backend
  - agents
  - identity
status: draft
---
# SDD — Módulo `modules/agents`

- **Estado:** draft G1 — revisão independente pendente.
- **Referências:** [Pesquisa de capacidades](../research/agents-capability-research.md), [Módulos não implementados](../planning/unimplemented-modules-analysis.md), [Convenção MVC](./modules-mvc-convention-sdd.md), [Proposta 0001](../proposals/0001-backend-core-modules-mvc.md).
- **Premissas:** Auth verificável do owner no transporte permanece bloqueada; **registro HTTP** usa `AgentRegistry` em memória com **espelhamento best-effort** em PostgreSQL (`PgAgentIdentityStore` + `persist_agent_after_mutation`) quando `DATABASE_URL` conecta. Fundação **IdentityOnly** com invariantes `IdentityOnly` e seam de advisory delegando a [`core::providers::jev`](../../src/core/providers/jev/mod.rs). Persistência durável e autenticação do owner no transporte ficam para Gate 1; rotas mutantes podem exigir `BOT_HTTP_ADMIN_TOKEN` e, opcionalmente, `BOT_HTTP_OWNER_ID` no registro e `BOT_HTTP_AGENCY_ID` nas rotas de agentes — seam fail-closed, não bootstrap verificável do owner.

## 1. Contexto e objetivo

O produto bot define uma hierarquia humana e de agentes (owner → CEO → Level B → Level A → especialistas/workers). O módulo `modules/agents` existe em `backend/src` como fundação IdentityOnly em memória. Jev/TypeSafe já opera no monitor como consulta **sem autoridade** via `core::providers`.

**Objetivo desta fatia:** introduzir `modules/agents` com MVC real, tipos de identidade, registro consultável, validação de hierarquia, ciclo de vida administrativo, trilha de eventos em memória e `run_advisory_step` que reutiliza `JevAdvisor` — sem ordens live, sem substituir `risk`/`monitor`, sem worker, ferramentas, memória semântica ou LLM como “cérebro” do agente.

## Relação com bots, backtest e monitor

No domínio do produto bot, **agente** e **bot** são conceitos distintos. No código Rust atual existem `modules/agents` (governança) e **`modules/bots`** (executores strategy×timeframe — fundação em memória + HTTP; ver [SDD bots](./bots-module-sdd.md)). `backtest::BotId` reexporta a mesma chave canônica que `bots::BotId`; não é `AgentId`.

| Dimensão | `modules/agents` | `modules/bots` | `backtest::BotId` | `modules/monitor` |
|----------|------------------|-------------------------|-------------------|-------------------|
| **Existe hoje?** | Sim — fundação `IdentityOnly` em memória | Sim — fundação (sem runtime live/PostgreSQL) | Sim — reexport + simulação em `backtest` | Sim — supervisor de mercado |
| **Propósito** | Identidade e governança administrativa (owner → CEO → … → worker) | Executores versionados de trading, ciclo de promoção/avaliação, artefatos | Identificar uma **instância simulada** (estratégia@versão:timeframe:símbolo) no ranking/backtest | Loop live/paper: candles, sinais, risco, TUI, persistência opcional |
| **Executa ordens / worker?** | Não — registrar agente não inicia task nem LLM | Será o lugar previsto para runtime de executor (após gates) | Não — só simulação offline | Não envia ordens reais hoje; não é cadastro de identidade |
| **Relação com hierarquia do produto** | Fonte de verdade da hierarquia administrativa | Subordinado ao desenho de domínio; **não** substitui `AgentId` | Nenhuma — nome “Bot” é legado de simulação | Pode integrar `MonitorAgentHook` no futuro; hoje noop |
| **Persistência** | Eventos em memória; PostgreSQL após Gate 1 | A definir (versionamento, métricas, promoção) | Métricas/resultados de backtest em memória/JSON da CLI | Estado de sessão, gaps, datasets |

**Mensagem para implementadores:** `AgentRegistry::register` cria uma **identidade administrativa**, não um executor de mercado. `BotId::new(...)` compõe uma **chave de experimento** no backtest. O módulo **`bots`** concentra bots versionados e ranking; runtime live e promoção seguem em gates posteriores ([pesquisa de capacidades](../research/agents-capability-research.md) — linha “Bots executores de tarefa/mercado”); até lá, não criar `modules/bots` por analogia com `agents` nem renomear `BotId` para “agente”.

### Não objetivos

- Autenticação verificável do owner, bootstrap único ou autorização por agência no transporte.
- Leitura durável do registry a partir de PostgreSQL (hoje só write-through nas mutações HTTP).
- Runtime durável, scheduler, gateway MCP, canais externos, execução financeira.
- Alterar comportamento do monitor, risco ou estratégia nesta fatia (apenas documentar hook futuro).
- Expandir `modules/bots` com runtime live ou PostgreSQL nesta fatia (gates próprios).

## 2. Convenção MVC

```text
modules/agents/
  mod.rs
  models/
    mod.rs          # AgentId, AgentDefinition, hierarquia, lifecycle, erros
  controllers/
    mod.rs
    registry.rs     # AgentRegistry (in-memory)
    lifecycle.rs    # transições pausar/retomar/aposentar
    advisory.rs     # run_advisory_step
    supervisor_hook.rs  # NoopMonitorAgentHook — integração futura com supervisor
  adapters/
    mod.rs
    jev.rs          # delegação fina a core::providers::jev
```

| Camada | Responsabilidade |
|--------|------------------|
| **Model** | Identidade, papéis, supervisor tipado, estados, capacidades, eventos de auditoria. |
| **Controller** | Registro, validação de hierarquia, ciclo de vida, elegibilidade de advisory, dispatch. |
| **Adapter** | Chamada HTTP Jev existente; sem política de domínio nova. |

## 3. Seams públicos

| Símbolo | Consumidor previsto | Contrato |
|---------|---------------------|----------|
| `AgentId`, `AgencyId`, `OwnerId` | Admin futuro, testes | Strings normalizadas, não vazias, tamanho máximo 64. |
| `AgentRole`, `SupervisorRef` | Registro | CEO → `SupervisorRef::Owner`; demais → agente supervisor com papel compatível. |
| `AgentDefinition`, `AgentCapabilities` | Registro | `IdentityOnly` por padrão; `consult_jev` explícito para advisory. |
| `AgentRegistry` | Composition root / API futura | `register`, `get`, `list_agency`, eventos append-only em memória. |
| `transition_pause/resume/retire` | Owner futuro | Idempotente onde aplicável; aposentado é terminal. |
| `run_advisory_step` | Monitor (futuro), testes | Falha se agente inexistente, agência errada, inativo/aposentado ou sem `consult_jev`; delega a `JevAdvisor::review`. |
| `MonitorAgentHook` | `modules::monitor::controllers::supervisor` | Stub `NoopMonitorAgentHook`; sem acoplamento nesta fatia. |

**Integração Jev:** único caminho suportado é `crate::core::providers::{JevAdvisor, JevReviewInput}`. Não há `modules::jev` no worktree atual; reexport legado não é necessário.

## 4. Regras de domínio (IdentityOnly)

1. Registrar agente **não** inicia task, worker, fila nem chamada de modelo.
2. Papéis válidos: `Ceo`, `LevelB`, `LevelA`, `Specialist`, `Worker`.
3. Cadeia de supervisão sem ciclos; CEO reporta ao owner; cada nível reporta ao nível imediatamente acima.
4. Agência isola consultas: registro e leitura filtrados por `AgencyId`.
5. Advisory exige `AgentCapabilities.consult_jev == true` e estado `Active`.

## 5. Alternativas

| Alternativa | Decisão |
|-------------|---------|
| Persistir identidades em PostgreSQL já | Adiada — Gate 1 bloqueado; registro em memória com eventos auditáveis. |
| Colocar agents em `core::providers` | Rejeitada — identidade é domínio de produto, não provedor LLM. |
| Advisory sempre no monitor sem agente | Mantido hoje; `run_advisory_step` é seam opcional por identidade. |

## 6. Riscos e validação

| Risco | Mitigação |
|-------|-----------|
| Confundir advisory do monitor com “cérebro” do agente | Capacidade explícita `consult_jev`; doc e testes de negação. |
| Registro em memória perdido no restart | Documentado; migração PostgreSQL em fatia futura. |
| Import proibido `agents` → `presentation` | Script `check-import-direction.sh` + clippy. |

**Rollout:** adicionar `pub mod agents` em `modules/mod.rs`; sem wire no `main` nesta fatia.

**Rollback:** remover módulo e SDD; monitor inalterado.

**Testes G3:**

- Hierarquia válida/inválida e detecção de ciclo.
- Ciclo de vida idempotente e estado terminal aposentado.
- `run_advisory_step` negado para IdentityOnly sem `consult_jev`.
- Delegação a Jev testada via elegibilidade (sem rede obrigatória).

## 7. Referências

- [Catálogo de módulos](../architecture/module-catalog.md)
- [Estado e roadmap](../planning/current-state-and-roadmap.md)


## HTTP e PostgreSQL (parcial)

- Mutations: `persist_agent_after_mutation` espelha em PG.
- Boot: `presentation/http/server.rs` chama `load_agent_identity_snapshot` + `apply_agent_identity_snapshot` quando o registry compartilhado está vazio.
