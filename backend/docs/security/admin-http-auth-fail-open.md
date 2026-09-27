---
title: Achado — auth admin HTTP documentada como fail-closed é fail-open
description: BOT_HTTP_ADMIN_TOKEN ausente deixa todas as rotas mutantes abertas; severidade, locais e critérios de aceite SEC-ADM
tags:
  - security
  - threat-model
  - backend
  - http
  - finding
status: draft
---

# Achado F-ADM-01 — auth admin HTTP "fail-closed" é, na prática, opcional (fail-open)

- **Estado:** proposto — aguardando revisão do Critic independente. Documentação apenas; nenhuma correção foi aplicada.
- **Origem:** auditoria read-only em HEAD `acb7a97e` (arquivo de apoio fora do repositório, "module-audit-2026-09-27", item 6 de "Docs claim more than code"), **verificada estaticamente** por este documento no checkout local em HEAD `2d1e3863` (os arquivos citados não mudaram entre `0880cdd5` e `2d1e3863`; working tree sem alterações em `backend/src`).
- **Data:** 2026-09-27. Nenhum valor de token ou `.env` foi lido; a presença do token foi avaliada apenas pelo formato do código/config.
- **Relacionados:** [org-module-threat-model](./org-module-threat-model.md), [orders-g2-threat-model](./orders-g2-threat-model.md), [provider-credentials-plaintext](./provider-credentials-plaintext.md).

## 1. Evidência (estática)

| Item | Local | Observação |
|---|---|---|
| Ramo sem token | `backend/src/presentation/http/admin_auth.rs:91-94` | `verify_headers`: `if self.token.is_none() { return Ok(()); }` — sem token configurado, **toda** verificação passa. |
| Fonte do token | `backend/src/core/config/http/file.rs:3-17` | `HttpAdminAuthConfig::from_env()` lê só `BOT_HTTP_ADMIN_TOKEN` (via `env_nonempty`); não há leitura de `system.toml` nem validação de força. `derive(Debug)` com `token`. |
| Config padrão | `backend/src/core/config/system.toml:38-39` | `[http.admin]` vazio; só um comentário apontando para `.env`. Ou seja, o padrão distribuído é auth desligada. |
| Construção | `admin_auth.rs:19-37` | `from_env`/`from_config`; owner binding só com token; nenhum erro quando o token falta. |
| `Debug` com token | `admin_auth.rs:7-12` | `#[derive(Clone, Debug, Default)] struct HttpAdminAuth { token: Option<String>, … }`. |
| Comparação | `admin_auth.rs:143-152` | `constant_time_eq` retorna cedo se os tamanhos diferem (vaza tamanho). |
| Aplicação | `backend/src/presentation/http/state.rs:800-802` + chamadas por handler | `require_http_admin` é chamado manualmente em cada handler (não há layer/middleware); esquecer a chamada numa rota nova a deixa aberta. |
| Startup | `backend/src/presentation/http/server.rs:16-73` | Só registra `http_admin_auth_enabled` em log `info`; não recusa subir sem auth. |
| Bind | `backend/src/presentation/http/cli.rs:9-10` | Padrão `127.0.0.1:8080` (mitigação); `--bind` aceita qualquer endereço sem checagem. |
| Router | `server.rs:81-95` | Sem layer de validação de `Host` nem CORS explícito; um serviço sem auth em loopback fica sujeito a DNS rebinding a partir de um navegador local (**não testado**, inferência estática). |

### Cobertura das rotas (`routes/mod.rs:25-93`)

**Mutantes que chamam `require_http_admin` (portanto abertas quando o token falta):** `POST/PUT/DELETE /admin/provider-credentials*` (`provider_credentials_admin.rs:42, 67, 93, 117` — inclui o `GET` de listagem mascarada), `POST /agents`, `/agents/{id}/pause|resume|retire|advisory` (`agents.rs:43, 100, 121, 142, 166`), `POST /bots/catalog/persist`, `/bots/runtime/promote|demote` (`bots.rs:60, 117, 135`), `POST /orders/reconciliation/poll`, `POST /orders/submit` (`orders.rs:73, 99`), `POST /monitor/commands` (`monitor.rs:38`), `GET /admin/graph/*` (`graph_admin.rs:60, 82, 106, 131`).

**POST sem nenhuma chamada de auth (mesmo com token):** `/risk/profile-limits`, `/risk/validate-intent`, `/risk/gate-signal`, `/strategy/evaluate-sma`, `/bots/ranking`, `/backtest/sma-crossover` — declaradas de simulação/cálculo e "abertas" por decisão no SDD do seam (seção "Fora de escopo"). Precisam de prova de ausência de efeito colateral (SEC-ADM-03).

**Leituras sem auth (mesmo com token):** `GET /agents`, `/agents/{id}`, `/agents/audit` (agency da query), `GET /orders/reconciliation/{client_order_id}`, `/config/*`, `/monitor/snapshot`, `/portfolio/paper-snapshot`, `/bots/catalog*`, `/bots/runtime/status`, `/exchanges/*`, `/providers/status`, `/meta`, `/openapi.json`, `/docs`, `/healthz`, `/readyz`.

### Documentação que afirma "fail-closed"

- `backend/docs/sdd/http-admin-auth-seam-sdd.md:2` (título "HTTP admin bearer seam (fail-closed)") e `:16` ("bearer fail-closed em rotas mutantes") — contraditório com a própria tabela "Comportamento" do mesmo SDD ("`BOT_HTTP_ADMIN_TOKEN` ausente/vazio → Sem exigência de bearer").
- `backend/docs/reference/cli-and-config.md:138` ("…exigem `Authorization: Bearer <token>` (fail-closed…)").
- `backend/src/presentation/http/admin_auth.rs:1` (doc comment "Optional fail-closed admin bearer").

A afirmação só vale **quando** o token está definido; o padrão distribuído (`system.toml`) não o define.

## 2. Severidade

**Alto** no padrão atual; **Crítico** em qualquer implantação em que a porta seja alcançável por terceiros (`--bind` não-loopback, porta publicada de container, proxy reverso) ou em que existam chaves de valor real.

Justificativa:

- **Impacto:** controle total das rotas mutantes sem credencial: sobrescrever/apagar chaves de LLM em `provider_credentials` (armazenadas em texto claro, `provider_credentials_encryption=none` — ver [F-CRED-01](./provider-credentials-plaintext.md)); trocar a chave de um provider por uma do atacante, fazendo prompts/respostas trafegarem numa conta controlada por ele (exfiltração plausível, depende do provider) ou negar serviço; registrar agentes com `promote_runtime_bot=true` e promover bots; pausar/aposentar agentes; comandos de monitor; disparar poll de reconciliação (chamadas à exchange); `POST /orders/submit` quando o operador habilita `paper`/`dev_accept`/`live_exchange` (testnet/recording), agravado pelos limites de risco vindos do corpo ([F-ORD-01](./orders-g2-threat-model.md)). O `GET` de credenciais devolve só `****` + 4 últimos caracteres, então a leitura integral do segredo **não** é possível via API.
- **Probabilidade:** o padrão é auth desligada; nada impede `--bind 0.0.0.0` sem token; não há validação de `Host`, o que torna DNS rebinding um vetor plausível mesmo em loopback (não verificado dinamicamente).
- **Mitigações existentes:** bind padrão loopback; `orders.execution` padrão `disabled` (503); prod REST bloqueado; runtime de bots desligado por padrão; credenciais mascaradas na listagem.
- **Por que não Crítico no padrão:** exige alcance à porta loopback (processo local ou rebinding) e o dano financeiro direto hoje está limitado a testnet/paper.

## 3. Correção recomendada

Tornar a auth admin realmente fail-closed: o servidor não sobe (ou não monta rotas admin/mutantes) sem auth configurada; opt-out explícito só para dev e só em loopback; enforcement central por layer; token forte, redigido e comparado em tempo constante; docs alinhadas. A auth admin continua sendo **seam** e não substitui owner auth (P1) — ver SEC-ORG-20 e SEC-CRED-07.

## 4. Critérios de aceite testáveis (SEC-ADM)

| ID | Critério |
|---|---|
| **SEC-ADM-01** | `bot serve` (ou entrypoint HTTP) sem auth admin configurada **não inicia**: encerra com código ≠ 0 antes de abrir o socket e mensagem estável `admin_auth_not_configured` (sem ecoar valores). Teste de CLI em `backend/tests/` (padrão de `config_cli.rs`) verifica exit code, stderr e que a porta não foi aberta. |
| **SEC-ADM-02** | Opt-out apenas por flag explícita de dev (nome a definir, ex.: `BOT_HTTP_ADMIN_AUTH_DISABLED_DEV=1`), aceita **somente** com bind loopback (`127.0.0.1`/`::1`) e perfil não produtivo; com bind não-loopback, ou com perfil de produção/`Environment::Prod`/`production.enabled=true`, o processo recusa subir (exit ≠ 0). Com opt-out ativo: log `WARN` no boot e `GET /meta` → `http_seams.http_admin_auth_mode = "dev_disabled"`. Testes para as quatro combinações. |
| **SEC-ADM-03** | Teste de cobertura de rotas: enumera todas as operações `POST/PUT/PATCH/DELETE` do router (ou do OpenAPI gerado, conferido contra o router) e afirma **401** `unauthorized` sem `Authorization` e com token errado. Rotas públicas de cálculo ficam numa allowlist revisada no próprio teste, cada uma com teste provando ausência de persistência/efeito externo. Nova rota mutante sem auth faz o teste falhar. |
| **SEC-ADM-04** | Enforcement central: auth aplicada por layer/middleware no sub-router admin/mutante, não por chamada manual em cada handler; teste com handler de exemplo sem chamada explícita ainda retorna 401. |
| **SEC-ADM-05** | Leituras sensíveis exigem auth: `/admin/*`, `GET /agents*`, `GET /orders/reconciliation/{id}`, `/config/snapshot` (lista final decidida no SDD) → **401** sem credencial. |
| **SEC-ADM-06** | Comparação em tempo constante sem vazar tamanho: comparar digests SHA-256 de tamanho fixo (ou `subtle::ConstantTimeEq` sobre digests). Teste de correção (igual/diferente/tamanhos distintos) + revisão do Critic de que não há retorno antecipado por tamanho. |
| **SEC-ADM-07** | Força mínima verificada no startup: ≥ 32 bytes de entropia (ex.: ≥ 43 caracteres base64url ou 64 hex), sem espaços, rejeitando placeholders conhecidos (`changeme`, `secret`, `token`, `admin`, caractere repetido). Violação → exit ≠ 0 com mensagem que não contém o valor. Testes para cada regra. |
| **SEC-ADM-08** | Token nunca logado nem impresso: `format!("{:?}", HttpAdminAuth)` e `HttpAdminAuthConfig` contêm `<redacted>`; captura de tracing em boot, 401 e 200 não contém o token de fixture; `TraceLayer` não registra o header `Authorization`. |
| **SEC-ADM-09** | Parsing estrito do header: múltiplos `Authorization` → 401; token com espaço interno → 401; esquema diferente de `Bearer` → 401. |
| **SEC-ADM-10** | Proteção contra DNS rebinding: requisição com `Host` fora da allowlist (`localhost`, `127.0.0.1`, `[::1]`, hosts configurados) → **421** ou **403**; sem CORS permissivo (`Access-Control-Allow-Origin: *` ausente). |
| **SEC-ADM-11** | Limite de tentativas: mais de N respostas 401 por minuto por origem → **429**; evento de log sem o token. |
| **SEC-ADM-12** | Rotação: suporte a token atual + próximo durante janela configurada (ambos aceitos), ou procedimento de restart documentado; após remover o antigo, ele recebe 401. |
| **SEC-ADM-13** | Documentação alinhada ao comportamento: título/escopo de `http-admin-auth-seam-sdd.md`, a linha de `BOT_HTTP_ADMIN_TOKEN` em `cli-and-config.md` e o doc comment de `admin_auth.rs:1` descrevem o novo modo; a tabela "Comportamento" passa a dizer "ausente → processo não inicia (exceto opt-out dev em loopback)". Verificação: busca por "Optional fail-closed" retorna vazio e o Critic confere docs × testes SEC-ADM-01/02. |
| **SEC-ADM-14** | Auth admin não substitui owner auth: rotas de `org`, de mutação de credenciais e de kill switch de orders exigem principal P1 quando P1 existir (SEC-ORG-20, SEC-CRED-07, SEC-ORD-12); com apenas token admin válido essas rotas retornam 401/503. |

**Impacto em testes existentes:** testes HTTP que hoje rodam com auth desabilitada (`HttpAdminAuth::disabled()`) precisarão usar token de fixture ou o opt-out de dev; isso faz parte da correção (Builder + Critic).

## 5. Itens não verificados

- Se alguma implantação real usa `--bind` não-loopback ou define o token (não lido, por regra).
- Exploração por DNS rebinding e comportamento de navegadores (inferência estática; nenhum teste ou probe executado).
- Ausência de efeito colateral das rotas POST de cálculo (leitura superficial; deve ser provada por teste — SEC-ADM-03).
