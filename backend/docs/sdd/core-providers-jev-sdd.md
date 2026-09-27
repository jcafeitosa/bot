---
title: SDD — core::providers e Jev
description: Cliente OpenAI-compatible, nine_router env, Jev como submódulo de core/providers
tags:
  - sdd
  - backend
  - core
  - jev
status: draft
---

# SDD — `core::providers` e migração do Jev

## Contexto

Consultas advisory Jev/TypeSafe hoje vivem em `modules/jev` com HTTP ad hoc em `adapters/typesafe`. A proposta agrupa integrações LLM OpenAI-compatible em `core::providers`, move Jev para `core::providers::jev` e preserva o gate consultivo no monitor (sem autoridade de ordem).

Não há documentação de **9router** no repositório; o alvo de deploy é configurável via env genérica OpenAI-compatible.

## Objetivo

1. Introduzir `src/core/providers/` (`openai_compatible`, `nine_router`, `jev/*`).
2. Migrar comportamento de `modules/jev` sem alterar payload advisory nem regras HTTPS/localhost.
3. Respeitar direção de imports: `core` não importa `modules`; entrada de review usa `JevReviewInput` no monitor.

## Não-objetivos

- Trocar o contrato HTTP TypeSafe SystemOne por chat/completions no fluxo Jev default.
- Documentar ou acoplar API proprietária 9router além de env vars.
- Mover `StrategySnapshot` ou `Signal` para `core`.

## Seams públicos

| Seam | Caminho | Uso |
|------|---------|-----|
| `OpenAiCompatibleClient` | `core::providers::openai_compatible` | HTTP JSON + bearer; paths relativos ou URL absoluta. |
| `nine_router` | `core::providers::nine_router` | Resolução de `NINE_ROUTER_BASE_URL` / `OPENAI_BASE_URL`. |
| `JevAdvisor` | `core::providers::jev` (reexport em `core::providers`) | `from_env(JevConfig)`, `review(JevReviewInput)`. |
| `ProviderConfig` | `core::config` | Campo opcional TOML; credenciais/endpoints primários via env. |

## Env vars

| Variável | Papel |
|----------|--------|
| `TYPESAFE_API_KEY` | Obrigatória com `jev.enabled=true` (fallback `OPENAI_API_KEY` para proxies compatíveis). |
| `TYPESAFE_ENDPOINT` | URL completa do advisory (default TypeSafe SystemOne). |
| `OPENAI_API_KEY` | Fallback de bearer quando `TYPESAFE_API_KEY` ausente. |
| `OPENAI_BASE_URL` | Base OpenAI-compatible (`/v1/chat/completions` via cliente). |
| `NINE_ROUTER_BASE_URL` | Alias documentado para 9router; precede `OPENAI_BASE_URL` na resolução de base. |

NVIDIA NIM (provider adicional, não usado pelo Jev default): ver [core-providers-nim-sdd.md](./core-providers-nim-sdd.md).
