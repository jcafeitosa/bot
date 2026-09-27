---
title: Achado — credenciais de provider/exchange em texto claro
description: Registro do problema conhecido de credenciais sem criptografia em repouso, com locais, impacto, correção e critérios SEC-CRED
tags:
  - security
  - threat-model
  - backend
  - credentials
  - finding
status: draft
---

# Achado F-CRED — credenciais de provider/exchange em texto claro

- **Estado:** proposto — aguardando revisão do Critic independente. Documentação apenas; nenhuma correção aplicada.
- **Data:** 2026-09-27. **Base:** HEAD `2d1e3863` (arquivos citados inalterados desde `0880cdd5`).
- **Método:** leitura estática de schema, repositório, cache, HTTP e logging. **Nenhum valor real foi lido**: `.env*` e o banco não foram acessados; o template `seed-provider-credentials.example.sql` não foi necessário.
- **Entradas:** [`sdd/provider-credentials-db-sdd.md`](../sdd/provider-credentials-db-sdd.md) (sem frontmatter; declara "plaintext until encryption ADR"), migrations `0007_provider_credentials.sql`/`0008`, `core/providers/credentials/{mod.rs,cache.rs,store.rs}`, `modules/http_bridge/provider_credentials.rs`, `presentation/http/routes/provider_credentials_admin.rs`, `presentation/http/error.rs`, `core/config/{mod.rs,exchanges/credentials.rs,providers/file.rs}`, `core/providers/{openai_compatible.rs,nvidia_nim.rs}`.
- **Relacionados:** [admin-http-auth-fail-open](./admin-http-auth-fail-open.md) (F-ADM-01 agrava este achado), [orders-g2-threat-model](./orders-g2-threat-model.md), [org-module-threat-model](./org-module-threat-model.md).

## 1. Onde e como as credenciais estão

| Credencial | Armazenamento | Evidência |
|---|---|---|
| API keys de LLM (`typesafe`, `openai`, `nvidia`, `ngc`) | PostgreSQL `trading_bot.provider_credentials.secret TEXT NOT NULL`, **texto claro** | `core/database/migrations/0007_provider_credentials.sql:3-9`; `core/providers/credentials/mod.rs:5` (`PROVIDER_CREDENTIALS_ENCRYPTION_MODE = "none"`), exposto em `/meta` e `/healthz`. |
| Cache em processo | `static CACHE: Mutex<Option<HashMap<(String,String), String>>>` com os segredos em claro | `cache.rs:12, 14-31`. |
| Fallback legado | env `TYPESAFE_API_KEY`, `OPENAI_API_KEY`, `NVIDIA_API_KEY`, `NGC_API_KEY` (tipicamente `.env` em disco) | `cache.rs:41-63`; `core/config/providers/file.rs:10-11, 65-66`. |
| Chaves de exchange | Somente env: `BINANCE_TESTNET_API_KEY/SECRET`; loader também lê `BINANCE_PROD_API_KEY/SECRET` | `core/config/exchanges/credentials.rs:5-24` (`credentials_for_environment`). Não estão no PG. |
| Admin CRUD | `GET/POST/PUT/DELETE /api/v1/admin/provider-credentials*`, protegido só por `require_http_admin` (fail-open sem token — F-ADM-01) | `provider_credentials_admin.rs:42, 67, 93, 117`. |

**Logging/exposição observados:** listagem devolve `****` + 4 últimos caracteres (`store.rs:11-21`), mas o `SELECT` traz o segredo completo para a aplicação só para mascarar (`store.rs:93-97`); nenhum `tracing` imprime o valor (o fallback loga só o nome da variável, `cache.rs:53-58`); porém vários tipos com segredo derivam `Debug`: `UpsertProviderCredentialRequest`/`ReplaceProviderCredentialRequest` (`http_bridge/provider_credentials.rs:23-33`), `Credentials` (`core/config/mod.rs:259-263`), `OpenAiCompatibleConfig`/`OpenAiCompatibleClient` (`openai_compatible.rs:10-22`, `NvidiaNimClient` embrulha o último), `HttpAdminAuth`/`HttpAdminAuthConfig`. Erros de store devolvem texto do driver ao cliente (`store.rs:48-52` → `error.rs:258-262`). Não há trilha de auditoria de quem criou/alterou/apagou (só `updated_at`).

## 2. Achados

| ID | Sev. | Achado | Local | Impacto | Correção | Critérios |
|---|---|---|---|---|---|---|
| **F-CRED-01** | **Alto** | API keys de LLM em texto claro no PostgreSQL (problema conhecido, `encryption=none`). | `0007_provider_credentials.sql:6`; `mod.rs:5`; `cache.rs:14-31`; `store.rs:93-97, 114-142` | Qualquer leitura do banco (backup/`pg_dump`, réplica, role de leitura, DBA, SQL injection futura em outro módulo, log de parâmetros do servidor PG com `INSERT`/`UPDATE` bindados) revela as chaves → abuso de faturamento, acesso aos dados/prompt logs da conta do provider. Agravado por F-ADM-01 (sobrescrita/remoção sem credencial). | Envelope encryption (DEK por registro, KEK em KMS/cofre fora do DB), AAD vinculando `provider_id‖key_name‖version`, migração dos registros, redação de `Debug`, auditoria de acesso, rotação versionada, menor privilégio no PG. | SEC-CRED-01…10 |
| F-CRED-02 | Médio | Chaves de exchange só em env/`.env` em texto claro; loader de `BINANCE_PROD_*` existe apesar de prod REST bloqueado. | `core/config/exchanges/credentials.rs:17-24` | Hoje só testnet. Torna-se **Crítico** se chaves mainnet com permissão de trade forem colocadas no ambiente sem restrição de IP/saque. | Secret manager/arquivo cifrado; loader prod atrás de gate explícito; checklist de permissões da chave. | SEC-CRED-11 |
| F-CRED-03 | Médio | `Debug` derivado em tipos com segredo (vazamento latente em qualquer `?x`/`{:?}`/panic). | ver §1 | Um log de depuração ou mensagem de pânico expõe chave/token. | `secrecy::SecretString` ou `Debug` manual redigido. | SEC-CRED-04, 05 |
| F-CRED-04 | Médio | Sem trilha de auditoria de criação/alteração/remoção e sem ator (token compartilhado, possivelmente ausente). | `store.rs:114-164`; `provider_credentials_admin.rs` | Impossível investigar troca maliciosa de chave. | Evento append-only por operação com principal. | SEC-CRED-06, 07 |
| F-CRED-05 | Baixo | Máscara revela 4 últimos caracteres e a listagem carrega o segredo inteiro na memória. | `store.rs:11-21, 93-97` | Exposição desnecessária; ajuda correlação. | Não selecionar `secret`; mostrar fingerprint (HMAC truncado) ou só metadados. | SEC-CRED-10 |
| F-CRED-06 | Baixo | Redação de erros de exchange só por substituição exata dos valores testnet (não cobre prod, assinaturas, valores parciais/codificados); erros de store devolvem texto do driver. | `core/config/exchanges/credentials.rs:27-37`; `error.rs:258-262` | Vazamento em mensagens de erro. | Redator central + códigos genéricos. | SEC-CRED-12 |
| F-CRED-07 | Baixo | Duas fontes de verdade (PG + fallback env legado) sem visibilidade de qual foi usada. | `cache.rs:41-63` | Chave antiga em `.env` usada silenciosamente após rotação no PG. | Desligar fallback fora de dev; expor origem (sem valor) em `/meta`. | SEC-CRED-08 |

**Contagem:** Crítico 0 · Alto 1 (F-CRED-01) · Médio 3 · Baixo 3.

## 3. Critérios de aceite testáveis (SEC-CRED)

| ID | Critério |
|---|---|
| **SEC-CRED-01** | Criptografia envelope: colunas `secret_ciphertext BYTEA`, `wrapped_dek BYTEA`, `kek_id TEXT`, `alg TEXT`, `nonce BYTEA`, `version INT`; AEAD (ex.: AES-256-GCM) com AAD = `provider_id‖key_name‖version`. Teste PG isolado: após upsert de segredo-marcador, nenhuma coluna contém os bytes do marcador; trocar `provider_id` da linha (AAD diferente) faz a decifragem falhar. |
| **SEC-CRED-02** | KEK fora do banco e do repositório (KMS/cofre ou arquivo com permissão restrita fora do checkout); sem KEK disponível → clientes de provider não iniciam, CRUD retorna **503** `provider_credentials_kek_unavailable`, sem fallback para texto claro. Teste com KEK ausente. |
| **SEC-CRED-03** | Migração: comando/migration reversível cifra as linhas existentes e, ao final, `secret` (texto claro) é removida ou fica `NULL` com `CHECK (secret IS NULL)`; `PROVIDER_CREDENTIALS_ENCRYPTION_MODE` e `http_seams.provider_credentials_encryption` só mudam de `none` quando leitura e escrita usam o encoder (o teste `provider_credentials_encryption_mode_is_explicit_none_fail_closed` é substituído por teste que exige o novo modo junto com SEC-CRED-01). |
| **SEC-CRED-04** | Redação de tipos: `format!("{:?}", x)` para cada tipo listado em §1 contém `<redacted>` e não o valor de fixture. |
| **SEC-CRED-05** | Captura de logs: com subscriber de teste em nível `TRACE`, fluxos de upsert/replace/delete/list/reload e erro de provider não emitem o segredo de fixture. Checklist ops (verificação manual registrada): servidor PG sem log de parâmetros bindados para essa tabela (`log_parameter_max_length = 0` ou equivalente) — ou irrelevante após SEC-CRED-01. |
| **SEC-CRED-06** | Auditoria: cada create/replace/delete (e qualquer leitura integral futura) grava evento append-only com principal, `provider_id`, `key_name`, ação, resultado, `correlation_id`, **sem** segredo. Role de runtime sem `UPDATE`/`DELETE` na tabela de auditoria (teste). |
| **SEC-CRED-07** | Autorização: sem auth configurada as rotas não existem ou retornam **503** (nunca 2xx) — depende de SEC-ADM-01; pós-P1, mutação exige principal owner com step-up; token admin sozinho → **401**. |
| **SEC-CRED-08** | Rotação: nova versão ativada atomicamente; `reload` sem restart; teste com provider fake confirma que a chamada seguinte usa a nova chave e nunca a antiga; versões antigas destruídas após janela configurada; fallback env desligado fora de dev (teste: com linha no PG e env diferente, o valor do PG vence; com perfil prod e sem linha, erro em vez de env). |
| **SEC-CRED-09** | Menor privilégio no PG: roles de leitura/analytics sem `SELECT` em `provider_credentials` (`has_table_privilege` = false); só a role de runtime acessa. |
| **SEC-CRED-10** | Listagem não seleciona o segredo: query de list retorna só metadados + fingerprint HMAC truncado; teste de que a resposta não contém sufixo do segredo. |
| **SEC-CRED-11** | Chaves de exchange: carregadas de secret manager ou arquivo cifrado; loader `BINANCE_PROD_*` só com gate prod explícito aprovado (fora deste escopo, bloqueado); checklist registrado de chave com restrição de IP e saque desabilitado antes de qualquer mainnet. |
| **SEC-CRED-12** | Redação central de erros: erro fake contendo chave, secret, `signature=…` e header `X-MBX-APIKEY` → corpo HTTP e logs sem esses valores; store errors → mensagem genérica. |

## 4. Itens não verificados

- Configuração real de logging do servidor PG, backups, réplicas e privilégios de roles nos ambientes.
- Se há chaves de valor real nos ambientes (por regra, nenhum valor foi lido).
