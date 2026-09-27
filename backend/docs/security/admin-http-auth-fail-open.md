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

- **Estado:** proposto — aguardando revisão do Critic independente (ciclo 2 de ENTREGA SEC-TM; follow-ups A1-A6 do ciclo 1 aplicados, ver §6). Documentação apenas; nenhuma correção foi aplicada.
- **Origem:** auditoria read-only em HEAD `acb7a97e` (arquivo de apoio fora do repositório, "module-audit-2026-09-27", item 6 de "Docs claim more than code"), **verificada estaticamente** por este documento no checkout local em HEAD `2d1e3863` (os arquivos citados não mudaram entre `0880cdd5` e `2d1e3863`; working tree sem alterações em `backend/src`). Ciclo 2: os arquivos citados foram reconferidos por sha256 no checkout em HEAD `bb0d320f` e são idênticos aos lidos em `2d1e3863`.
- **Data:** 2026-09-27. Nenhum valor de token ou `.env` foi lido; a presença do token foi avaliada apenas pelo formato do código/config.
- **Relacionados:** [org-module-threat-model](./org-module-threat-model.md), [orders-g2-threat-model](./orders-g2-threat-model.md), [provider-credentials-plaintext](./provider-credentials-plaintext.md), [owner-auth-idp-threat-model](./owner-auth-idp-threat-model.md).

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

### Cobertura das rotas (`routes/mod.rs:27-98`; `/openapi.json` e `/docs` em `server.rs:86-91`)

**Mutantes que chamam `require_http_admin` (portanto abertas quando o token falta):** `POST/PUT/DELETE /admin/provider-credentials*` (`provider_credentials_admin.rs:42, 67, 93, 117` — inclui o `GET` de listagem mascarada), `POST /agents`, `/agents/{id}/pause|resume|retire|advisory` (`agents.rs:43, 100, 121, 142, 166`), `POST /bots/catalog/persist`, `/bots/runtime/promote|demote` (`bots.rs:60, 117, 135`), `POST /orders/reconciliation/poll`, `POST /orders/submit` (`orders.rs:73, 99`), `POST /monitor/commands` (`monitor.rs:38`), `GET /admin/graph/*` (`graph_admin.rs:60, 82, 106, 131`).

**POST sem nenhuma chamada de auth (mesmo com token):** `/risk/profile-limits`, `/risk/validate-intent`, `/risk/gate-signal`, `/strategy/evaluate-sma`, `/bots/ranking`, `/backtest/sma-crossover` — declaradas de simulação/cálculo e "abertas" por decisão no SDD do seam (seção "Fora de escopo"). Precisam de prova de ausência de efeito colateral (SEC-ADM-03).

**Leituras sem auth (mesmo com token):** `GET /agents`, `/agents/{id}`, `/agents/audit` (agency da query), `GET /orders/reconciliation/{client_order_id}`, `/config/*`, `/monitor/snapshot`, `/portfolio/paper-snapshot`, `/bots/catalog*`, `/bots/runtime/status`, `/exchanges/*`, `/providers/status`, `/application/signals` (`routes/mod.rs:28`), `/strategy/periods` (`:70`), `/orders/execution-status` (`:81`), `/meta`, `/openapi.json`, `/docs`, `/healthz`, `/readyz`.

**Conferência A5 (ciclo 2):** as listas acima agora cobrem os 25 `GET` registrados em `routes/mod.rs:27-98` mais os 2 de `server.rs`. Faltavam **exatamente 3** (`/application/signals`, `/strategy/periods`, `/orders/execution-status`), e não 4 como contou o Critic. Os demais `GET` já estavam cobertos por curinga (`/config/*` = `active` e `snapshot`; `/bots/catalog*` inclui `snapshot`; `/exchanges/*` = `catalog` e `routing`). Se o Critic tiver uma quarta rota em mente, ela não aparece no router em `bb0d320f`. `GET /orders/execution-status` revela o modo de execução (`disabled`/`paper`/`live_exchange`), o que ajuda a escolher alvo; por isso entra na classificação de SEC-ADM-05.

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

### 2.1 Intenção de implantação (Deployment Intent, estrutura Mantis)

A diferença entre Alto e Crítico depende só de onde a porta fica exposta. Nada no código fixa isso: o padrão é loopback (`cli.rs:9-10`), mas `--bind` aceita qualquer endereço sem checagem e o perfil de produção existe (`production.enabled`). O owner ainda não declarou o nível pretendido.

| Nível | Exposição | Severidade de F-ADM-01 | Pré-requisito mínimo |
|---|---|---|---|
| E0 | Só loopback, operador único na máquina | **Alto** | SEC-ADM da fatia 1 |
| E1 | Container com porta publicada, LAN, proxy reverso ou `--bind` não-loopback | **Crítico** | fatia 1 + follow-up (SEC-ADM-05, 09, 10, 11, 12) |
| E2 | Qualquer nível com chaves de valor real (provider pago, exchange mainnet) | **Crítico** | E1 + P1 (SEC-ADM-14) + SEC-CRED-01/02 |

Até o owner declarar o nível, este documento assume E0 para a severidade e trata qualquer passo para E1/E2 como bloqueado.

## 3. Correção recomendada

Tornar a auth admin realmente fail-closed: o servidor não sobe (ou não monta rotas admin/mutantes) sem auth configurada; opt-out explícito só para dev e só em loopback; enforcement central por layer; token forte, redigido e comparado em tempo constante; docs alinhadas. A auth admin continua sendo **seam** e não substitui owner auth (P1) — ver SEC-ORG-20 e SEC-CRED-07.

## 4. Critérios de aceite testáveis (SEC-ADM)

**Níveis (A1):** **F1** = fatia 1, bloqueia o fechamento da correção de F-ADM-01. **FU** = follow-up, obrigatório antes de qualquer exposição E1/E2 (§2.1). **P1** = depende de owner auth.

| ID | Nível | Critério |
|---|---|---|
| **SEC-ADM-01** | F1 | `bot serve` (ou entrypoint HTTP) sem auth admin configurada **não inicia**: encerra com código ≠ 0 antes de abrir o socket e mensagem estável `admin_auth_not_configured` (sem ecoar valores). Teste de CLI em `backend/tests/` (padrão de `config_cli.rs`) verifica exit code, stderr e que a porta não foi aberta. |
| **SEC-ADM-02** | F1 | Opt-out apenas por flag explícita de dev (nome a definir, ex.: `BOT_HTTP_ADMIN_AUTH_DISABLED_DEV=1`), **lida somente por `core::config`** (campo em `HttpAdminAuthConfig`, `core/config/http/file.rs`, com a chave em `[http.admin]` do `system.toml` e override por env pelos helpers de `core/config/load.rs`), nunca por `std::env::var` na camada `presentation`; um teste de guarda falha se o nome da flag aparecer fora de `core/config/`. A flag é aceita **somente** com bind loopback (`127.0.0.1`/`::1`) e perfil não produtivo; com bind não-loopback, ou com perfil de produção/`Environment::Prod`/`production.enabled=true`, o processo recusa subir (exit ≠ 0). Com opt-out ativo: log `WARN` no boot e `GET /meta` → `http_seams.http_admin_auth_mode = "dev_disabled"`. Testes para as quatro combinações. |
| **SEC-ADM-03** | F1 | Fonte única de verdade (A2): uma tabela declarativa de rotas (ex.: `RouteSpec { method, path, handler, class }`, com `class ∈ {Public, PublicCompute, Admin, Owner}`) da qual o router é construído; o OpenAPI é conferido contra ela. A allowlist de rotas sem auth existe **só** nessa tabela, não no teste. Testes: (a) todo par método/caminho do router está na tabela e vice-versa; (b) toda rota `POST/PUT/PATCH/DELETE` e todo `GET` com classe diferente de `Public`/`PublicCompute` responde **401** `unauthorized` sem `Authorization` e com token errado; (c) cada rota `PublicCompute` tem teste provando ausência de persistência e de chamada externa. Nova rota fora da tabela, ou mutante marcada `Public`, faz o teste falhar. |
| **SEC-ADM-04** | F1 | Enforcement central: auth aplicada por layer/middleware no sub-router admin/mutante, não por chamada manual em cada handler; teste com handler de exemplo sem chamada explícita ainda retorna 401. |
| **SEC-ADM-05** | F1 | Leituras sensíveis exigem auth: `/admin/*`, `GET /agents*`, `GET /orders/reconciliation/{id}`, `/config/snapshot`, `/orders/execution-status` → **401** sem credencial. Cada um dos 27 `GET` atuais (§1, incluindo `/application/signals` e `/strategy/periods`) recebe classe explícita na tabela de SEC-ADM-03; a lista final é decisão do SDD, mas nenhum `GET` fica sem classe. |
| **SEC-ADM-06** | F1 | Comparação em tempo constante sem vazar tamanho: comparar digests SHA-256 de tamanho fixo (ou `subtle::ConstantTimeEq` sobre digests). Teste de correção (igual/diferente/tamanhos distintos) + revisão do Critic de que não há retorno antecipado por tamanho. |
| **SEC-ADM-07** | F1 | Força mínima verificada no startup: ≥ 32 bytes de entropia (ex.: ≥ 43 caracteres base64url ou 64 hex), sem espaços, rejeitando placeholders conhecidos (`changeme`, `secret`, `token`, `admin`, caractere repetido). Violação → exit ≠ 0 com mensagem que não contém o valor. Testes para cada regra. |
| **SEC-ADM-08** | F1 | Token nunca logado nem impresso: `format!("{:?}", HttpAdminAuth)` e `HttpAdminAuthConfig` contêm `<redacted>`; captura de tracing em boot, 401 e 200 não contém o token de fixture; `TraceLayer` não registra o header `Authorization`. |
| **SEC-ADM-09** | FU | Parsing estrito do header: múltiplos `Authorization` → 401; token com espaço interno → 401; esquema diferente de `Bearer` → 401. |
| **SEC-ADM-10** | FU | Proteção contra DNS rebinding: requisição com `Host` fora da allowlist (`localhost`, `127.0.0.1`, `[::1]`, hosts configurados) → **421** ou **403**; sem CORS permissivo (`Access-Control-Allow-Origin: *` ausente). |
| **SEC-ADM-11** | FU | Limite de tentativas (A6): chave = IP do peer TCP (`ConnectInfo<SocketAddr>`); `X-Forwarded-For`/`Forwarded` só é considerado quando o peer está numa lista `trusted_proxies` configurada, senão é ignorado. No padrão loopback todos os clientes locais compartilham `127.0.0.1`, então o limitador vira um balde global; isso é aceito em E0 e documentado. Orçamento N por minuto = decisão do owner **D-SEC-ADM-RATE**. Teste: mais de N respostas 401 por minuto para a mesma chave → **429**; header `X-Forwarded-For` forjado vindo de peer não confiável não muda a chave; evento de log sem o token. |
| **SEC-ADM-12** | FU | Rotação: suporte a token atual + próximo durante janela configurada (ambos aceitos), ou procedimento de restart documentado; após remover o antigo, ele recebe 401. |
| **SEC-ADM-13** | F1 | Documentação alinhada ao comportamento: título/escopo de `http-admin-auth-seam-sdd.md`, a linha de `BOT_HTTP_ADMIN_TOKEN` em `cli-and-config.md` e o doc comment de `admin_auth.rs:1` descrevem o novo modo; a tabela "Comportamento" passa a dizer "ausente → processo não inicia (exceto opt-out dev em loopback)". Verificação: busca por "Optional fail-closed" retorna vazio e o Critic confere docs × testes SEC-ADM-01/02. |
| **SEC-ADM-14** | P1 | Auth admin não substitui owner auth: rotas de `org`, de mutação de credenciais e de kill switch de orders exigem principal P1 quando P1 existir (SEC-ORG-20, SEC-CRED-07, SEC-ORD-12); com apenas token admin válido essas rotas retornam 401/503. |
| **SEC-ADM-15** | F1 | Caminho de dev documentado (A4): `cli-and-config.md` explica como gerar um token local que passe em SEC-ADM-07 (ex.: `openssl rand -base64 32`) e onde colocá-lo (arquivo local não versionado, já ignorado pelo git); testes usam um token de fixture constante ≥ 32 bytes num helper de teste; `HttpAdminAuth::disabled()` passa a existir só em `#[cfg(test)]`. Verificação: `cargo build` sem `cfg(test)` não encontra `disabled()`; o passo a passo da doc, seguido do zero, sobe o servidor em loopback. |

**Impacto em testes existentes:** testes HTTP que hoje rodam com auth desabilitada (`HttpAdminAuth::disabled()`) precisarão usar token de fixture ou o opt-out de dev; isso faz parte da correção (Builder + Critic).

## 5. Itens não verificados

- Se alguma implantação real usa `--bind` não-loopback ou define o token (não lido, por regra).
- Exploração por DNS rebinding e comportamento de navegadores (inferência estática; nenhum teste ou probe executado).
- Ausência de efeito colateral das rotas POST de cálculo (leitura superficial; deve ser provada por teste — SEC-ADM-03).
- Nível de implantação pretendido (§2.1): pendente de declaração do owner.

## 6. Registro do ciclo 2 (follow-ups A1-A6 do Critic)

O texto literal dos itens A1-A6 do ciclo 1 não estava disponível nesta sessão de edição; o mapeamento abaixo segue os temas repassados pelo coordenador. Se algum item não corresponder, o Critic deve apontar no ciclo 2.

| Item | O que mudou | Seção |
|---|---|---|
| A1 | Critérios divididos em níveis F1 (fatia 1), FU (follow-up antes de E1/E2) e P1; nova §2.1 com a premissa de exposição | §2.1, §4 (coluna "Nível") |
| A2 | SEC-ADM-03 passa a exigir tabela declarativa de rotas como fonte única; allowlist mora nela, não no teste | §4 SEC-ADM-03 |
| A3 | Flag de opt-out lida só via `core::config` (`HttpAdminAuthConfig`/`[http.admin]`), com teste de guarda | §4 SEC-ADM-02 |
| A4 | Novo SEC-ADM-15: caminho de dev para gerar token, fixture de teste, `disabled()` só em teste | §4 SEC-ADM-15 |
| A5 | Três `GET` que faltavam adicionados; divergência com a contagem do Critic (4) registrada | §1 "Cobertura das rotas", §4 SEC-ADM-05 |
| A6 | Chave do rate limit definida (peer TCP, `trusted_proxies`), comportamento em loopback explicado, N vira D-SEC-ADM-RATE | §4 SEC-ADM-11 |

Severidade de F-ADM-01 sem mudança: **Alto** em E0, **Crítico** em E1/E2.
