---
title: SDD W0-06 — hook de agentes no monitor documentado como ligado e só log
description: Fatia Onda 0 que corrige doc, comentários e anotações do MonitorAgentHook para o estado real (ligado, log-only, só com BOT_AGENCY), sem mudar comportamento
tags:
  - sdd
  - backend
  - agents
  - monitor
  - wave0
status: draft
---

# SDD W0-06 — hook de agentes: ligado e só log

- **Estado:** draft. Nenhum gate aprovado. Precisa de Critic independente (G1).
- **Plano:** W0-06 em [master-plan](../planning/master-plan.md) §4.1. Esta fatia **não** remove o hook e **não** dá efeito a ele; dar efeito é W2-05.
- **SDD do módulo:** [agents-module-sdd](./agents-module-sdd.md).

## Contexto (evidência no código, HEAD `d42b71a5`; entre `72eb471d` e `d42b71a5` nenhum arquivo citado aqui mudou em `backend/src`)

- Ligação: `main.rs:118` (`serve --with-monitor`) e `main.rs:165` (TUI) chamam `monitor_agent_hook_from_env()` (`modules/agents/controllers/runtime.rs:18-40`). Com `BOT_AGENCY` válido devolve `RegistryMonitorAgentHook`; sem ele, `NoopMonitorAgentHook`.
- Uso: `modules/monitor/controllers/supervisor.rs:794-795` chama `evaluation_agents()` e `on_evaluation_cycle(...)` a cada barra avaliada.
- Efeito: `RegistryMonitorAgentHook::on_evaluation_cycle` (`modules/agents/controllers/supervisor_hook.rs:54-65`) só emite `tracing::trace!` com a contagem de agentes `consult_jev`. Não age sobre sinal, risco nem ordem.
- Detalhes: o parâmetro recebido (`_agent_ids`) é ignorado e `consultant_ids()` é calculado duas vezes por ciclo (uma em `evaluation_agents`, outra dentro de `on_evaluation_cycle`).
- Na TUI, o registry é o do processo, não hidratado do PG (hidratação é W2-04/W2-05).

## Contradições doc × código

| Onde | Diz | Real |
|---|---|---|
| `supervisor_hook.rs:1` | `#![allow(dead_code)] // Registry hook seam; monitor wiring pending.` | Ligado desde `main.rs:118,165` |
| `supervisor_hook.rs:25` | `#[allow(dead_code)] // Wired when monitor passes MonitorAgentHook…` | Já é passado |
| [agents-module-sdd](./agents-module-sdd.md) linha 57 (árvore) | `NoopMonitorAgentHook — integração futura com supervisor` | Integrado; há também `RegistryMonitorAgentHook` |
| [agents-module-sdd](./agents-module-sdd.md) linha 79 (seams) | `Stub NoopMonitorAgentHook; sem acoplamento nesta fatia.` | Acoplado via `run_with_agent_hook` |

Já corretos (sem mudança): [agents-module-sdd](./agents-module-sdd.md) linha 32, [module-catalog](../architecture/module-catalog.md) linhas 88 e 96, [unimplemented-modules-analysis](../planning/unimplemented-modules-analysis.md) linha 70.

Observação: `agents-module-sdd.md` tem mudanças não commitadas de outra sessão no momento desta redação. Por isso este SDD **aponta** as duas linhas a corrigir e não edita o arquivo; o Builder corrige quando o arquivo estiver limpo.

## Decisão

1. Remover `#![allow(dead_code)]` da linha 1 e o `#[allow(dead_code)]` da linha 25 de `supervisor_hook.rs`, com os comentários falsos. `cargo clippy -- -D warnings` sem os allows prova que o código é usado.
2. Doc-comment de `RegistryMonitorAgentHook` passa a dizer: "ligado ao monitor, só log `trace`, ativo só com `BOT_AGENCY`".
3. Corrigir as linhas 57 e 79 do [agents-module-sdd](./agents-module-sdd.md) com o mesmo texto.
4. Nenhum comportamento muda. A duplicação de `consultant_ids()` e o parâmetro ignorado ficam como estão (mudar a chamada seria mudança de comportamento, mesmo pequena); registrar como nota para W2-05.

**Alternativa considerada:** renomear o tipo para algo como `LoggingMonitorAgentHook`, para o nome dizer que só observa. É interno (sem contrato público), mas mexe em reexports (`modules/agents/mod.rs`) e em testes, e o nome vai ficar errado de novo quando W2-05 der efeito ao hook. Recomendo só o doc-comment; a renomeação fica a critério do owner.

## Seams públicos

Nenhum. `MonitorAgentHook` não muda.

## Critérios de aceite

- A1. `supervisor_hook.rs` sem `allow(dead_code)`; clippy com `-D warnings` verde.
- A2. Nenhum doc em `backend/docs` diz que o hook é "futuro", "stub" ou "sem acoplamento": `rg -n "integração futura|sem acoplamento" backend/docs -g '!**/sdd/wave0-*'` sem hit sobre o hook (os SDDs `wave0-*` citam as frases antigas como evidência e ficam fora da busca).
- A3. Testes existentes do hook e do monitor passam sem alteração.

## Dependências

- Nenhuma para começar. Habilita W2-05.

## Riscos

- Remover o allow pode revelar outro item realmente morto no arquivo (por exemplo, um helper só de teste); nesse caso, mover para `#[cfg(test)]`, não reintroduzir o allow.

## Validação

- `backend/scripts/verify-backend-gates.sh` (fmt, clippy `-D warnings`, testes).

## Rollout / rollback

- Rollout: sem efeito em runtime.
- Rollback: reverter o commit.
