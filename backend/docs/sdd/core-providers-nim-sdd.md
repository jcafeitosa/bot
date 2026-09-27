---
title: SDD — core::providers NVIDIA NIM
description: Cliente OpenAI-compatible para NVIDIA integrate API, categorias de modelo e seam NimLlmProvider
tags:
  - sdd
  - backend
  - core
  - nim
  - nvidia
status: draft
---

# SDD — `core::providers::nvidia_nim`

## Contexto

O bot já expõe `openai_compatible`, `nine_router` e `jev` em `core::providers`. Agentes futuros podem precisar de LLMs e modelos especiais hospedados na **NVIDIA NIM** (integrate API). O catálogo de modelos é **dinâmico** na NVIDIA; a referência oficial por categoria está em [NIM models reference](https://docs.api.nvidia.com/nim/reference/models-1) (LLM, retrieval, visual, multimodal, healthcare, route optimization, climate).

## Objetivo

1. Adicionar `src/core/providers/nvidia_nim.rs` reutilizando `OpenAiCompatibleClient`.
2. Configurar base URL e chave via env (primário) e TOML opcional (`providers.nim_base_url`).
3. Expor tipos estáveis (`NimModelCategory`, `NimModelRef`, `NvidiaNimClient`) e `list_models_hint()` com exemplos curados — sem lista enorme hardcoded.
4. Trait opcional `NimLlmProvider` para wiring futuro de agent brains; **Jev advisory permanece inalterado**.

## Não-objetivos

- Substituir o fluxo TypeSafe SystemOne do Jev por NIM.
- Implementar listagem remota de modelos (GET catalog) nesta fatia.
- Acoplar HTTP bridge ou agents a NIM além do seam exportado.

## Seams públicos

| Seam | Caminho | Uso |
|------|---------|-----|
| `NvidiaNimClient` | `core::providers::nvidia_nim` | `chat_completions(model, messages)` via `/v1/chat/completions`. |
| `NimLlmProvider` | idem | Trait async para injeção em agentes. |
| `NimModelCategory` / `NimModelRef` | idem | Seleção tipada alinhada à doc NVIDIA. |
| `list_models_hint` | idem | Defaults documentais; IDs reais vêm da API/catálogo NVIDIA. |
| `ProviderConfig::nim_base_url` | `core::config` | Override TOML; env `NVIDIA_NIM_BASE_URL` precede. |

## Env vars

| Variável | Papel |
|----------|--------|
| `NVIDIA_API_KEY` | Bearer para integrate API (preferencial). |
| `NGC_API_KEY` | Fallback de bearer quando `NVIDIA_API_KEY` ausente. |
| `NVIDIA_NIM_BASE_URL` | Raiz da API (aceita `https://integrate.api.nvidia.com` ou com sufixo `/v1`; normalizado para evitar `/v1/v1/...`). |

Default de base: `https://integrate.api.nvidia.com`.

## Exemplos de `model_id` (configuráveis)

Strings típicas para chat LLM (não fixadas em runtime):

- `meta/llama-3.1-70b-instruct`
- `meta/llama-3.1-8b-instruct`
- `nvidia/nemotron-mini-4b-instruct`

Outras categorias seguem o prefixo `org/model` da documentação NVIDIA.

## Riscos e rollback

- **Risco:** URL com `/v1` duplicado. **Mitigação:** `normalize_nim_base_url`.
- **Risco:** HTTP inseguro em produção. **Mitigação:** mesma regra `validate_https_or_localhost` que Jev/openai_compatible.
- **Rollback:** remover módulo e campo TOML; reexports em `mod.rs`.

## Validação

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --locked`, `./scripts/check-import-direction.sh`.
- Testes httpmock em `nvidia_nim`: POST chat + rejeição de HTTP não-local.

## Relacionado

- [core-providers-jev-sdd.md](./core-providers-jev-sdd.md)
