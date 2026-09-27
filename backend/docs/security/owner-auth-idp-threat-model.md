---
title: Threat model — autenticação do owner / IdP (gate P1)
description: Ativos, fronteiras, atores, casos de abuso, achados e critérios de aceite SEC-OWN para autenticação humana do owner, bootstrap e sessão
tags:
  - security
  - threat-model
  - backend
  - auth
  - idp
status: draft
---

# Threat model — autenticação do owner / IdP (P1)

- **Estado:** proposto — aguardando revisão do Critic independente (`AGENTS.md`: o autor não aprova o próprio trabalho). Ciclo 2 de ENTREGA SEC-TM: números alinhados às decisões do owner D-SEC-* (G4), ver §8.1. Somente documentação; nenhum código, config ou SDD foi alterado.
- **Data / método:** 2026-09-27; leitura estática do checkout local em HEAD `b003a9b7` (os arquivos de código citados têm o mesmo hash que em `2d1e3863`). Nenhum `.env`, token ou segredo foi lido; nenhum teste, scan ou chamada a IdP foi executado.
- **Relacionados:** [admin-http-auth-fail-open](./admin-http-auth-fail-open.md) (F-ADM-01), [org-module-threat-model](./org-module-threat-model.md) (F-ORG-02, F-ORG-03, F-ORG-06, F-ORG-14, F-ORG-15), [orders-g2-threat-model](./orders-g2-threat-model.md) (F-ORD-02), [provider-credentials-plaintext](./provider-credentials-plaintext.md) (F-CRED-01).

## 0. Fontes lidas

| Fonte | Versão / estado |
|---|---|
| [`sdd/org-module-sdd.md`](../sdd/org-module-sdd.md) | Lido em sha256 `03bd020a…`; a versão atual é `905ed5e7…` (381 linhas), usada no [org-module-threat-model](./org-module-threat-model.md). As linhas citadas aqui são de `03bd020a` e **não foram remapeadas** neste ciclo. Referências: §1 linha 27, §3 D-OWNER-BIND/D-OWNER-POSITION (linhas 67-68), §4 linha 74, §5 linha 92, §7.1 linha 131, §8 linhas 168-182, §12 linha 243, §14 linha 260 (P1), §18 linha 310, §20 linha 330. |
| [`planning/org-complete-implementation-plan.md`](../planning/org-complete-implementation-plan.md) | sha256 `e199cb1a…` (não rastreado). Gate P1: linhas 67-73. |
| [`sdd/agents-owner-bootstrap-g1-sdd.md`](../sdd/agents-owner-bootstrap-g1-sdd.md) | `status: partial`; modificado no working tree por outra sessão. Linhas 14-26. |
| [`sdd/http-admin-auth-seam-sdd.md`](../sdd/http-admin-auth-seam-sdd.md) | `status: partial`; linhas 16, 24-29, 35-38. |
| Código | `presentation/http/{register_owner.rs, admin_auth.rs, state.rs, error.rs, routes/agents.rs, routes/meta.rs}`; `modules/agents/{models/product_owner.rs, adapters/pg_owner_bootstrap.rs}`; `core/config/{agents/owner_bootstrap.rs, http/file.rs}`; migration `0010_product_owner_bootstrap.sql`; `docs/operations/runbook.md`. |

Não existe SDD nem ADR de IdP/P1, nem pasta `docs/adr`. Também não há `core/auth` (o plano confirma isso na linha 71). Este modelo cobre o **estado atual** (seam + bootstrap PG) e os **requisitos** que o SDD de P1 precisa atender.

## 1. Estado atual em uma frase

Hoje não existe autenticação de owner. O "owner verificado" (`VerifiedProductOwner`) é um `OwnerId` textual gravado num singleton PG por env + ACK (`product_owner.rs:3`, `0010:1-8`). A "prova" de ser o owner em uma requisição é mandar essa string no corpo (`owner_id`, `promoted_by`), e a única barreira de rede é o bearer admin, que é opcional (F-ADM-01).

## 2. Ativos

| ID | Ativo | Por que importa |
|---|---|---|
| A1 | Identidade do owner (`HumanPrincipal(issuer, subject)` futuro; hoje `OwnerId` textual) | Raiz de autoridade: agents, promoção de bots, policy global, transferência. |
| A2 | Vínculo owner↔organização (`org_owner_bindings`, SDD §11) e singleton `product_owner_bootstrap` | Quem é owner de qual org; migração D-OWNER-BIND. |
| A3 | Sessões/tokens do owner (access token, refresh, step-up) | Personificação enquanto válidos. |
| A4 | Chaves de verificação do IdP (JWKS) e configuração `issuer`/`audience`/algs | Trocar uma delas equivale a forjar qualquer owner. |
| A5 | Epoch de sessão/revogação por principal | Garante que revogação vale na próxima requisição (F-ORG-02). |
| A6 | Ações de alto risco que o owner autoriza | CRUD de credenciais de provider (texto claro, F-CRED-01), `orders/submit`, `bots/runtime/promote`, transferência de owner, aprovação de policy global, revogação em massa. |
| A7 | Trilha de auditoria de identidade | Atribuição e investigação. |
| A8 | Código de enrollment / canal de recuperação | Caminho clássico de tomada de conta. |

## 3. Atores

| Ator | Confiança | Capacidade relevante |
|---|---|---|
| Owner humano legítimo | Alta | Autentica no IdP e autoriza ações de alto risco. |
| Operador de infraestrutura | Alta, mas **não** é owner | Controla env/deploy e DB; faz o bootstrap. Não deve conseguir se tornar owner sem passar pelo IdP. |
| Portador do token admin (`BOT_HTTP_ADMIN_TOKEN`) | Média | Operações de infra; nunca identidade de owner (SDD §8, plano linha 70). |
| Cliente de rede não autenticado (local ou remoto, incl. página web via DNS rebinding) | Nenhuma | Alcança a porta HTTP. |
| Atacante com token/sessão roubados (malware, phishing, log vazado) | Nenhuma | Repete tokens válidos. |
| Atacante no canal de recuperação do IdP (e-mail/SIM do owner) | Nenhuma | Tenta recuperar a conta do owner. |
| IdP comprometido ou mal configurado | Externa | Emite tokens válidos para qualquer subject. |
| Agente (runtime) | Não é principal humano | Não autentica como owner; autenticação de agente é P3. |

## 4. Fronteiras de confiança

1. **Cliente → HTTP API** (`presentation/http`): toda identidade que chega no corpo é não confiável. Hoje essa fronteira tem só o bearer admin opcional.
2. **API → IdP** (discovery/JWKS por HTTPS): a origem das chaves precisa ser fixada; resposta do IdP é entrada externa.
3. **API → PostgreSQL** (singleton `product_owner_bootstrap`, futuro `org_owner_bindings` e epoch): quem escreve no DB consegue "virar owner" hoje.
4. **Operador → processo** (env `BOT_PRODUCT_OWNER_BOOTSTRAP_*`, `BOT_HTTP_*`): canal de bootstrap sem prova humana.
5. **Token admin ↔ identidade de owner**: separação exigida no design, acoplada no código (F-OWN-05).

## 5. Casos de abuso

| ID | Caso | Estado / evidência |
|---|---|---|
| AB-OW1 | **Spoofing de owner pelo corpo:** cliente manda `owner_id`/`promoted_by` igual ao owner bootstrapped e age como owner. | **Código atual.** `register_owner.rs:10-15, 37-45`; `state.rs:363-395, 804-810`; `routes/agents.rs:43-45`. Com F-ADM-01, basta alcançar a porta (F-OWN-01). |
| AB-OW2 | **Autodesignação de owner:** sem bootstrap PG e sem `BOT_HTTP_OWNER_ID`, `POST /agents` aceita qualquer `owner_id`; com PG conectado mas sem bootstrap, só a promoção é barrada. | **Código atual.** `register_owner.rs:10-15` (os dois ramos retornam Ok), `admin_auth.rs:109-116`, `register_owner.rs:19-27` (F-OWN-02). |
| AB-OW3 | **Tomada do bootstrap:** primeira instância que sobe com `BOOTSTRAP_ID`+`ACK` contra o DB grava o owner (quem escreve primeiro ganha); `INSERT` direto no DB tem o mesmo efeito; nada liga o owner a uma pessoa autenticada. | **Código atual.** `owner_bootstrap.rs:10-20`, `pg_owner_bootstrap.rs:19-45`, `0010:3-8` (F-OWN-06). |
| AB-OW4 | **Validação fraca de token:** `alg: none`, confusão HS/RS (chave pública usada como segredo HMAC), `kid` apontando para chave do atacante, `jku`/`x5u` no header, `iss`/`aud` não conferidos, ID token usado como access token, `exp`/`nbf` ignorados ou com skew grande. | **Não especificado.** P1 cita issuer/audience/assinatura/rotação/expiry (SDD §8 item 1, §14; plano 69, 73) mas não fixa allowlist de algoritmos, origem do JWKS, tipo de token, skew nem comportamento com `kid` desconhecido (F-OWN-04). |
| AB-OW5 | **Rotação/indisponibilidade do JWKS:** o atacante força refresh contínuo (DoS no IdP), ou uma falha de JWKS faz o verificador "aceitar por enquanto". | Não especificado (F-OWN-04). |
| AB-OW6 | **Token roubado segue válido:** sem checagem de revogação por requisição nem TTL limitado. | **Aberto** — herdado de **F-ORG-02** (SDD §14 linha 260 e §20 linha 330 só dizem "expiry/revogação"). Critérios SEC-OWN-07. |
| AB-OW7 | **Replay** de requisição/token de alto risco (log de proxy, histórico do navegador, extensão). | Plano linha 73 exige teste "replayed", sem mecanismo definido (F-OWN-04, SEC-OWN-08). |
| AB-OW8 | **Tomada de conta via recuperação:** o atacante recupera a conta do owner no IdP (e-mail/SMS) ou usa um procedimento de "recuperação excepcional" sem quórum. | SDD §7.1 linha 131 manda recuperação para "runbook próprio"; `operations/runbook.md` não tem procedimento de owner (F-OWN-07). |
| AB-OW9 | **Sessão roubada executa ação irreversível** (criar/ler credencial, enviar ordem, promover bot, transferir owner) sem nova prova de presença. | Nenhum requisito de MFA/step-up no SDD, no plano ou no código (F-OWN-03). |
| AB-OW10 | **Token admin usado como owner** ou P1 implementado estendendo o seam admin. | Proibido no SDD (§8 linha 172, §10) e no plano (linha 70); acoplado no código (F-OWN-05). |
| AB-OW11 | **Mesma pessoa com duas identidades no IdP** (owner e "CEO humano") burla a separação de funções. | Herdado de F-ORG-06. |
| AB-OW12 | **Negação/repúdio:** ação atribuída ao owner sem registro de quem autenticou. | Eventos só de `bootstrapped`, sem ator (F-OWN-08). |
| AB-OW13 | **CSRF / DNS rebinding** contra a API com sessão do owner em cookie, ou sem auth em loopback. | Sem layer de Host/CORS (F-ADM-01, SEC-ADM-10); modelo de sessão não definido (SEC-OWN-18). |
| AB-OW14 | **Reconhecimento:** `GET /api/v1/meta` sem auth informa se o bearer admin, o owner binding e o bootstrap estão ativos. | `routes/meta.rs:11-14, 43-48` (F-OWN-09). |
| AB-OW15 | **Cross-org:** token válido do owner da org A usado contra a org B. | Plano linha 73 exige o teste; SDD §8 item 4. Critério SEC-OWN-19. |
| AB-OW16 | **IdP comprometido/mal configurado** (autorregistro aberto no tenant do IdP, `aud` curinga). | Não especificado: é preciso ligar o owner a `(issuer, subject)` exato, não a e-mail/grupo (SEC-OWN-10). |

## 6. Achados

| ID | Sev. | Descrição | Local | Correção | Critérios |
|---|---|---|---|---|---|
| **F-OWN-01** | **Alto** (Crítico com F-ADM-01 + porta exposta ou chaves reais) | Não há autenticação de owner: a identidade do owner é uma string comparada com o corpo da requisição. Quem sabe (ou chuta) o `OwnerId` age como owner ao registrar agents e promover bots. Com o bearer admin desligado (padrão), basta alcançar a porta. É a raiz de F-ORG-03 no escopo owner. | `register_owner.rs:5-16, 32-48`; `state.rs:363-395, 804-810`; `routes/agents.rs:43-45`; `product_owner.rs:3` | Implementar P1: verificador de IdP que produz `AuthContext` no servidor; rotas de owner ignoram identidade do corpo; até lá, rotas de owner fail-closed em qualquer ambiente exposto. | SEC-OWN-01…06, 11, 12, 22 |
| **F-OWN-02** | **Alto** | Autodesignação: sem bootstrap e sem `BOT_HTTP_OWNER_ID`, `POST /agents` aceita qualquer `owner_id` (registro não exige bootstrap; só a promoção exige). Dá para criar hierarquias sob um "owner" arbitrário que depois pode ser migrado (D-OWNER-BIND). | `register_owner.rs:10-15, 19-27`; `admin_auth.rs:24-37, 109-116` | Toda rota que atribui ou usa owner exige owner autenticado e bootstrap concluído (403 `owner_bootstrap_required`); nenhuma identidade de owner vem do corpo. | SEC-OWN-01, 11, 23 |
| **F-OWN-03** | **Alto** | Sem MFA/step-up para ações de alto risco. Nenhum documento exige fator forte nem `auth_time` recente para CRUD de credenciais (segredos em texto claro, F-CRED-01), `orders/submit`, promoção de bot, transferência de owner ou aprovação da policy global. Uma sessão roubada (AB-OW6) executa ações financeiras irreversíveis. | SDD §8 linhas 172-182, §14 linha 260 (ausência); plano linhas 67-73 (ausência) | Incluir no SDD de P1 a lista de ações de alto risco com step-up (`amr` resistente a phishing, `auth_time` ≤ D-SEC-STEPUP-MAXAGE) e desafio 401 `step_up_required`. | SEC-OWN-09, 13 |
| F-OWN-04 | Médio (bloqueia o fechamento de P1) | O requisito de validação de token está incompleto: faltam allowlist de algoritmos, proibição de `jku`/`x5u`/chave embutida, origem fixa do JWKS por HTTPS, cache/refresh com rate limit, tratamento de `kid` desconhecido, `nbf`/`iat`/skew, tipo de token (access × ID), `azp`, vida máxima do token, anti-replay e comportamento fail-closed quando o IdP cai. | SDD §8 item 1 (linha 174), §14 linha 260, §18 linha 310; plano linhas 69, 73 | Especificar conforme SEC-OWN-02…06, 08, 21, com testes negativos no entrypoint real. | SEC-OWN-02…06, 08, 21, 22 |
| F-OWN-05 | Médio | O token admin e a identidade de owner estão acoplados no código: o owner binding só existe com o bearer admin; `verify_register_owner_id` delega ao seam admin; o erro `owner_mismatch` cita `BOT_HTTP_OWNER_ID` mesmo quando a comparação foi contra o bootstrap. Isso incentiva implementar P1 como extensão do seam. | `admin_auth.rs:24-37, 108-116`; `register_owner.rs:15`; `error.rs:106-112` | Tipo `AuthContext` sem construtor a partir de `HttpAdminAuth`; rotas de owner rejeitam só-bearer-admin; mensagens de erro neutras. | SEC-OWN-12 |
| F-OWN-06 | Médio | Bootstrap sem prova humana e com "quem escreve primeiro ganha": env ID + um booleano `ACK`; qualquer instância com acesso ao DB (ou um `INSERT` direto) define o owner; conflito env×PG só gera log e o processo continua servindo; `source` só aceita `env_explicit`; nenhum vínculo com IdP nem com organização. | `owner_bootstrap.rs:10-20`; `pg_owner_bootstrap.rs:19-45`; `state.rs:260-280`; `0010:3-8` | Bootstrap P1 por comando de operador ou enrollment assinado, de uso único, que liga um `(issuer, subject)` provado por login no IdP; conflito é fatal no boot. | SEC-OWN-10, 11, 21, 23 |
| F-OWN-07 | Médio | Recuperação de conta e transferência sem procedimento: o SDD manda para um runbook que não existe; não há quórum, período de espera, notificação ao owner atual nem revogação de sessões. | SDD §7.1 linha 131, §17 linha 303; `operations/runbook.md` (sem seção de owner) | Runbook de recuperação com quórum + espera + notificação + revogação total; política de MFA obrigatória no IdP para a conta owner. | SEC-OWN-13, 14 |
| F-OWN-08 | Médio | Trilha de auditoria de identidade insuficiente: `product_owner_bootstrap_events` só tem `kind='bootstrapped'`, sem ator, host ou processo; não há proteção append-only no DB (nenhum `REVOKE`/trigger nas migrations); ações "do owner" registram o `promoted_by` enviado pelo cliente. | `0010:10-15`; migrations sem `REVOKE`/`TRIGGER`; `state.rs:363-395` | Eventos append-only de auth/bootstrap/step-up/transferência/recuperação/revogação com ator do contexto; role de runtime sem `UPDATE`/`DELETE`. | SEC-OWN-15 |
| F-OWN-09 | Baixo | `GET /api/v1/meta` sem auth revela `http_admin_auth_enabled`, `http_owner_binding_active` e `product_owner_bootstrap_active`, o que ajuda um atacante a saber se a API está aberta. | `routes/meta.rs:11-14, 43-48`; `state.rs:659-669` | Em produção, meta sem auth não expõe o estado de auth (ou exige auth). | SEC-OWN-20 |
| F-OWN-10 | Baixo | O nome `VerifiedProductOwner` sugere autenticação (o próprio comentário diz "not human IdP auth"), o que abre risco de uso indevido como prova de owner em código novo. | `product_owner.rs:3-5` | Renomear/documentar (ex. `BootstrappedProductOwnerId`) e barrar seu uso em autorização de `org` (SDD §3 D-OWNER-BIND já proíbe). | SEC-OWN-12 |
| F-OWN-11 | Baixo | O relógio de expiração/auditoria vem da aplicação (`SystemTime` → `at_ms`, com `unwrap_or(0)`); relógio errado grava `0` ou aceita tokens vencidos. | `state.rs:264-267` | Relógio autoritativo (PG `now()` ou NTP monitorado), skew limitado (ver F-ORG-14). | SEC-OWN-04 |

**Contagem:** Crítico 0 (condicional em F-OWN-01) · **Alto 3** (F-OWN-01, 02, 03) · Médio 5 (F-OWN-04…08) · Baixo 3 (F-OWN-09…11). Herdados e referenciados, não recontados aqui: **F-ADM-01** (Alto), **F-ORG-02** (Alto, revogação por requisição), **F-ORG-03** (Alto, autoridade vinda do cliente), F-ORG-06, F-ORG-14, F-ORG-15.

## 7. Mitigações (visão geral)

1. **Verificador P1 isolado** (local definido no SDD de P1, ver plano linha 71): OIDC/OAuth 2.x com issuer fixo; produz `AuthContext { principal: HumanPrincipal, org, session_id, auth_time, amr, session_epoch }`; controllers só recebem esse tipo.
2. **Validação estrita** (RFC 8725 / RFC 9068): algoritmos por allowlist; chaves só do JWKS do issuer fixado; `iss`/`aud`/`azp`/`typ`/`exp`/`nbf`/`iat` conferidos; fail-closed se o IdP estiver indisponível sem chave em cache válida.
3. **Sessão curta + epoch por requisição**: vida do access token ≤ **D-SEC-TTL**; epoch por principal lido no PG em toda mutação; logout-all/transferência/recuperação incrementam o epoch (fecha F-ORG-02).
4. **Step-up para alto risco** com fator resistente a phishing (WebAuthn) e tokens com vínculo ao remetente (DPoP ou mTLS) ou `jti` de uso único.
5. **Bootstrap por prova** (login no IdP durante enrollment de uso único) e migração D-OWNER-BIND sem promover strings.
6. **Separação por tipo** entre bearer admin e owner; F-ADM-01 corrigido antes de qualquer rota de owner.
7. **Recuperação com quórum**, espera e notificação; política de MFA no IdP.
8. **Auditoria append-only** com ator do contexto e referência pseudonimizada (SDD §12 linha 243).

## 8. Critérios de aceite (testáveis)

| ID | Critério | Teste de aceite |
|---|---|---|
| **SEC-OWN-01** | Identidade sempre derivada do servidor. Nenhuma rota de owner usa `owner_id`, `promoted_by`, `actor` ou equivalente do corpo como identidade. | Requisição com token do owner X e corpo com `owner_id=Y` → ação registrada como X, ou **400** se o campo for proibido; nunca Y. Teste arquitetural: controllers de owner só recebem `AuthContext`. |
| **SEC-OWN-02** | Assinatura: algoritmos por allowlist configurada (ex. `RS256`, `ES256`, `EdDSA`); o `alg` do token precisa bater com o da JWK; `jku`, `x5u` e `jwk` do header são ignorados. | Tokens com `alg: none`, HS256 assinado com a chave pública RSA, `alg` divergente da JWK, `jku` apontando para chave do atacante → **401** em todos. |
| **SEC-OWN-03** | `iss` igual (byte a byte) ao issuer configurado; `aud` contém o audience da API; com várias audiences, `azp` = client esperado. | Token de outro issuer, de outro audience ou com `azp` estranho → **401**. |
| **SEC-OWN-04** | Tempo: `exp` obrigatório; `nbf`/`iat` checados com tolerância ≤ **D-SEC-LEEWAY**; `exp − iat` ≤ **D-SEC-TTL** para access token; relógio autoritativo. | Sem `exp`, vencido há D-SEC-LEEWAY + 1 s, `nbf` além de D-SEC-LEEWAY no futuro, `exp − iat` > D-SEC-TTL → **401**. |
| **SEC-OWN-05** | Só access token é aceito como bearer (`typ: at+jwt` ou claim configurado); ID token e refresh token → rejeitados. | ID token válido do mesmo issuer usado como bearer → **401**. |
| **SEC-OWN-06** | JWKS: `jwks_uri` só do discovery do issuer fixado, por HTTPS com TLS verificado; cache com TTL; `kid` desconhecido → no máximo um refresh por **D-SEC-JWKS-REFRESH**, depois **401**; IdP fora do ar sem chave válida em cache → **401/503**, nunca aceitar sem verificar; chave removida deixa de valer ao fim do TTL. | Fixture de IdP: rotação (chave nova aceita após refresh; chave antiga rejeitada após TTL); rajada de `kid` aleatórios gera ≤ 1 refresh por D-SEC-JWKS-REFRESH; JWKS 500 com cache vazio → negado. |
| **SEC-OWN-07** | Revogação por requisição (fecha F-ORG-02): `session_epoch` por principal no PG, conferido em toda requisição mutante (leituras: cache ≤ **D-SEC-EPOCH-READ-CACHE**); logout-all, revogação, transferência e recuperação incrementam o epoch; vida total da sessão/refresh limitada a **D-SEC-SESSION-MAX**. | Token emitido antes do bump do epoch → **401** na próxima mutação (sem esperar `exp`); refresh após logout-all → negado. |
| **SEC-OWN-08** | Anti-replay para alto risco: token com vínculo ao remetente (DPoP RFC 9449 ou mTLS) **ou** prova de step-up com `jti` de uso único guardado até expirar, além da idempotency key da ação. | Reenviar a mesma prova DPoP/`jti` → **401/409**; token sem a chave DPoP correspondente → **401**. |
| **SEC-OWN-09** | Step-up/MFA: a lista de ações de alto risco (CRUD `admin/provider-credentials*`, `orders/submit` em modo que executa, `bots/runtime/promote`, transferência de owner, aprovação de policy global, mudança de config de auth, revogação em massa) exige `amr` com fator resistente a phishing (ex. `hwk`/WebAuthn) e `auth_time` não mais antigo que **D-SEC-STEPUP-MAXAGE**. | Token válido sem esse fator ou com `auth_time` mais antigo que D-SEC-STEPUP-MAXAGE → **401** `step_up_required` com desafio (`max_age`/`acr_values`); com step-up → sucesso; teste enumera a lista. |
| **SEC-OWN-10** | Bootstrap do primeiro owner: sem endpoint público; comando de operador (ou enrollment assinado) gera código de uso único (guarda só o hash, expira em ≤ **D-SEC-ENROLL-TTL**) resgatado com **login no IdP**; liga `(issuer, subject)` exato (não e-mail/grupo) à organização. | Código reutilizado ou expirado → erro; resgate sem token do IdP → **401**; segunda tentativa com owner já ligado → erro e exit ≠ 0 no comando; evento auditado. |
| **SEC-OWN-11** | Sem owner ligado, todas as rotas que usam ou atribuem owner (incl. `POST /agents`, não só promote) → **403** `owner_bootstrap_required`; o perfil de produção não serve rotas de owner sem config P1 válida. | Teste HTTP com PG sem bootstrap: `POST /agents` → 403. |
| **SEC-OWN-12** | Separação admin × owner: `AuthContext` não pode ser construído a partir de `HttpAdminAuth` (tipo); rotas de owner com apenas o bearer admin → **401**; `VerifiedProductOwner` (ou sucessor) não autoriza nada no `org`. | Teste de compilação/arquitetura + teste HTTP com só o bearer admin em rota de owner → 401. |
| **SEC-OWN-13** | Transferência de owner: aprovação autenticada **com step-up** do owner atual e do novo; incrementa o epoch dos dois; evento append-only; nunca via token admin. | Só uma aprovação → estado pendente; sessão antiga do owner anterior → 401 depois da transferência. |
| **SEC-OWN-14** | Recuperação de conta: runbook fora da API comum com quórum de ≥ 2 operadores ou prova fora de banda, período de espera **D-SEC-RECOVERY-WAIT** com notificação ao owner atual e opção de cancelar, revogação de todas as sessões e evento auditado; política documentada de MFA obrigatório no IdP para a conta owner. | Exercício de mesa registrado; tentativa de recuperação durante a espera cancelada pelo owner → sem efeito. |
| **SEC-OWN-15** | Auditoria append-only: login com sucesso, falhas agregadas, step-up, bootstrap, transferência, recuperação, revogação e toda ação de alto risco com `principal_ref` pseudonimizado, hash do `session_id`, ação, alvo, decisão e correlation ID; role de runtime sem `UPDATE`/`DELETE`. | `UPDATE`/`DELETE` pela role de runtime → erro de permissão; cada ação de alto risco gera exatamente um evento com o ator do contexto. |
| **SEC-OWN-16** | Tokens, códigos de enrollment e prova DPoP nunca aparecem em log, `Debug` ou erro. | Captura de logs com token conhecido → ausente; `format!("{:?}")` dos tipos de credencial → redigido. |
| **SEC-OWN-17** | Rate limit/lockout de falhas por IP e por principal; resposta 401 uniforme (não revela se o principal existe nem por que falhou). | N falhas → **429**; corpo de erro idêntico para token inválido, vencido ou de principal desconhecido. |
| **SEC-OWN-18** | Modelo de sessão definido: com cookie, `Secure; HttpOnly; SameSite=Strict` + token CSRF em mutações; com bearer, sem credencial em cookie; CORS por allowlist; Host por allowlist (SEC-ADM-10). | Mutação cross-site sem token CSRF → **403**; `Host` fora da allowlist → **421/400**. |
| **SEC-OWN-19** | Isolamento de org: principal owner da org A em rota da org B → **403/404** uniforme. | Teste HTTP cross-org (plano linha 73). |
| **SEC-OWN-20** | `GET /meta` sem auth, em produção, não expõe estado de auth/bootstrap (decisão explícita registrada). | Teste no perfil de produção: campos ausentes ou exige auth. |
| **SEC-OWN-21** | Config P1 validada no boot: issuer, audience, algs e client obrigatórios fora do perfil dev; ausentes ou inválidos → exit ≠ 0 e porta não aberta (espelha SEC-ADM-01). | Subir o perfil de produção sem issuer → exit ≠ 0. |
| **SEC-OWN-22** | Matriz de testes no entrypoint HTTP real com IdP de teste: válido; assinatura inválida; vencido; `nbf` futuro; issuer errado; audience errado; `alg: none`; HS/RS; `kid` desconhecido; chave rotacionada; revogado por epoch; replay; org divergente; ID token como bearer; alto risco sem step-up; só bearer admin. | Todos os casos negativos → 401/403 esperados; o caso válido passa. |
| **SEC-OWN-23** | Migração D-OWNER-BIND: o `product_owner_bootstrap.owner_id` textual não vira `HumanPrincipal` sem login do próprio owner no IdP durante a migração; até lá, nenhuma autoridade de owner é derivada do singleton. | Migração sem login → vínculo não criado; teste confirma que o singleton sozinho não autoriza. |

### 8.1 Parâmetros que são decisão do owner (G4)

Este documento não fixa os números. Os IDs compartilhados com o `org` têm a mesma definição de [org-module-threat-model §6.4](./org-module-threat-model.md); os valores entre parênteses são só sugestão inicial.

| ID | Parâmetro | Usado em | Sugestão (não vinculante) |
|---|---|---|---|
| D-SEC-TTL | Vida máxima do access token humano (compartilhado com `org`) | SEC-OWN-04, §7 item 3; SEC-ORG-15 | ≤ 15 min |
| D-SEC-LEEWAY | Tolerância de relógio em `exp`/`nbf`/`iat` (compartilhado com `org`) | SEC-OWN-04; SEC-ORG-15 | ≤ 60 s |
| D-SEC-JWKS-REFRESH | Intervalo mínimo entre refreshes de JWKS por `kid` desconhecido | SEC-OWN-06 | 1 min |
| D-SEC-EPOCH-READ-CACHE | Idade máxima do epoch em cache para leituras (mutações sempre leem o PG) | SEC-OWN-07 | ≤ 30 s |
| D-SEC-SESSION-MAX | Vida total de sessão/refresh | SEC-OWN-07 | ≤ 12 h |
| D-SEC-STEPUP-MAXAGE | Idade máxima de `auth_time` para ações de alto risco | SEC-OWN-09 | ≤ 5 min |
| D-SEC-ENROLL-TTL | Validade do código de enrollment do primeiro owner | SEC-OWN-10 | ≤ 15 min |
| D-SEC-RECOVERY-WAIT | Período de espera da recuperação de conta | SEC-OWN-14 | 24-72 h |

Os limites de rate limit/lockout de SEC-OWN-17 ("N falhas") também são decisão do owner e seguem a mesma regra de SEC-ADM-11 (D-SEC-ADM-RATE, em [admin-http-auth-fail-open](./admin-http-auth-fail-open.md)).

## 9. Ordem sugerida

1. Corrigir **F-ADM-01** (SEC-ADM-01…03) antes de qualquer trabalho em rotas de owner.
2. O Architect escreve o SDD de P1 incorporando F-OWN-03/04/06/07 e F-ORG-02 (SEC-OWN-02…10, 13, 14, 21).
3. O Builder implementa o verificador e `AuthContext` (SEC-OWN-01, 11, 12, 22); só então habilitar step-up e alto risco.
4. Critic independente revisa este modelo e o SDD de P1 antes do LGTM.

## 10. Não verificado

- Não existe SDD/ADR de P1: provedor, protocolo e modelo de sessão são hipóteses (OIDC/OAuth assumido).
- Não verifiquei o ambiente real (bind, proxy, TLS, config do IdP, roles do PG); nenhum teste foi executado.
- O SDD `org` e o plano estão em edição ativa (hashes na §0); as linhas citadas podem mudar.
- `agents-owner-bootstrap-g1-sdd.md` está modificado no working tree por outra sessão; li a versão presente no checkout em 2026-09-27.
