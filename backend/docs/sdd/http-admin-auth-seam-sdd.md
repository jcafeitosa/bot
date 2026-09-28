---
title: SDD — HTTP admin bearer seam (serve sem token; rotas protegidas retornam 503)
description: BOT_HTTP_ADMIN_TOKEN, BOT_HTTP_OWNER_ID e BOT_HTTP_AGENCY_ID; não substitui Gate 1 owner auth
tags:
  - sdd
  - backend
  - security
  - http
  - wave0
status: partial
w0_01_status: ready-for-g1-review
---

# SDD — HTTP admin bearer seam (serve sem token; rotas protegidas retornam 503)

### Decisão do owner — comportamento sem credencial (2026-09-27)

Owner aprovou este contrato em 2026-09-27: `serve` permanece ativo sem `BOT_HTTP_ADMIN_TOKEN`; mutações e GETs sensíveis são protegidos; rotas de health/docs/leitura e os seis POSTs de cálculo sem efeito colateral são públicos; token ausente ou fraco deixa o servidor ativo e faz rotas protegidas responderem **503**; token configurado válido exige Bearer válido; bearer incorreto retorna **401**; rota sem classificação retorna **404** antes de executar handler. Isso substitui a proposta incompatível de falhar todo o startup sem token (A1/A3). A decisão sobre comportamento e classes não escolhe a arquitetura interna do router nem aprova Host allowlist, rate limit ou detalhes do OpenAPI.

#### Contrato observável aprovado

| Estado da configuração/credencial | Rotas protegidas | Rotas públicas |
|---|---|---|
| Token ausente, vazio ou fraco | **503** `admin_auth_not_configured`; handler não executa; servidor/listener continuam ativos | resposta normal, sem bearer |
| Token configurado conforme política, bearer ausente ou incorreto | **401** `unauthorized`; handler não executa | resposta normal, sem bearer |
| Token configurado conforme política, bearer válido | requisição prossegue e preserva a resposta normal da operação | resposta normal, sem bearer |
| Rota sem classificação | **404** `route_not_found`; handler não executa | — |

Token válido tem no mínimo 32 bytes aleatórios e não contém espaços. Token ausente, vazio ou que não satisfaz esses requisitos é configuração não utilizável para autenticar; não desabilita nem torna públicas as rotas protegidas. A implementação pode validar uma vez ao carregar configuração, mas não deve interromper `serve` por token ausente/fraco. O tratamento exato de outras formas inválidas, além de ausente/vazio/fraco/espaços, deve seguir a mesma regra fail-closed de 503 e nunca ecoar o valor.

#### Classificação aprovada das rotas existentes

Os paths abaixo são relativos ao prefixo `/api/v1`, salvo os quatro endpoints de sistema/docs explicitamente fora dele.

**Protegidas — todas as mutações/admin:**

- todos os métodos em `/api/v1/admin/*`;
- `POST /agents`, `POST /agents/{agent_id}/pause`, `POST /agents/{agent_id}/resume`, `POST /agents/{agent_id}/retire`, `POST /agents/{agent_id}/advisory`;
- `POST /bots/catalog/persist`, `POST /bots/runtime/promote`, `POST /bots/runtime/demote`;
- `POST /orders/reconciliation/poll`, `POST /orders/submit`;
- `POST /monitor/commands`.

**Protegidas — leituras sensíveis:**

- `GET /meta`, `GET /config/active`, `GET /config/snapshot`;
- `GET /agents/audit`, `GET /agents`, `GET /agents/{agent_id}`;
- `GET /orders/execution-status`, `GET /orders/reconciliation/{client_order_id}`;
- `GET /monitor/snapshot`, `GET /bots/runtime/status`;
- todos os métodos em `/api/v1/admin/*`, conforme acima.

**Públicas — health/docs e leituras não sensíveis:**

- fora de v1: `GET /healthz`, `GET /readyz`, `GET /openapi.json`, `GET /docs`;
- em `/api/v1`: `GET /application/signals`, `GET /providers/status`, `GET /exchanges/catalog`, `GET /exchanges/routing`, `GET /strategy/periods`, `GET /portfolio/paper-snapshot`, `GET /bots/catalog`, `GET /bots/catalog/snapshot`.

**Públicas — seis POSTs de cálculo sem efeito colateral:**

- `POST /risk/profile-limits`, `POST /risk/validate-intent`, `POST /risk/gate-signal`;
- `POST /strategy/evaluate-sma`, `POST /bots/ranking`, `POST /backtest/sma-crossover`.

Esta classificação é decisão do owner e substitui qualquer classe diferente na tabela histórica A2.2. Toda rota existente deve constar de uma das listas; futura rota exige classe explícita. Para rota sem classificação o comportamento aprovado é 404 `route_not_found`, sem chamar handler. Não usar a proposta anterior de 401 deny-by-default para rota desconhecida.

#### Seams de implementação — decisões de comportamento fechadas

1. **Configuração:** ler `BOT_HTTP_ADMIN_TOKEN`; considerar utilizável somente valor com pelo menos 32 bytes aleatórios, sem espaços; ausente/vazio/fraco é estado `NotConfigured`, não erro fatal de startup.
2. **Verificação:** Bearer válido compara com o token configurado sem expor o segredo; ausente/incorreto resulta em 401 nas rotas protegidas; auth sem token nunca equivale a sucesso.
3. **Classificação:** a lista método+path acima é a classificação aprovada: mutações e GETs sensíveis protegidos; leituras/health/docs e exatamente seis cálculos públicos.
4. **Rota futura desconhecida:** 404 `route_not_found` antes do handler; não herda auth pública nem protegida por omissão.
5. **Fluxo do router:** precisa aplicar a classificação antes do handler e incluir rotas de sistema/docs; nome e composição internos de função/middleware ficam como decisão de implementação revisável em G1, sem mudar os contratos observáveis.
6. **Respostas:** 503 para auth ausente/vazia/fraca, 401 para bearer ausente/incorreto quando token utilizável está configurado, 404 para rota não classificada; Bearer válido prossegue. Os códigos acima são estáveis e segredos nunca aparecem em respostas/logs.



#### Alternativas e trade-offs

- **Contrato aprovado: responder 503 nas rotas protegidas quando config ausente.** Mantém `serve` e rotas públicas utilizáveis, comunica indisponibilidade de auth na operação protegida e evita executar o handler; exige distinguir “auth não configurada” (503) de credencial rejeitada (401).
- **Startup falha sem token (decisão antiga, substituída).** Impede qualquer uso do serviço, inclusive rotas públicas e probes, por uma configuração opcional; owner rejeitou esse comportamento.
- **Deixar rotas protegidas abertas sem token.** Mantém compatibilidade, mas permite operações administrativas sem autenticação; rejeitada pelo contrato aprovado.
- **Responder 401 quando token não está configurado.** Confunde ausência de configuração do servidor com falha de credencial do cliente; contrato aprovado escolhe 503.

#### Riscos, rollout e rollback

Riscos: caminho de rota novo sem classificação pode ficar aberto; classificação por prefixo pode capturar rotas públicas indevidamente; 503 em config ausente pode ser tratado como indisponibilidade pelo balanceador; validação e comparação do segredo podem vazar informação ou imprimir token; manter chamadas de auth nos handlers além da camada de proteção pode duplicar respostas. A decisão de roteamento pendente precisa cobrir cada risco antes da implementação.

Rollout proposto: implementar 503/401/continuação primeiro em testes do seam público acordado; proteger a lista aprovada de rotas sem mudar as públicas; observar contadores/logs sem registrar credenciais; documentar env e atualizar OpenAPI após acordo sobre classificação. Rollback: reverter apenas o middleware/guard e bindings introduzidos pela fatia, mantendo a instância anterior disponível; não reverter para comportamento sem token aberto. Como o comando continua subindo, a recuperação operacional é configurar token válido e reiniciar, ou reverter a fatia se a proteção bloquear tráfego válido.

#### Validação comportamental observável (design; ainda não executada)

Para uma rota protegida representativa e para cada classe/lista aprovada: sem env, resposta 503 + `admin_auth_not_configured`, handler não chamado; com env e sem bearer / bearer inválido, resposta 401 + `unauthorized`, handler não chamado; com env e bearer válido, handler chamado uma vez e resposta normal. Para cada rota pública, sem env e sem bearer, resposta normal. Para rota futura sem classificação, handler não chamado e resposta de negação conforme seam escolhido. Cobrir também o fluxo real de `serve`: listener ativo sem token, rota pública respondendo e rota protegida retornando 503. Os nomes de helper, localização do middleware, status/corpos estáveis e estratégia de lista exigem acordo antes de testes TDD; nenhum teste foi executado nesta entrega de documentação.

#### Seams públicos de comportamento — decisões fechadas pelo owner em 2026-09-27

Os seis seams de comportamento foram fechados pelo owner:

1. **Configuração:** token deve vir de `BOT_HTTP_ADMIN_TOKEN`, conter pelo menos 32 bytes aleatórios e nenhum espaço; ausente/vazio/fraco mantém o servidor ativo e retorna 503 nas protegidas.
2. **Autenticação:** Bearer válido prossegue; Bearer ausente/incorreto com token utilizável retorna 401; nunca tratar auth ausente como sucesso.
3. **Classes das rotas atuais:** lista completa método+path acima; mutações e leituras sensíveis protegidas; health/docs/leitura não sensível e os seis cálculos públicos.
4. **Rotas futuras:** classificar explicitamente; rota sem classe retorna 404 `route_not_found` antes do handler.
5. **Respostas:** 503 `admin_auth_not_configured`, 401 `unauthorized`, 404 `route_not_found`; rotas públicas preservam resposta normal sem bearer.
6. **Escopo do middleware:** toda rota protegida passa pela decisão antes do handler; detalhes do tipo Rust, assinatura de funções, composição do router e anotação OpenAPI são decisões de design interno para revisão G1, não seams de comportamento pendentes do owner.

Não há acordo pendente do owner para começar a revisão G1. O Critic deve revisar se as interfaces e a composição propostas em A2–A6 implementam fielmente estes contratos; implementação e testes seguem bloqueados até aprovação G1 e eventual acordo sobre API pública de teste conforme AGENTS.md.

1. Qual API/config seam expõe o estado de configuração: tipo/enum e campos para ausente, vazio, válido e inválido; regra de força/validação do token e ponto único de leitura do env.
2. Função/método público que decide/verifica credencial e sua assinatura: entrada do request, como representar segredo sem expô-lo em `Debug`, saída de decisão e erros/códigos públicos exatos.
3. Lista completa método + path + classe para cada rota existente, em especial GETs sensíveis e as seis POSTs de cálculo atualmente públicas; decidir se novas rotas sem classificação falham na construção, retornam 401/503 em runtime ou são barradas por outro seam.
4. Ponto de aplicação da autenticação no router e extensão do seam para registro de rotas futuras; confirmar como health, readiness, OpenAPI e docs são tratados.
5. Contrato público exato de resposta 503, 401 e de bearer válido (status, corpo/código, headers) e comportamento quando a variável está presente mas inválida.
6. OpenAPI: como declarar bearer e `security` por operação, após a lista ficar acordada.



## W0-01 — fail-closed de verdade (Onda 0, proposta para G1) [SEGURANÇA]

- **Estado desta seção: pronta para revisão independente G1; G1 ainda não aprovado.** As decisões do owner foram registradas em 2026-09-27; Critic independente deve revisar o design atualizado antes de qualquer implementação ou teste. Vale só para esta seção: o `status: partial` do front-matter descreve as seções "Gate 1" em diante, que documentam o código atual.
- **Plano:** W0-01, prioridade 1, em [master-plan](../planning/master-plan.md) §4.1.
- **Fonte dos critérios de segurança:** [F-ADM-01](../security/admin-http-auth-fail-open.md) §4 (SEC-ADM-01…15). Na versão atual do TM, SEC-ADM-01…11, 13 e 15 são F1, SEC-ADM-12 é FU e SEC-ADM-14 é P1.
- **Leitura importante:** as seções abaixo desta ("Gate 1", "Comportamento") descrevem o código **atual**, que é fail-open sem token. Esta seção descreve o alvo. Quando W0-01 for implementado, a tabela "Comportamento" e o título mudam junto (SEC-ADM-13).
- **Seams:** todos os seams desta seção estão **a acordar com o owner** (tabela "Seams públicos"). Nenhum foi acordado ainda.

### Contexto (evidência no código, HEAD `d42b71a5`)

Entre `b8370a75` e `d42b71a5`, `backend/src` mudou só em `core/database/postgres.rs`, `core/health`, `core/persistence`, `v18_pg_tests.rs` e `agents/adapters/pg_registry.rs`. Nenhum arquivo citado nesta seção mudou; as linhas abaixo foram conferidas de novo em `d42b71a5`.

- `backend/src/presentation/http/admin_auth.rs:91-94`: `verify_headers` devolve `Ok(())` quando não há token (ramo `None` em `:92-93`). Sem `BOT_HTTP_ADMIN_TOKEN`, toda rota "protegida" fica aberta, inclusive CRUD de `provider_credentials`, `orders/submit`, `bots/runtime/promote` e `monitor/commands`.
- `admin_auth.rs:7`: `#[derive(Clone, Debug, Default)]`. O `Default` monta auth **sem token** (aberta) e o `Debug` imprime o token. `disabled()` (`:15-17`) é `Self::default()`.
- `backend/src/core/config/http/file.rs:10-17`: token só vem de env, sem validação de força. `backend/src/core/config/system.toml:38-39`: `[http.admin]` só tem comentário; não existe tabela `[http]` com outras chaves. `backend/.env.example:68` traz a chave vazia.
- `admin_auth.rs:143-152`: `constant_time_eq` sai cedo quando os tamanhos diferem (vaza tamanho).
- `backend/src/presentation/http/routes/mod.rs:25-93`: `v1_routes()` monta as rotas uma a uma, sem layer de auth; cada handler chama `state.require_http_admin` à mão (`state.rs:800-802`). Rota nova sem a chamada fica aberta.
- `backend/src/presentation/http/server.rs:81-94` (`build_router`): além de `/api/v1`, registra `routes::system_routes()` (`/healthz`, `/readyz`, `routes/mod.rs:95-99`), `/openapi.json` (`:87-90`) e o `Scalar` em `/docs` (`:91`), todos **fora** de `v1_routes()`.
- `server.rs:16-75` (`run`): antes de `TcpListener::bind` (`:62`), o processo já conectou e migrou o PG (`:21` → `bundle.rs:47-49` → `:51-71`), subiu o worker do outbox (`:22`) e hidratou agentes, owner bootstrap, reconciliação e catálogo (`state.rs:230-304`). No braço `Serve` de `main.rs` (`:99-138`), o monitor headless sobe antes disso (`main.rs:106-136`, com `bootstrap_monitor_postgres` em `:113`). Um check de boot em `server.rs` chegaria tarde.
- `core/config/mod.rs:381-389`: `Config::validate` recusa `Environment::Prod` sempre. Todo `serve` que sobe é "não prod".
- `presentation/http/cli.rs:9-10`: `--bind` padrão `127.0.0.1:8080`, aceita qualquer endereço.
- **Auth aberta ainda montável em produção (F-01-2):**
  - `HttpApiSeams::disabled_fail_closed()` (`state.rs:47-54`) usa `HttpAdminAuth::disabled()` (`:50`), ou seja, auth admin **aberta**. É chamado por `ApiState::new` (`state.rs:142-160`, em `:157`), usado por `impl Default for ApiState` (`state.rs:1175-1184`). Tudo `pub` e fora de `#[cfg(test)]`.
  - `HttpApiSeams::with_admin` (`state.rs:65-71`), `ApiState::with_agent_registry` (`:162-182`), `ApiState::with_agent_registry_and_verified_owner` (`:184-205`) e `ApiState::with_stores` (`:307`) são `pub`, fora de `#[cfg(test)]`, e aceitam qualquer `HttpAdminAuth`, inclusive a aberta. No HEAD só testes os chamam (conferido com `rg`), mas nada impede um chamador de produção.
  - **TOCTOU:** `ApiState::for_http_server` (`state.rs:207-227`) chama `HttpApiSeams::from_env()` (`:224`), que relê `HttpAdminAuth::from_env()` (`:56-63`, em `:59`). Se a decisão de boot (A3) validar um token e o servidor ler o env de novo depois, o token usado pode não ser o validado.
- **Requisições sem `Host` (F-01-1):** 101 testes em `presentation/http` montam `Request::builder()` sem `Host` (`server.rs` 22, `http_integration_tests.rs` 75, `state.rs` 4; `rg "Request::builder\(\)"`), e `rg 'header\(HOST|header::HOST'` em `backend/src` não acha nada. Em HTTP/2 o cliente manda `:authority`, que o hyper coloca na URI da requisição (`uri().authority()`), sem header `Host` (comportamento do hyper; não verificado neste repositório por teste).
- **OpenAPI sem esquema de segurança (F-01-5):** `rg 'security|bearer|SecurityScheme|modifiers' presentation/http/openapi.rs` só acha a descrição textual da linha 171. Nenhuma rota declara `security`.
- Contradições ainda abertas no código: `admin_auth.rs:1` diz "Optional fail-closed"; `disabled_fail_closed()` monta auth aberta. `cli-and-config.md:138` e o título deste SDD já dizem "ausente ou vazio = nenhuma autenticação".

> **Nota de precedência (2026-09-27):** a decisão do owner acima substitui os textos antigos abaixo que exigem token no startup ou classificam rotas de forma diferente. A lista método+path de cima é a classificação aprovada; a tabela A2.2 é inventário histórico cujo rótulo de classe não prevalece quando divergir dela. A2–A6 e F1–F10 continuam como detalhes de design e validação propostos para revisão, mas não são decisões do owner sobre middleware, tabela central, guard, Host, limitador ou OpenAPI. Os critérios antigos de startup recusado e rota desconhecida 401 estão supersedidos por listener ativo e 503 para auth ausente/fraca, e 404 para rota sem classe. O documento está pronto para revisão independente de G1; G1 não está aprovado.

### A1 — escopo obrigatório e follow-ups

| Classe | IDs de [F-ADM-01](../security/admin-http-auth-fail-open.md) | Tratamento |
|---|---|---|
| Obrigatório em W0-01 | SEC-ADM-01, 02 (forma depende da decisão do opt-out, A3), 03, 04, 05, 06, 07, 08, 09, 10, 11 (forma de A6), 13, 15 | critérios de aceite desta fatia |
| Follow-up explícito | SEC-ADM-12 (rotação com dois tokens) | item próprio no plano; não bloqueia G4 de W0-01 |
| Bloqueado | SEC-ADM-14 | depende de P1 (auth humano/IdP); registrado, não implementado |

**Decisão de runtime (2026-09-27):** auth ausente, vazia ou fraca não impede o boot nem a abertura do socket. Rotas protegidas retornam **503** `admin_auth_not_configured`; com token utilizável configurado, bearer ausente/incorreto retorna **401** `unauthorized` e bearer válido prossegue. Rotas públicas seguem abertas sem token. As classes exatas aprovadas constam acima; a composição técnica do router continua para revisão G1.

**Decisão antiga substituída:** falhar o startup sem token e impedir o listener. A resposta 503 no endpoint protegido é o contrato escolhido pelo owner, não uma alternativa rejeitada.

### A2 — proposta de roteamento e defesa em profundidade (classes aprovadas acima; detalhes para G1)

Evidência: o `Router` do axum (0.8.9 no `Cargo.lock`) não lista as rotas registradas; hoje as rotas são registradas em dois arquivos (`routes/mod.rs` e `server.rs`); o OpenAPI vem de `#[utoipa::path]` agregado em `ApiDoc` (`openapi.rs`), então um teste guiado só pelo OpenAPI não vê rota registrada e esquecida na anotação.

- **Classes de rota:** `Protected` | `PublicRead` | `PublicCompute` | `Org`.
  - `Protected`: exige o bearer admin (`BOT_HTTP_ADMIN_TOKEN`).
  - `PublicRead`: GET sem auth, sem segredo e sem dado de operação (A2.2).
  - `PublicCompute`: POST sem auth da allowlist literal de 6 pares (A2.1).
  - `Org` (**reservada**, pedido do Segurança; SEC-ADM-03 e [SEC-ORG-33](../security/org-module-threat-model.md)): rotas futuras do módulo `org`. Exigem o bearer P1 do owner no header `Authorization`, validado pelo verificador de owner auth, independente de `BOT_HTTP_ADMIN_TOKEN`. Sem verificador P1 configurado → **503** `owner_auth_required`. **Sem fallback em nenhuma direção:** o bearer admin numa rota `Org` não autentica, e o bearer P1 numa rota `Protected` não autentica (ambos → 401 quando o P1 existir). Nenhuma rota `org` existe nesta fatia; a classe e a regra entram agora para que a primeira rota `org` não herde a auth admin. Enquanto P1 não existe, toda rota `Org` responde 503 `owner_auth_required`, com qualquer header.
- **Mapeamento de nomes TM × SDD:** a versão do TM lida pelo Critic (linha 88) usava `Public`, `PublicCompute`, `Admin` e `Owner`. Correspondência: `Public` → `PublicRead`; `PublicCompute` → `PublicCompute`; `Admin` → `Protected`; `Owner` → `Org`. A versão atual do TM (SEC-ADM-03, linha 87) já usa os nomes deste SDD mais `Org`; SEC-ORG-33 (linha 187 do TM de `org`) ainda escreve `Owner`/`Org`, para o Segurança uniformizar.
- **Regra de classificação (decisão do Segurança e do Arquiteto):** a classe vem do **tipo de resposta**. O mesmo tipo de payload de sucesso não pode aparecer em classes diferentes. Se um tipo carrega dado que exige auth numa rota, exige em todas. Critério verificável em A2.1 item 6 e F3 (j).
- **Tabela única:** um módulo de tabela em `presentation/http/routes` (nome proposto `routes/table.rs`) declara cada par método+path com handler e classe. Esse módulo é o **único** lugar do `src/` de produção que registra rota; ele monta também `/healthz`, `/readyz`, `/openapi.json` e `/docs`.
- **Montagem em duas funções, sem seam de teste dentro de `build_router`:** `apply_layers(router: Router<ApiState>, state) -> Router` aplica, nesta ordem de execução, a layer de `Host` (A5), a layer de auth e o `TraceLayer`, e chama `with_state`. Ela não registra rota. `build_router(state)` = `apply_layers(table::router(), state)`. O teste F3 (e) monta `table::router().route("/extra", …)` num módulo de teste e passa a `apply_layers`; com isso `build_router` não tem nenhum `#[cfg(test)]` e o guard estático (F4) não precisa de exceção para ele.
- **Auth no router inteiro:** a layer de auth é aplicada com `Router::layer` dentro de `apply_layers`, cobrindo todas as rotas (inclusive as que alguém registrar fora da tabela) e o fallback. A layer lê `MatchedPath` e o método. **HEAD é normalizado para GET** antes do lookup, porque o `get()` do axum também atende HEAD; sem isso, `HEAD /healthz` cairia em "método não listado" e receberia 401. Com o par encontrado: `PublicRead`/`PublicCompute` passam sem credencial; `Protected` exige bearer admin válido; `Org` segue a regra acima. Qualquer outro caso exige bearer admin válido, senão **401** `unauthorized`: par ausente da tabela, método não listado para um path conhecido e requisição sem `MatchedPath` (rota inexistente). Rota inexistente responde 401 sem credencial, não 404; com credencial, segue para o fallback do router, que responde 404 com código próprio `route_not_found`. Par com `MatchedPath` mas fora da tabela é negado e gera evento `warn` `route_not_in_table` (sem token no log).
- **Handlers:** as chamadas manuais a `require_http_admin` continuam nesta fatia (defesa em profundidade); remover fica para depois de F3 verde.
- **Guard estático (F4):** `verify-backend-gates.sh` reprova, em `backend/src`, qualquer uma destas chamadas fora do módulo da tabela: `.route(`, `.nest(`, `.merge(`, `.route_service(`, `.nest_service(`, `.fallback(`, `.fallback_service(`. Ficam isentos: (a) o módulo da tabela; (b) arquivos `*_tests.rs`; (c) blocos `#[cfg(test)] mod <nome> {` inline, como `server.rs:96-97` e `state.rs:98-99`. **Como (c) é detectado (o mais simples que é determinístico):** o guard pula do `#[cfg(test)]` seguido de linha `mod <nome> {` até a linha `}` com a mesma indentação do `mod`. Isso é determinístico porque o `cargo fmt --check` do próprio gate (`verify-backend-gates.sh:4`) garante essa indentação. No HEAD, `rg` acha essas chamadas só em `routes/mod.rs:27-98` e `server.rs:85-91`.
- **Alternativa (rejeitada):** `route_layer` só no sub-router `Protected` de `v1_routes()`. Deixa aberto tudo o que está fora de `v1_routes()`; é allow-by-default.
- **Alternativa (rejeitada):** OpenAPI como fonte. Não pega rota sem anotação e o axum não oferece paridade router × OpenAPI.

#### A2.1 — invariantes da tabela (SEC-ADM-03)

1. Todo método diferente de GET é `Protected`, exceto a allowlist **literal** de 6 pares `PublicCompute` abaixo. A allowlist é uma constante no módulo da tabela, e o teste compara a tabela com essa constante.
2. Todo `PublicRead` é GET.
3. Todo path `/api/v1/admin/*` é `Protected`, em qualquer método, e todo GET da lista mínima do SEC-ADM-05 (`GET /agents*`, `GET /orders/reconciliation/{id}`, `/config/snapshot`, `/orders/execution-status`) mais `/meta`, `/config/active` e `/monitor/snapshot` é `Protected`. O teste confere essa lista contra a tabela.
4. Cada `PublicCompute` tem teste próprio de ausência de efeito colateral: com `ApiState` montado com stores e executor contáveis, a chamada não grava em PG nem em memória compartilhada (registry de agentes, catálogo, ledgers de ordens/reconciliação, runtime de bots), não chama executor de ordens nem rede, e o estado observável antes e depois é igual.
5. Nesta fatia a tabela não tem entrada `Org`, e o teste de invariantes afirma isso (lista `Org` vazia). A primeira rota `org` obriga a mudar esse teste junto com o verificador P1. A resposta 503 da classe `Org` é testada pela função pura de decisão da layer (F3 (k)).
6. **Um tipo de resposta, uma classe (checagem escolhida: teste sobre a tabela × `ApiDoc::openapi()`, o jeito mais simples):** para cada entrada da tabela, o teste pega os nomes de schema `$ref` das respostas 2xx da operação correspondente no OpenAPI (a mesma junção que F3 (d) já faz) e monta o mapa `schema → conjunto de classes`. Falha se algum schema aparecer em mais de uma classe. Só o tipo de primeiro nível conta. Operações sem `$ref` de sucesso ficam numa lista literal no teste: `/openapi.json`, `/docs` (fora do OpenAPI), `POST /backtest/sma-crossover` e `POST /bots/ranking` (anotação `serde_json::Value`), `DELETE /admin/provider-credentials/{provider_id}/{key_name}` e `POST /bots/runtime/demote` (204 sem corpo). Entrada nova sem `$ref` fora dessa lista faz o teste falhar. **Limites:** a checagem confia na anotação `#[utoipa::path]`, não no tipo real do handler (ex.: `/bots/ranking` anota `serde_json::Value` em `routes/bots.rs:33` e devolve `BotRankingResponse` em `:40`); e não olha tipos aninhados (ver "Ponto aberto" em A2.2).

#### A2.2 — inventário completo (HEAD `d42b71a5`)

Conferido no código. Refs: `mod.rs` = `routes/mod.rs`; `pca.rs` = `routes/provider_credentials_admin.rs`; `ga.rs` = `routes/graph_admin.rs`. Paths de v1 com prefixo `/api/v1`. "Tipo 2xx" é o `body` da resposta de sucesso no `#[utoipa::path]` do handler; "—" = sem `$ref` (ver A2.1 item 6).

| # | Método | Path | Classe | Tipo 2xx | Registro | `require_http_admin` hoje |
|---|---|---|---|---|---|---|
| 1 | GET | `/healthz` | PublicRead | `HealthResponse` | `mod.rs:97` | não |
| 2 | GET | `/readyz` | PublicRead | `ReadyResponse` | `mod.rs:98` | não |
| 3 | GET | `/openapi.json` | PublicRead | — | `server.rs:87-90` | não |
| 4 | GET | `/docs` | PublicRead | — | `server.rs:91` (Scalar) | não |
| 5 | GET | `/meta` | **Protected** | `MetaResponse` | `mod.rs:27` | não |
| 6 | GET | `/application/signals` | PublicRead | `SignalsResponse` | `mod.rs:28` | não |
| 7 | GET | `/config/active` | **Protected** | `ConfigSnapshotResponse` | `mod.rs:29` | não |
| 8 | GET | `/config/snapshot` | **Protected** | `ConfigSnapshotResponse` | `mod.rs:30` | não |
| 9 | GET | `/providers/status` | PublicRead | `ProvidersStatusResponse` | `mod.rs:31` | não |
| 10 | GET | `/admin/provider-credentials` | **Protected** | `ProviderCredentialsListResponse` | `mod.rs:32-36` | sim, `pca.rs:42` |
| 11 | POST | `/admin/provider-credentials` | Protected | `ProviderCredentialMaskedBody` | `mod.rs:32-36` | sim, `pca.rs:67` |
| 12 | PUT | `/admin/provider-credentials/{provider_id}/{key_name}` | Protected | `ProviderCredentialMaskedBody` | `mod.rs:37-41` | sim, `pca.rs:93` |
| 13 | DELETE | `/admin/provider-credentials/{provider_id}/{key_name}` | Protected | — (204) | `mod.rs:37-41` | sim, `pca.rs:117` |
| 14 | GET | `/admin/graph/agents` | **Protected** | `ProjectedAgentList` | `mod.rs:42` | sim, `ga.rs:60` |
| 15 | GET | `/admin/graph/supervision-chain` | **Protected** | `ProjectedSupervisionChain` | `mod.rs:43-46` | sim, `ga.rs:82` |
| 16 | GET | `/admin/graph/bots-for-agent` | **Protected** | `ProjectedBotsForAgent` | `mod.rs:47-50` | sim, `ga.rs:106` |
| 17 | GET | `/admin/graph/code-impact` | **Protected** | `ProjectedCodeImpactForModule` | `mod.rs:51-54` | sim, `ga.rs:131` |
| 18 | GET | `/exchanges/catalog` | PublicRead | `ExchangeCatalogResponse` | `mod.rs:55` | não |
| 19 | GET | `/exchanges/routing` | PublicRead | `ExchangeRoutingResponse` | `mod.rs:56` | não |
| 20 | GET | `/agents/audit` | **Protected** | `AuditLogResponse` | `mod.rs:57` | não |
| 21 | GET | `/agents` | **Protected** | `AgentListResponse` | `mod.rs:58-61` | não |
| 22 | POST | `/agents` | Protected | `AgentResponse` | `mod.rs:58-61` | sim, `agents.rs:43` |
| 23 | POST | `/agents/{agent_id}/pause` | Protected | `LifecycleResponse` | `mod.rs:62` | sim, `agents.rs:100` |
| 24 | POST | `/agents/{agent_id}/resume` | Protected | `LifecycleResponse` | `mod.rs:63` | sim, `agents.rs:121` |
| 25 | POST | `/agents/{agent_id}/retire` | Protected | `LifecycleResponse` | `mod.rs:64` | sim, `agents.rs:142` |
| 26 | POST | `/agents/{agent_id}/advisory` | Protected | `AdvisoryResponse` | `mod.rs:65` | sim, `agents.rs:166` |
| 27 | GET | `/agents/{agent_id}` | **Protected** | `AgentResponse` | `mod.rs:66` | não |
| 28 | POST | `/risk/profile-limits` | PublicCompute | `ProfileLimitsResponse` | `mod.rs:67` | não |
| 29 | POST | `/risk/validate-intent` | PublicCompute | `ValidateIntentResponse` | `mod.rs:68` | não |
| 30 | POST | `/risk/gate-signal` | PublicCompute | `GateSignalResponse` | `mod.rs:69` | não |
| 31 | GET | `/strategy/periods` | PublicRead | `StrategyPeriodsResponse` | `mod.rs:70` | não |
| 32 | POST | `/strategy/evaluate-sma` | PublicCompute | `EvaluateSmaResponse` | `mod.rs:71` | não |
| 33 | GET | `/portfolio/paper-snapshot` | PublicRead | `PaperSnapshotResponse` | `mod.rs:72` | não |
| 34 | GET | `/bots/catalog` | PublicRead | `BotCatalogResponse` | `mod.rs:73` | não |
| 35 | POST | `/bots/catalog/persist` | Protected | `BotCatalogPersistResponse` | `mod.rs:74` | sim, `bots.rs:60` |
| 36 | GET | `/bots/catalog/snapshot` | PublicRead | `BotCatalogResponse` | `mod.rs:75` | não |
| 37 | POST | `/bots/ranking` | PublicCompute | — (`serde_json::Value`) | `mod.rs:76` | não |
| 38 | GET | `/bots/runtime/status` | PublicRead | `BotRuntimeStatus` | `mod.rs:77` | não |
| 39 | POST | `/bots/runtime/promote` | Protected | `BotPromotionRecord` | `mod.rs:78` | sim, `bots.rs:117` |
| 40 | POST | `/bots/runtime/demote` | Protected | — (204) | `mod.rs:79` | sim, `bots.rs:135` |
| 41 | POST | `/backtest/sma-crossover` | PublicCompute | — (`serde_json::Value`) | `mod.rs:80` | não |
| 42 | GET | `/orders/execution-status` | **Protected** | `OrderExecutionStatusResponse` | `mod.rs:81` | não |
| 43 | GET | `/orders/reconciliation/{client_order_id}` | **Protected** | `OrderReconciliationResponse` | `mod.rs:82-85` | não |
| 44 | POST | `/orders/reconciliation/poll` | Protected | `OrderReconciliationPollResponse` | `mod.rs:86-89` | sim, `orders.rs:73` |
| 45 | POST | `/orders/submit` | Protected | `SubmitOrderResponse` | `mod.rs:90` | sim, `orders.rs:99` |
| 46 | GET | `/monitor/snapshot` | **Protected** | `MonitorSnapshotResponse` | `mod.rs:91` | não |
| 47 | POST | `/monitor/commands` | Protected | `MonitorCommandResponse` | `mod.rs:92` | sim, `monitor.rs:38` |

**Totais:** 27 GET (4 fora de v1 + 23 em v1): **14 `Protected`** e **13 `PublicRead`**. 20 não-GET: 14 `Protected` + 6 `PublicCompute`. Total 47 pares; 28 `Protected`. São 19 chamadas a `require_http_admin` hoje: 14 nas mutações e 5 nos GET `/admin/*`.

**Como a contagem foi verificada (no HEAD `d42b71a5`):** `rg -o 'get\(|\.post\(|post\(|put\(|\.delete\(' routes/mod.rs | sort | uniq -c` → 25 `get(` (23 em `v1_routes` + 2 em `system_routes`), 18 POST (16 `post(` + 2 `.post(`), 1 `put(`, 1 `.delete(`; mais `/openapi.json` e `/docs` em `server.rs:87-91` → 27 GET e 20 não-GET. `rg -c 'require_http_admin\(' routes/` → 19. A classe de cada linha foi conferida uma a uma contra a tabela acima; a verificação automática da tabela contra o router é F3 (g).

- **SEC-ADM-05 (F1):** `Protected` por ele: `/admin/*` (#10, 14-17), `GET /agents*` (#20, 21, 27), `/orders/reconciliation/{client_order_id}` (#43), `/config/snapshot` (#8), `/orders/execution-status` (#42).
- **Além do mínimo, `Protected` por decisão deste SDD:**
  - `GET /meta` (#5): `http_seams` expõe `order_execution_mode` e `live_exchange_wired` (`routes/meta.rs:9-20`, preenchidos em `:44` e `:50`), a mesma informação que torna `/orders/execution-status` sensível.
  - `GET /monitor/snapshot` (#46) e `GET /config/active` (#7): **decisão do Segurança e do Arquiteto** (confirmada neste ciclo), com dois motivos:
    1. `MonitorSnapshotResponse` (`http_bridge/monitor.rs:7-26`) tem `last_error` (`:19`), texto livre que pode carregar mensagens de exchange, URLs e fragmentos de config; e `promoted_bot_id`/`promoted_by` (`:23-25`) expõem quem opera.
    2. `/config/active` devolve o mesmo `ConfigSnapshotResponse` (`http_bridge/config.rs:17-31`) que o `Protected` `/config/snapshot` (`routes/config.rs:16-24` e `:26-34`). O que torna o `/config/snapshot` sensível é carregar config de um caminho enviado pelo cliente (`http_bridge/config.rs:33-45`, `ConfigSnapshotQuery.config` → `Config::load`); com o mesmo payload, `/config/active` aberto seria um bypass do `/config/snapshot`. Daí a regra "um tipo de resposta, uma classe" (A2.1 item 6).
- **Continuam `PublicRead` (13):** `/healthz`, `/readyz`, `/openapi.json`, `/docs`, `/application/signals`, `/providers/status`, `/exchanges/catalog`, `/exchanges/routing`, `/strategy/periods`, `/portfolio/paper-snapshot`, `/bots/catalog`, `/bots/catalog/snapshot`, `/bots/runtime/status`.
- **Ponto aberto para o Segurança e o Arquiteto (não muda a contagem acima):** `GET /bots/runtime/status` (#38) devolve `BotRuntimeStatus`, cujo campo `active` é um `BotPromotionRecord` com `bot_id` e `promoted_by` (`modules/bots/models/runtime.rs:14-25`). É o mesmo motivo 1 usado para `/monitor/snapshot`. `BotPromotionRecord` também é o tipo 2xx do `Protected` `POST /bots/runtime/promote` (#39). A checagem de A2.1 item 6 olha só o primeiro nível, então não pega esse caso. Se o Segurança decidir que `/bots/runtime/status` também é `Protected`, os totais passam a 12 `PublicRead` e 15 GET `Protected`, e a checagem pode passar a incluir tipos aninhados.

#### A2.3 — prova de completude tabela × router (SEC-ADM-03 (a))

O axum não enumera as rotas de um `Router`, então a prova é feita pelos dois lados:

1. **Router ⊆ tabela, por construção + guard.** O módulo da tabela monta o router num único laço sobre a lista de entradas; `/openapi.json` e `/docs` também são entradas da tabela. O guard estático (F4) proíbe as sete formas de registro em qualquer outro arquivo de produção. Se alguém registrar mesmo assim (ex.: em teste), a layer nega e emite `route_not_in_table`; F3 (e) prova isso.
2. **Tabela ⊆ router, por teste, sem efeito colateral.** Um teste percorre a tabela e, para cada par, envia a requisição com token de fixture válido e `Host: localhost`; falha se a resposta for **405** ou **404 com código `route_not_found`** (vindo do fallback). 404 de handler tem outro código e conta como "rota existe". **Por que não há efeito:** o probe roda sobre um `ApiState` de teste descartável (sem PG, sem monitor, executor de ordens e runtime de bots fail-closed, stores em memória próprios do teste) e manda corpo **vazio sem `Content-Type`**. Nas rotas com extractor `Json<…>` (ex.: `POST /monitor/commands`, `POST /orders/submit`), o extractor rejeita antes do corpo do handler (4xx do axum, que conta como "rota existe"). Nas rotas sem corpo, o handler roda, mas não há efeito fora do `ApiState` descartável: `DELETE` de credencial (#13) → 503 por falta de PG; `pause`/`resume`/`retire` (#23-25) → 404 com `agent_id` de fixture inexistente; `poll` (#44) → retorno antecipado porque `live_exchange_wired` é falso (`state.rs:734-736`); `demote` (#40) → 503 com runtime fail-closed; `bots/catalog/persist` (#35) → grava só no catálogo em memória do `ApiState` do teste. A layer **não** rejeita antes nesses casos (o token é válido), então a garantia vem do estado descartável e do corpo vazio, não da layer.
3. **Tabela × OpenAPI:** o teste de paridade (F3 (d)) confere que todo par da tabela, exceto `/openapi.json` e `/docs`, está em `ApiDoc::openapi()` e vice-versa, **e** que a classe bate com o `security` declarado (A2.4).

#### A2.4 — contrato: `/meta` e GETs que passam a `Protected` (F-01-5)

- **Consumidores em código:** não há frontend nem script no repositório que chame a API (a raiz tem só `backend/`, `docker-compose.bot.yml`, `AGENTS.md` e `opencode.json`); os consumidores em código são só os testes. A quebra de contrato atinge clientes externos que leem `/meta`, `/config/active`, `/monitor/snapshot` e os GETs do SEC-ADM-05 sem token.
- **Docs que descrevem `/meta` como aberto (a migrar pelo Guardião de docs depois do merge; esta fatia não as edita):** `backend/docs/reference/cli-and-config.md:61` e `:137`, `backend/docs/architecture/integrations.md:23`, `backend/docs/sdd/agents-module-sdd.md:132`, `backend/docs/sdd/agents-owner-bootstrap-g1-sdd.md:20` e a seção "Observabilidade" deste SDD (linha 322). Também citam `/config/active` sem falar de auth: `cli-and-config.md:117` e `backend/docs/reference/test-matrix.md:36`.
- **OpenAPI com segurança declarada por rota:** `ApiDoc` ganha um `Modify` que registra `components.securitySchemes.admin_bearer` (`type: http`, `scheme: bearer`), e cada `#[utoipa::path]` de rota `Protected` declara `security(("admin_bearer" = []))`. Rotas `PublicRead`/`PublicCompute` não declaram `security`. Rotas `Org` futuras declaram um esquema próprio (`owner_bearer`), nunca `admin_bearer`. Assim a quebra fica visível em `/openapi.json`.
- **Paridade de classe no teste:** F3 (d) passa a comparar, além de método+path, a classe da tabela com o `security` da operação: `Protected` ⇔ `admin_bearer`; `PublicRead`/`PublicCompute` ⇔ sem `security`.

### A3 — proposta antiga de boot obrigatório (supersedida; não normativa)

- **Onde:** a decisão "pode subir?" roda no braço `Some(BotCommand::Serve(args))` de `main.rs`, logo depois de `Config::load` (`main.rs:104`), antes de `--with-monitor` (`:106-136`) e de `run_server` (`:137`). Assim nenhuma conexão a PG/Neo4j, migração, outbox, hidratação ou monitor acontece sem auth válida.
- **Forma:** função pura em `core/config` (onde o token já é lido, `core/config/http/file.rs`); nenhuma leitura de env em `presentation/`. Entrada: config de auth admin (token, bindings), `allowed_hosts` (A5) e `bind`. Saída: `Enforced` ou erro estável (`admin_auth_not_configured`, `admin_auth_token_weak`, `http_allowed_hosts_required`) sem ecoar o valor.
- **`Enforced` fecha a auth aberta (F-01-2):**
  - O token passa a ser **não opcional** dentro do tipo validado. `HttpAdminAuth` perde o `#[derive(Default)]` (`admin_auth.rs:7`) e o `Debug` derivado (vira manual, com `<redacted>`); `disabled()` (`:15-17`) e o ramo `None` de `verify_headers` (`:92-93`) são removidos. `HttpAdminAuth` só é construído a partir do `Enforced`.
  - O `Enforced` desce por parâmetro: `main.rs` → `run_server` → `server::run` → `ApiState::build_api_state_for_http_serve` → `ApiState::for_http_server` → `HttpApiSeams::for_serve(enforced)`. `HttpApiSeams::from_env()` (`state.rs:56-63`) deixa de existir em produção: `for_serve` monta o executor de ordens e o runtime de bots como hoje (`HttpOrderExecutor::from_env()`, `shared_bot_runtime()`), mas **não relê** `BOT_HTTP_ADMIN_TOKEN`. O env de auth é lido uma vez, no boot (fecha o TOCTOU).
  - Viram `#[cfg(test)]`: `HttpApiSeams::disabled_fail_closed` (`state.rs:47-54`, renomeado para `for_tests_open_admin`), `HttpApiSeams::with_admin` (`:65-71`), `HttpApiSeams::from_env` (`:56-63`, se algum teste ainda precisar), `ApiState::new` (`:142-160`), `impl Default for ApiState` (`:1175-1184`), `ApiState::with_agent_registry` (`:162-182`) e `ApiState::with_agent_registry_and_verified_owner` (`:184-205`). `ApiState::with_stores` (`:307`) deixa de ser `pub` (fica privado, usado por `for_http_server`) e ganha um wrapper `#[cfg(test)]` para os testes que o chamam hoje. Em produção só existe o caminho `for_http_server(…, enforced)`, que não aceita auth aberta porque o tipo não a representa.
  - F6 verifica esses nomes; F10 verifica o TOCTOU.
- **Opt-out de dev: decisão do owner, com recomendação. Não bloqueia G1; bloqueia G3** (o primeiro teste de boot depende de a flag existir ou não).
  - **Recomendado (mais simples): remover o opt-out.** Sem token válido, `serve` nunca sobe, em qualquer bind. O token de dev já é barato (A4). Loopback não protege contra proxy reverso local, túnel (ngrok, `ssh -R`) nem DNS rebinding. Com esta opção, o `/meta` não ganha campo novo: `http_admin_auth_enabled` continua e passa a ser sempre `true` num `serve` no ar.
  - **Alternativa registrada: manter `DevDisabled`** (SEC-ADM-02 do TM), com flag `BOT_HTTP_ADMIN_AUTH_DISABLED_DEV` (`1`/`true`, lida em `core/config/http/file.rs`) e bind loopback; a allowlist de `Host` (A5) vale em todos os modos. Acrescenta o erro `admin_auth_dev_optout_requires_loopback`, log `WARN` no boot, `GET /meta` → `http_seams.http_admin_auth_mode = "dev_disabled" | "enforced"`, e a heurística anti-túnel de F11. Nesta alternativa **não há token**, então um túnel ou proxy local que reescreve `Host` para `localhost` expõe o admin sem nenhuma barreira (ver A5).
  - Se o owner aceitar a recomendação, SEC-ADM-02 é atendido na forma "não existe opt-out" (teste: nenhuma flag faz o `serve` subir sem token).
- **Guard de env:** o regex de `verify-backend-gates.sh:8` (`env::var\(|std::env::var\(`) não pega `env::var_os`, `std::env::vars` nem `dotenvy::var`. O aceite amplia o guard para `(std::)?env::var(s|_os)?\(` e `dotenvy::vars?\(`, com as mesmas exceções de caminho. No HEAD não há ocorrência dessas formas fora de `core/config/` (conferido com `rg`).

### A4 — geração de token de desenvolvimento (proposta; sem falha de startup)

- O desenvolvedor gera o token localmente: `openssl rand -base64 32` (32 bytes de entropia, 44 caracteres; SEC-ADM-15 e SEC-ADM-07) e grava em `backend/.env` (ignorado pelo git em `backend/.gitignore:2`). O procedimento vai para [cli-and-config](../reference/cli-and-config.md) e para o runbook de dev; `.env.example` continua com valor vazio (AGENTS.md proíbe placeholder de credencial). Copiar o exemplo sem gerar token faz o `serve` recusar subir, com mensagem que aponta o comando.
- A mesma regra de força (SEC-ADM-07) vale sempre; não há "token fraco permitido em dev".
- Testes usam token de fixture forte gerado no próprio teste. Hoje há 39 ocorrências de `HttpAdminAuth::disabled()` em `backend/src`: 38 em testes (`state.rs` 20, `http_integration_tests.rs` 15, `admin_auth.rs` 2, `register_owner.rs` 1) e 1 em produção (`state.rs:50`, dentro de `disabled_fail_closed`). Como `disabled()` sai (A3), os 38 usos de teste passam a usar `HttpAdminAuth::for_test(token)` ou `for_tests_open_admin()` só onde o teste precisa de auth aberta; `for_test_bound_agency` (`admin_auth.rs:66-73`, hoje sem token) passa a receber token.
- **Alternativa:** o `serve` gerar um token efêmero no boot e imprimi-lo uma vez. Rejeitada: imprime segredo em terminal/log (conflita com SEC-ADM-08).

### A5 — allowlist de `Host` (SEC-ADM-10)

- **Posição:** primeira layer do router inteiro (antes da auth). Recusa com **421** `host_not_allowed`, antes de qualquer handler e antes da checagem de token.
- **Host efetivo (F-01-1, igual ao SEC-ADM-10 do TM):** o header `Host`; se ausente, a authority da URI (`uri().authority()`, que é onde o hyper põe o `:authority` do HTTP/2). **421 somente quando:** (a) `Host` e authority estão ausentes; (b) os dois estão presentes e divergem depois de normalizados; (c) o valor normalizado não está na allowlist.
- **Normalização:** minúsculas; remove um ponto final (`localhost.` → `localhost`); separa host e porta, com IPv6 entre colchetes (`[::1]`, `[::1]:8080`); porta tratada explicitamente (abaixo).
- **Allowlist e portas:**
  - Padrão: `localhost`, `127.0.0.1` e `[::1]`, aceitos sem porta, com a porta padrão do esquema (80) ou com a porta do próprio `bind`.
  - Extras vêm da **chave nova `allowed_hosts` numa tabela nova `[http]`** do `system.toml` (hoje só existe `[http.admin]`, `system.toml:38-39`), com override por env `BOT_HTTP_ALLOWED_HOSTS` (lista separada por vírgula) pelos helpers de `core/config/load.rs`. Cada item é `host` (aceita sem porta ou porta 80) ou `host:porta` (aceita só aquela porta).
  - Porta fora dessas regras → fora da allowlist → 421.
- **Boot:** bind não-loopback (ex.: `0.0.0.0`) com `allowed_hosts` vazio → boot falha com `http_allowed_hosts_required`, porque nenhum cliente remoto passaria.
- **Efeito operacional (a documentar em `cli-and-config` e no runbook):**
  - **Docker com porta publicada:** o container precisa de bind `0.0.0.0`, então `allowed_hosts` passa a ser obrigatório. Com `-p 9000:8080`, o cliente no host manda `Host: localhost:9000`, que não é a porta do bind: o operador precisa listar `localhost:9000` (e o nome/porta pelos quais a API é acessada). Healthcheck de dentro do container (`curl localhost:8080`) passa sem configuração.
  - **Probes por IP do pod (Kubernetes):** o kubelet manda `Host: <ip-do-pod>:<porta>`, e o IP muda a cada pod. O operador configura o probe com header `Host: localhost` (`httpGet.httpHeaders`) em vez de listar IPs.
  - Cliente que acessa por IP da LAN ou nome próprio recebe 421 até o host entrar em `allowed_hosts`.
- **Testes sem `Host`:** os 101 `Request::builder()` de teste (Contexto) passariam a receber 421. O aceite cria um helper de teste (nome proposto `test_request(method, uri)`) que já põe `Host: localhost`, e os testes existentes passam a usá-lo (mudança mecânica).
- CORS: o código não tem layer de CORS (conferido com `rg`), então não há `Access-Control-Allow-Origin: *`; o aceite fixa isso num teste.
- **O que protege:** DNS rebinding pelo navegador. **O que não protege:** proxy ou túnel local que reescreve `Host` para `localhost`. Esse caso só é coberto pelo token obrigatório **se o opt-out de dev for removido** (recomendação de A3). Na alternativa `DevDisabled` não há token e o túnel expõe o admin sem barreira; por isso F11 exige a heurística e a proibição documentada, e registra que ela não detecta tudo.

### A6 — limitador de falhas de autenticação (SEC-ADM-11, forma revisada)

- **Propósito:** reduzir ruído e dar observabilidade a tentativas com token errado. **Não** é defesa contra força bruta: com 32 bytes de entropia (SEC-ADM-07) a força bruta já é inviável sem limitador.
- **Problema do texto do TM com chave por IP:** em loopback todo cliente é `127.0.0.1`; um balde por IP vira balde global e, se bloqueasse qualquer requisição, um processo local travaria o admin legítimo.
- **Decisão:** um contador **global** em janela fixa, que conta **só** 401 da layer em rotas **`Protected` com `MatchedPath` encontrado na tabela**. Rota inexistente (sem `MatchedPath`) e par fora da tabela também recebem 401, mas **não** incrementam o contador, para que varredura de paths não infle o limitador. Acima de N falhas na janela, as respostas de falha passam a **429** com header `Retry-After` (segundos até o fim da janela). O servidor **nunca** dorme nem atrasa a resposta. Uma requisição com token válido (comparação em tempo constante, SEC-ADM-06) nunca é bloqueada nem atrasada: a layer compara o token primeiro e só consulta o limitador quando a comparação falha. Rotas `PublicRead`/`PublicCompute` não passam pelo limitador. Log de 429 agregado por janela (um evento com o contador), sem token. Sem chave por IP e sem `X-Forwarded-For`/`trusted_proxies` nesta fatia.
- **N por janela:** decisão do owner **D-SEC-ADM-RATE** (TM); proposta padrão 20 falhas por minuto, configurável em `core/config`; não bloqueia G1.

### Seams públicos — contratos de comportamento fechados pelo owner

A decisão do owner fecha os seis seams públicos de comportamento listados no início desta seção: configuração token ausente/fraco, verificação Bearer, classificação da lista existente, comportamento para rota não classificada, respostas HTTP e ausência de token nas rotas públicas. O design interno do middleware/router, Host, rate limit e anotação OpenAPI permanece recomendação técnica sujeita à revisão G1; não altera estes contratos.

| Seam | Contrato aprovado / proposta interna para revisão |
|---|---|
| Configuração e opt-out | Não há opt-out para tornar protegidas públicas. Token ausente, vazio, fraco (menos de 32 bytes aleatórios) ou com espaços significa auth indisponível; `serve` continua ativo. |
| Respostas da auth | 503 `admin_auth_not_configured` para auth indisponível; 401 `unauthorized` para bearer ausente/incorreto com token configurado; sem ecoar token. |
| Ponto de leitura | Proposta: ler/validar env uma vez ao construir estado de auth para evitar TOCTOU; nunca falhar o startup por ausência/token fraco. |
| Classes de rota | Lista método+path aprovada na seção “Classificação aprovada das rotas existentes”: mutações e leituras sensíveis protegidas; health/docs/leitura e seis cálculos públicos. |
| Rota não classificada | 404 `route_not_found` antes de executar handler; sem herança pública ou auth por omissão. |
| Composição router | Proposta de G1: camada de classificação aplicada antes dos handlers, incluindo endpoints de sistema/docs; nome de funções e estrutura do registro ficam para Critic revisar. |

### Critérios de aceite W0-01 (pronto para revisão G1; critérios observáveis, não aprovados)

- SEC-ADM-01, 02, 03, 04, 05, 06, 07, 08, 09, 10, 11 (forma de A6), 13 e 15, conforme [F-ADM-01](../security/admin-http-auth-fail-open.md) §4, referenciados por ID.
- F1. Teste CLI em `backend/tests/` (padrão de `config_cli.rs`): `serve` sem token → exit ≠ 0, porta não aberta e **zero conexões ao DB**: `DATABASE_URL` aponta para `127.0.0.1:<porta>/trading_bot` de um `TcpListener` do próprio teste, que conta `accept`s; esperado 0. Entra na lista `INTEGRATION_TESTS` de `verify-backend-gates.sh`. Mesmo teste com token fraco.
- F2. Função de decisão de boot (A3) com teste para cada combinação de token (ausente, vazio, fraco, forte), bind (loopback, não-loopback) e `allowed_hosts` (vazio, preenchido); na alternativa, também a flag.
- F3. Testes guiados pela tabela (A2):
  - (a) toda entrada `Protected` responde 401 sem header e com token errado;
  - (b) invariantes A2.1 itens 1-3 e 5 verificados sobre a tabela;
  - (c) um teste de ausência de efeito colateral por `PublicCompute` (A2.1 item 4);
  - (d) paridade tabela × `ApiDoc::openapi()`: método+path **e** classe × `security` (`Protected` ⇔ `admin_bearer`; públicas sem `security`), e `components.securitySchemes.admin_bearer` = http bearer;
  - (e) router de teste montado com `table::router().route("/extra", …)` e passado a `apply_layers` responde 401 sem credencial e emite `route_not_in_table`;
  - (f) path inexistente sem credencial → 401;
  - (g) completude tabela ⊆ router (A2.3 item 2): nenhuma entrada responde 405 ou 404 `route_not_found` com token válido, com o `ApiState` descartável e o corpo vazio de A2.3;
  - (h) GETs do SEC-ADM-05, `/meta`, `/config/active` e `/monitor/snapshot` → 401 sem credencial;
  - (i) `HEAD /healthz` → 200 sem credencial; `HEAD /api/v1/meta` → 401 sem credencial;
  - (j) regra "um tipo de resposta, uma classe" (A2.1 item 6): falha se um schema 2xx aparecer em duas classes. Prova de RED registrada na entrega: com `/config/active` marcado `PublicRead`, o teste falha por `ConfigSnapshotResponse` em `PublicRead` e `Protected`;
  - (k) classe `Org` pela função pura de decisão da layer: sem verificador P1 → 503 `owner_auth_required`, inclusive com bearer admin válido (sem fallback).
- F4. Guard de `verify-backend-gates.sh` contra `.route(`, `.nest(`, `.merge(`, `.route_service(`, `.nest_service(`, `.fallback(` e `.fallback_service(` fora do módulo da tabela, com as isenções de A2 (arquivos `*_tests.rs` e blocos `#[cfg(test)] mod` inline). Casos registrados na entrega: arquivo temporário de produção com `.fallback(` dispara o guard; o mesmo código dentro de um `#[cfg(test)] mod tests {` inline não dispara.
- F5. Guard de env ampliado (A3) com caso negativo para `env::var_os` e `dotenvy::var`.
- F6. SEC-ADM-13 e F-01-2: o scanner do guard F4 (mesma regra de isenção de `#[cfg(test)]`, mais itens com `#[cfg(test)]` na linha anterior) não acha em código de produção nenhum destes nomes: `disabled_fail_closed`, `for_tests_open_admin`, `HttpAdminAuth::disabled`, `HttpApiSeams::with_admin`, `HttpApiSeams::from_env`, `ApiState::new`, `impl Default for ApiState`, `with_agent_registry`, `with_agent_registry_and_verified_owner`, e `pub fn with_stores`. `admin_auth.rs` não deriva `Default` para `HttpAdminAuth` e `verify_headers` não tem ramo sem token. `rg "Optional fail-closed" backend/src` vazio.
- F7. Suite HTTP existente verde com token de fixture e o helper `test_request`.
- F8 (SEC-ADM-10). Com handler de teste que registra chamada:
  - `Host: evil.example` → **421** `host_not_allowed` numa rota `Protected` e numa `PublicRead`; o handler não é chamado e a checagem de token não roda;
  - sem `Host` e sem authority → 421;
  - sem `Host` e URI absoluta `http://localhost:<porta-do-bind>/healthz` (forma do HTTP/2) → passa;
  - `Host` e authority divergentes → 421;
  - `Host: LOCALHOST.`, `localhost:<porta-do-bind>` e `[::1]:<porta-do-bind>` → passam;
  - `localhost:<outra-porta>` não listada → 421; com `localhost:<outra-porta>` em `allowed_hosts` → passa;
  - resposta sem `Access-Control-Allow-Origin`;
  - boot com bind não-loopback sem `allowed_hosts` → exit ≠ 0.
- F9 (SEC-ADM-11, A6). Com N de teste pequeno: N+1 requisições com token errado numa rota `Protected` → a última é **429** com `Retry-After`; requisições com token errado a paths inexistentes não mudam o contador; em seguida, token válido → **200**; rotas `PublicRead` não mudam de status durante o bloqueio; nenhum log contém o token; o Critic confere que não há `sleep` no caminho da layer.
- F10 (TOCTOU, F-01-2). Boot com token A no env produz `Enforced(A)`; o teste troca o env para B e monta o estado por `for_http_server(…, Enforced(A))`: requisição com B → 401, com A → 200.
- F11 (só se o owner mantiver `DevDisabled`). Com opt-out ativo: requisição com `X-Forwarded-For`, `X-Forwarded-Host` ou `Forwarded` → **403** (heurística anti-túnel do SEC-ADM-02, §2.1 E0-T do TM); `Host: example.ngrok.app` → 421; `WARN` no boot; `cli-and-config.md` proíbe operar o opt-out atrás de túnel ou proxy. A entrega registra que a heurística **não detecta** túnel que não acrescenta esses headers (ex.: `ssh -R`), que é o motivo da recomendação de remover o opt-out.

### Divergências registradas (Critic × código × threat model)

- O Critic citou "`[http] allowed_hosts`" como se existisse. Não existe: o `system.toml` só tem `[http.admin]` (`:38-39`). A chave é nova (A5).
- O ciclo anterior citava `meta.rs:43-44` para `live_exchange_wired`; o campo é preenchido em `meta.rs:50` (declarado em `:16`).
- O ciclo anterior dizia "39 usos em testes" de `disabled()`: são 38 em testes e 1 em produção (`state.rs:50`).
- O ciclo anterior citava `server.rs:61` para o `bind`; é `:62`, e `run` vai até `:75`.
- **Para o bot Segurança atualizar o TM antes do G4 de W0-01** (divergências aceitas para G1, sem editar o TM aqui):
  - SEC-ADM-04 fala em layer "no sub-router admin/mutante"; este SDD aplica a layer ao router inteiro (mais restritivo).
  - `GET /meta`, `GET /config/active` e `GET /monitor/snapshot` são `Protected` aqui e não estão na lista mínima do SEC-ADM-05.
  - SEC-ADM-11: o TM ainda diz "429 com backoff crescente"; este SDD usa 429 com `Retry-After`, sem atraso no servidor, contando só 401 em rotas `Protected` casadas (A6).
  - SEC-ADM-02 e SEC-ADM-13 ("exceto opt-out dev em loopback"): se o owner aceitar remover o opt-out, os dois perdem a menção.
  - SEC-ORG-33 usa `Owner`/`Org`; o nome adotado é `Org`.
  - A lista "Documentação que afirma fail-closed" do TM já está alinhada (só resta `admin_auth.rs:1`).
- O Critic citou `state.rs:157` como "construtor de produção". É código de produção (`pub`, fora de `#[cfg(test)]`); no HEAD não há chamador fora de testes. Mesmo assim vira `#[cfg(test)]` (A3).

### Dependências, riscos, validação, rollout (texto legado a reconciliar)

> **Obsoleto onde conflitar:** os bullets abaixo foram escritos para a decisão de startup obrigatório, já substituída pela decisão do owner de 2026-09-27. Não usar o rollback abaixo como plano da nova fatia. O rollout/rollback normativo para esta proposta está na seção “Riscos, rollout e rollback” acima.

- **Dependências:** nenhuma fatia W0 bloqueia. Habilita W0-07 (SEC-CRED-07 depende de SEC-ADM-01) e reduz o abuso de W0-12. G4 precisa de CI verde (W0-02). Acordo dos seams públicos bloqueia G3.
- **Riscos:** caminho de rota novo sem classificação pode ficar aberto; configuração de auth ausente deve produzir 503 sem parar `serve`; 503 pode ser tratado como indisponibilidade por probes; GETs sensíveis e os cálculos POST ainda precisam de classificação confirmada; conflito de merge com outras fatias que tocam `state.rs` (W0-12/W0-13), a sequenciar.
- **Validação:** nenhum teste foi executado nesta entrega de SDD; validação comportamental planejada na seção aprovada acima, após acordo de seams.
- **Rollout:** implementar em ambiente local/staging, conferir listener e rotas públicas sem token, e observar respostas/handler calls 503, 401 e bearer válido; nenhum deploy autorizado.
- **Rollback:** reverter o middleware/guard e bindings desta fatia de forma que rotas protegidas nunca voltem a executar abertas sem token; se essa propriedade não puder ser preservada no rollback, manter a instância atual e corrigir antes de reverter. Não há migração nem dado persistente a desfazer.

## Gate 1 — fatia HTTP (owner binding verificável no seam)

**Estado do seam:** existe, mas fica aberto sem token (falha aberta: `admin_auth.rs:91-94` devolve `Ok(())` sem `BOT_HTTP_ADMIN_TOKEN`). A correção é W0-01 (seção acima).

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
