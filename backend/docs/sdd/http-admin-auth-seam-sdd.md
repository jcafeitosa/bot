---
title: SDD — HTTP admin bearer seam (opcional; sem token = sem auth)
description: BOT_HTTP_ADMIN_TOKEN, BOT_HTTP_OWNER_ID e BOT_HTTP_AGENCY_ID; não substitui Gate 1 owner auth
tags:
  - sdd
  - backend
  - security
  - http
  - wave0
status: partial
---

# SDD — HTTP admin bearer seam

## W0-01 — fail-closed de verdade (Onda 0, proposta para G1) [SEGURANÇA]

- **Estado desta seção:** draft para G1. Nenhum gate aprovado. Precisa de Critic independente e de acordo do Julio sobre os seams antes do primeiro teste (AGENTS.md, G1/G3).
- **Plano:** W0-01, prioridade 1, em [master-plan](../planning/master-plan.md) §4.1.
- **Fonte dos critérios de segurança:** [F-ADM-01](../security/admin-http-auth-fail-open.md) §4 (SEC-ADM-01…14).
- **Leitura importante:** as seções abaixo desta ("Gate 1", "Comportamento") descrevem o código **atual**, que é fail-open sem token. Esta seção descreve o alvo. Quando W0-01 for implementado, a tabela "Comportamento" e o título mudam junto (SEC-ADM-13).

### Contexto (evidência no código, HEAD `b2001a7c`)

- `backend/src/presentation/http/admin_auth.rs:91-94`: `verify_headers` devolve `Ok(())` quando não há token. Sem `BOT_HTTP_ADMIN_TOKEN`, toda rota "protegida" fica aberta, inclusive CRUD de `provider_credentials`, `orders/submit`, `bots/runtime/promote` e `monitor/commands`.
- `backend/src/core/config/http/file.rs:10-17`: token só vem de env, sem validação de força. `backend/src/core/config/system.toml` `[http.admin]` vazio; `backend/.env.example:68` traz a chave vazia.
- `admin_auth.rs:143-152`: `constant_time_eq` sai cedo quando os tamanhos diferem (vaza tamanho). `admin_auth.rs:7` deriva `Debug` com o token.
- `backend/src/presentation/http/routes/mod.rs:25-93`: rotas montadas uma a uma, sem layer de auth; cada handler chama `state.require_http_admin` à mão (`state.rs:800`). Rota nova sem a chamada fica aberta.
- `backend/src/presentation/http/server.rs:16-73`: `serve` sobe sem auth e só registra `http_admin_auth_enabled` em log. `cli.rs:9-10`: `--bind` padrão loopback, aceita qualquer endereço.
- Contradições ainda abertas: (`cli-and-config.md:138` e o título deste SDD já descrevem o comportamento como opcional/aberto sem token); `admin_auth.rs:1` diz "Optional fail-closed"; `HttpApiSeams::disabled_fail_closed()` (`state.rs:47`) usa auth admin **desligada** (aberta).

### A1 — escopo obrigatório e follow-ups

| Classe | IDs de [F-ADM-01](../security/admin-http-auth-fail-open.md) | Tratamento |
|---|---|---|
| Obrigatório em W0-01 | SEC-ADM-01, 02, 03, 04, 06, 07, 08, 09, 13 | critérios de aceite desta fatia |
| Follow-up explícito | SEC-ADM-05 (leituras sensíveis), SEC-ADM-10 (Host allowlist), SEC-ADM-11 (limite de tentativas), SEC-ADM-12 (rotação com dois tokens) | cada um vira item próprio no plano; não bloqueia G4 de W0-01 |
| Bloqueado | SEC-ADM-14 | depende de P1 (auth humano/IdP); registrado, não implementado |

**Decisão explícita (boot × 401/503):** fora de dev+loopback, configuração de auth ausente ou inválida faz o boot falhar (exit ≠ 0, socket não aberto). Não existe modo "sobe e responde 401/503 em tudo". Com o servidor no ar, nenhuma requisição chega a handler de rota `Protected` sem passar pela layer de auth (sem credencial válida → 401 na layer). Único modo sem auth: opt-out dev (A3), só em loopback e perfil não produtivo.

Resumo da decisão: `bot serve` não abre socket sem auth válida; opt-out só dev + loopback + perfil não produtivo; auth por layer; digest em tempo constante; token forte; `Debug` redigido; header estrito; docs alinhadas.

**Alternativa considerada (A1):** subir o `serve` sem token e responder **503** `admin_auth_not_configured` em toda rota protegida. Menos quebra para quem só lê, mas adia o erro para a primeira requisição e contradiz SEC-ADM-01. Rejeitada.

### A2 — fonte única das rotas (router e teste)

Evidência: o `Router` do axum não oferece listagem das rotas registradas; o router é montado à mão em `routes/mod.rs`; o OpenAPI vem de anotações `#[utoipa::path]` agregadas em `ApiDoc` (`openapi.rs`). O axum não expõe a lista de rotas de um `Router`; então um teste guiado só pelo OpenAPI não vê rota registrada no router e esquecida na anotação.

- **Decisão:** uma tabela declarativa única em `presentation/http/routes` — cada entrada tem método, path, handler e classe de acesso (`Protected` | `PublicCompute` | `PublicRead`). `v1_routes()` monta dois sub-routers a partir dela (o `Protected` recebe a layer de auth via `route_layer`) e o teste de cobertura (SEC-ADM-03) itera a mesma tabela: sem header → 401, token errado → 401, para toda entrada `Protected`. Uma rota fora da tabela não existe no router. O teste também confere que método+path da tabela batem com `ApiDoc::openapi()` (paridade de docs).
- **Alternativa:** OpenAPI como fonte — o teste itera `ApiDoc::openapi().paths` e envia requisições sem auth. Mais simples de escrever, mas não pega rota sem anotação; aceitável só se somado a uma verificação de paridade router × OpenAPI, que o axum não oferece de forma direta.
- Allowlist inicial `PublicCompute` (revisada no próprio teste, com prova de ausência de efeito externo): `/risk/profile-limits`, `/risk/validate-intent`, `/risk/gate-signal`, `/strategy/evaluate-sma`, `/bots/ranking`, `/backtest/sma-crossover`. Leituras continuam `PublicRead` até SEC-ADM-05.

### A3 — opt-out lido por `core::config`

- A flag de opt-out (nome proposto `BOT_HTTP_ADMIN_AUTH_DISABLED_DEV`, aceita só `1`/`true`) é lida em `backend/src/core/config/http/file.rs`, no mesmo `HttpAdminAuthConfig`, junto do token. Nenhuma leitura de env em `presentation/`. O guard de `verify-backend-gates.sh` (reprova `env::var` fora de `*config.rs`/`core/config/`) cobre isso.
- A decisão "pode subir?" é função pura testável: entrada = (token?, flag dev?, bind, `Environment`, `production.enabled`); saída = `Enforced` | `DevDisabled` | erro estável (`admin_auth_not_configured`, `admin_auth_token_weak`, `admin_auth_dev_optout_requires_loopback`, `admin_auth_dev_optout_forbidden_in_prod`). É chamada antes de `TcpListener::bind` em `server.rs`.
- Com `DevDisabled`: log `WARN` no boot e `GET /meta` → `http_seams.http_admin_auth_mode = "dev_disabled"` (senão `"enforced"`).

### A4 — token de dev sem enfraquecer prod

- O desenvolvedor gera o token localmente, fora do binário: `openssl rand -base64 48` (≥ 32 bytes de entropia), e grava em `backend/.env` (ignorado pelo git em `backend/.gitignore:2`). O procedimento vai para [cli-and-config](../reference/cli-and-config.md) e para o runbook de dev; `.env.example` continua com valor vazio (AGENTS.md proíbe placeholder de credencial). Copiar o exemplo sem gerar token faz o `serve` recusar subir, com mensagem que aponta o comando.
- A mesma regra de força (SEC-ADM-07) vale em dev e prod; não há "token fraco permitido em dev". Quem não quer token usa o opt-out, que só existe em loopback e nunca em prod.
- Testes usam token de fixture forte gerado no próprio teste; `HttpAdminAuth::disabled()` passa a existir só em `#[cfg(test)]` (hoje há 39 usos em testes: `state.rs`, `http_integration_tests.rs`, `register_owner.rs`, `admin_auth.rs`).
- **Alternativa:** o `serve` gerar um token efêmero no boot em dev+loopback e imprimi-lo uma vez. Rejeitada: imprime segredo em terminal/log (conflita com SEC-ADM-08) e cria caminho de código que gera credencial.

### Seams públicos para acordo antes do TDD

| Seam | Proposta |
|---|---|
| Flag de opt-out | `BOT_HTTP_ADMIN_AUTH_DISABLED_DEV` (`1`/`true`), lida em `core/config/http/file.rs` |
| Erros de boot | `admin_auth_not_configured`, `admin_auth_token_weak`, `admin_auth_dev_optout_requires_loopback`, `admin_auth_dev_optout_forbidden_in_prod`; exit ≠ 0; sem ecoar valores |
| Tabela de rotas | tipo com método, path, handler, classe de acesso; única fonte de `v1_routes()` e do teste |
| `GET /meta` | `http_seams.http_admin_auth_mode`: `enforced` \| `dev_disabled` |
| Comparação | digest SHA-256 dos dois lados + comparação em tempo constante (crates `sha2`/`subtle` já no `Cargo.lock` como transitivas; torná-las diretas precisa de acordo) |

### Critérios de aceite W0-01

- SEC-ADM-01, 02, 03, 04, 06, 07, 08, 09, 13, conforme [F-ADM-01](../security/admin-http-auth-fail-open.md) §4, referenciados por ID.
- F1. Teste CLI em `backend/tests/` (padrão de `config_cli.rs`): `serve` sem token e sem flag → exit ≠ 0, porta não aberta; entra na lista `INTEGRATION_TESTS` de `verify-backend-gates.sh`.
- F2. Função de decisão de boot (A3) com teste para cada combinação de token/flag/bind/ambiente.
- F3. Teste de cobertura guiado pela tabela (A2) falha se uma entrada `Protected` responder algo diferente de 401 sem credencial.
- F4. Suite HTTP existente verde com token de fixture.

### Dependências, riscos, validação, rollout

- **Dependências:** nenhuma fatia W0 bloqueia. Habilita W0-07 (SEC-CRED-07 depende de SEC-ADM-01) e reduz o abuso de W0-12. G4 precisa de CI verde (W0-02).
- **Riscos:** quebra de uso local sem token (mitigado por A4 e mensagem clara); refatorar `routes/mod.rs` para a tabela mexe em todas as rotas (mitigado por F3 e OpenAPI smoke existente); conflito de merge com outras fatias que tocam `state.rs` (W0-12/W0-13) — sequenciar ou coordenar.
- **Validação:** `backend/scripts/verify-backend-gates.sh`; Critic de segurança confere SEC-ADM-06 (sem retorno antecipado) e SEC-ADM-08 (captura de logs).
- **Rollout:** próximo build local; quem roda `serve` gera token (A4) ou usa opt-out em loopback. Nenhum deploy autorizado.
- **Rollback:** reverter o commit restaura o comportamento aberto (reabre F-ADM-01); sem migração nem dado a desfazer.

## Gate 1 — fatia HTTP (owner binding verificável no seam)

**Escopo desta fatia:** bearer **opcional** em rotas mutantes (exigido só com `BOT_HTTP_ADMIN_TOKEN` definido; sem token não há autenticação — **lacuna de segurança ABERTA** (não corrigida), ver [admin-http-auth-fail-open](../security/admin-http-auth-fail-open.md)); `BOT_HTTP_OWNER_ID` efetivo somente com `BOT_HTTP_ADMIN_TOKEN` (`HttpAdminAuth::from_env`); **403** `owner_mismatch` no registro de agentes; **401** sem bearer; observabilidade em `GET /api/v1/meta` → `http_seams`. **Não** cobre IdP do owner humano; bootstrap PG explícito (ACK) é documentado em [owner bootstrap G1](./agents-owner-bootstrap-g1-sdd.md) e complementa `BOT_HTTP_OWNER_ID` (checklist [agents G1](./agents-module-sdd.md#critérios-de-fechamento-g1-checklist) item “Autenticação verificável do owner humano” permanece **Não**).

## Contexto

Rotas HTTP mutantes (agents lifecycle, bots catalog persist, bots runtime promote/demote, orders submit, `orders_reconciliation_poll` / `orders_reconciliation_poll_succeeds_with_admin_bearer_when_enabled`, monitor commands) precisam de um controle mínimo em ambientes expostos, sem implementar bootstrap verificável do owner (Gate 1 bloqueado na pesquisa).

## Comportamento

| Variável | Efeito |
|----------|--------|
| `BOT_HTTP_ADMIN_TOKEN` ausente/vazio | Sem exigência de bearer: `verify_headers` retorna OK (`admin_auth.rs:91-94`) e **todas** as rotas listadas (incl. CRUD `provider_credentials`, `orders/submit`, `GET /admin/graph/*`) aceitam requests sem auth; o boot não recusa e `--bind` aceita endereço não-loopback. Padrão distribuído (`system.toml` `[http.admin]` vazio). **Lacuna aberta, não corrigida.** |
| `BOT_HTTP_ADMIN_TOKEN` definido | Rotas mutantes listadas exigem `Authorization: Bearer <token>`; falha → **401**. |
| `BOT_HTTP_OWNER_ID` definido (com token) | `POST /api/v1/agents` exige `owner_id` igual; falha → **403** `owner_mismatch`. Sem `BOT_HTTP_ADMIN_TOKEN`, o bind de owner é ignorado no boot (`admin_auth.rs:25-31`). |
| `BOT_HTTP_AGENCY_ID` definido | Rotas `/api/v1/agents*` exigem `agency` igual (query ou body); falha → **403** `http_agency_mismatch`. Com bind ativo, `POST /api/v1/bots/runtime/promote` também exige que `promoted_by` seja agente ativo da agência com capability `promote_runtime_bot` (`assert_runtime_promotion_authorized`). |

Rotas `/api/v1/admin/provider-credentials*` exigem o mesmo bearer quando o token está ativo; CRUD mascarado com PG (**503** sem banco) — ver [provider-credentials-db-sdd](./provider-credentials-db-sdd.md).

Implementação: `presentation/http/admin_auth.rs`, `ApiState::require_http_admin`, `require_register_owner_id`, `require_bound_agency` (rotas `routes/agents.rs`); promoção de bot em `ApiState::promote_bot_http`; CRUD em `routes/provider_credentials_admin.rs` + `http_bridge/provider_credentials.rs`.

## Fora de escopo

- Prova de identidade do owner humano, bootstrap único (binding de agência por env é seam, não prova de tenant). Itens **Não** do checklist [agents G1](./agents-module-sdd.md#critérios-de-fechamento-g1-checklist) permanecem bloqueadores do goal de completude ([auditoria](../planning/modules-completeness-audit.md)).
- Proteção de rotas de simulação (`risk/*`, `backtest/*`) — permanecem abertas quando admin token ativo.

## Observabilidade (read-only)

`GET /api/v1/meta` inclui `http_seams` read-only: `http_admin_auth_enabled` (token admin ativo), `http_owner_binding_active` / `http_agency_binding_active` / `product_owner_bootstrap_active` (booleanos — não expõem IDs; espelham `BOT_HTTP_OWNER_ID` / `BOT_HTTP_AGENCY_ID`), `order_execution_mode` / `live_exchange_wired` (alinhados com `GET /orders/execution-status`), `bot_runtime_enabled` (alinhado com `GET /bots/runtime/status` → `runtime_enabled`). Não substitui auditoria de rotas mutantes. Bootstrap PG verificável: [owner bootstrap G1](./agents-owner-bootstrap-g1-sdd.md) (`VerifiedProductOwner` + `register_owner` / promote).

## Validação

- Testes unitários `admin_auth.rs` (`binding_active_flags_reflect_env_bindings_without_leaking_ids`).
- Testes HTTP `server.rs`: `meta_*`, `meta_and_*`, `router_after_build_api_state_meta_agrees_with_http_seam_endpoints` (boot `build_api_state_for_http_serve`: `http_seams` ↔ runtime + orders execution-status), agency/owner mismatch, bots runtime capability, monitor commands, OpenAPI smoke.
- Testes HTTP `http_integration_tests.rs`: bearer obrigatório (agents register/pause, bots catalog persist, bots runtime promote, orders submit paper/admin, `orders_reconciliation_poll`); owner bind positivo/negativo (`agents_register_accepts_matching_owner_when_bound`, `agents_register_rejects_owner_mismatch_when_bound`); ver [test-matrix](../reference/test-matrix.md#rotas-mutantes-com-bot_http_admin_token).
- `state.rs` `state_tests`: `for_http_server_wires_process_wide_bot_runtime_like_serve` (mesmo `Arc` que `shared_bot_runtime()` / `HttpApiSeams::from_env`).
- `./scripts/verify-backend-gates.sh` (sincronizar contagem `passed` com a linha `OK:` após mudanças).
