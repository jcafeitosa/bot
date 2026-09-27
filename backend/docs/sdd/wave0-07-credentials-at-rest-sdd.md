---
title: SDD W0-07 — credenciais de provider cifradas no PostgreSQL
description: Delta da Onda 0 sobre provider-credentials-db-sdd com criptografia envelope, KEK fora do banco e critérios SEC-CRED-01..12
tags:
  - sdd
  - backend
  - security
  - credentials
  - wave0
status: draft
---

# SDD W0-07 — credenciais de provider cifradas no PostgreSQL [SEGURANÇA]

- **Estado:** draft. Nenhum gate aprovado. Precisa de Critic independente (G1), revisão do bot Segurança contra [provider-credentials-plaintext](../security/provider-credentials-plaintext.md) e decisão do Julio sobre o [ADR 0001](../adr/0001-credentials-kek.md) antes do primeiro teste.
- **Plano:** W0-07 em [master-plan](../planning/master-plan.md) §4.1 e D5.
- **SDD base:** [provider-credentials-db-sdd](./provider-credentials-db-sdd.md). Este arquivo é um **delta**: substitui a seção "Encryption-at-rest (follow-up, not this slice)" daquele SDD. Foi escrito em arquivo separado porque o SDD base tinha mudanças não commitadas de outra sessão no momento da redação. Quando o base estiver limpo, ele deve ganhar um link para cá.
- **Threat model:** do bot Segurança (achados F-CRED-01..07); não repetido aqui.

## Contexto (evidência no código, HEAD `72eb471d`)

- `0007_provider_credentials.sql`: `secret TEXT` em claro.
- `core/providers/credentials/mod.rs:5`: `PROVIDER_CREDENTIALS_ENCRYPTION_MODE = "none"`, exposto em `/meta` (`routes/meta.rs:52`) e `/healthz` (`routes/health.rs:56`).
- `core/providers/credentials/cache.rs:12-31`: cache estático do processo com os segredos em claro; `:41-63`: se o cache não tem a chave, usa env (`TYPESAFE_API_KEY` etc.) sem dizer a origem.
- `core/providers/credentials/store.rs:93-97`: a listagem seleciona o segredo inteiro para montar a máscara; `:11-21` (`mask_secret`) corta por bytes (`trimmed.len() - 4`), o que pode entrar no meio de um caractere UTF-8 e dar pânico.
- `Database::migrate` chama `reload_from_pool` no boot.
- Dependências: `ring`, `aws-lc-rs` e `zeroize` só aparecem como transitivas no `Cargo.lock`; não há crate AEAD direta no `Cargo.toml`.

## Decisão

1. **Envelope (SEC-CRED-01):** por linha, uma DEK aleatória de 256 bits cifra o segredo com AES-256-GCM; a DEK é cifrada pela KEK (também AES-256-GCM). AAD = `provider_id‖key_name‖version` (com separador de tamanho fixo, para não haver ambiguidade de concatenação).
2. **KEK:** origem conforme [ADR 0001](../adr/0001-credentials-kek.md) (recomendação: env ou arquivo agora). Um `kek_id` por linha permite ter KEK antiga e nova durante a rotação.
3. **Migração (SEC-CRED-03):** SQL na **próxima sequência livre no momento da implementação** adiciona as colunas cifradas (anuláveis). A aplicação, com KEK presente, cifra as linhas em claro no boot e anula `secret` na mesma TX, de forma idempotente. Uma segunda migração, depois de verificado, só adiciona `CHECK (secret IS NULL)`. Backups do PG anteriores continuam com texto claro: depois da migração, as chaves nos providers devem ser rotacionadas (entra no runbook de W1-03).
4. **Modo:** `PROVIDER_CREDENTIALS_ENCRYPTION_MODE` passa a `"envelope-v1"` só quando leitura e escrita usam o encoder; o teste `provider_credentials_encryption_mode_is_explicit_none_fail_closed` e os dois testes HTTP `*_encryption_none` são substituídos.
5. **Sem KEK ou KEK errada (SEC-CRED-02):** clientes de provider não sobem; CRUD responde **503** `provider_credentials_kek_unavailable`; nenhum fallback para texto claro.
6. **Listagem (SEC-CRED-10):** não seleciona segredo nem ciphertext; mostra fingerprint HMAC truncado. Isso também remove o pânico de `mask_secret`.
7. **Cache e tipos (SEC-CRED-04/05):** segredo em memória como tipo com `Debug` redigido e zeroização ao descartar.
8. **Fallback env (SEC-CRED-08, parte):** continua só em dev; fora de dev, sem linha no PG → erro. `/meta` expõe a origem (`postgres` \| `env`), nunca o valor.
9. **Crate:** uma crate AEAD direta (ex.: `aes-gcm`, ou `ring`/`aws-lc-rs`, já presentes como transitivas). Escolha do Builder com o Critic; é dependência nova.

**Alternativa considerada:** cifrar com a KEK direto, sem DEK por linha (uma camada). É mais simples, mas trocar a KEK obriga re-cifrar todos os segredos, e a migração para KMS exigiria uma chamada ao KMS por leitura. Com envelope, a rotação e a ida para KMS só re-cifram as DEKs. Rejeitada.

## Mapa SEC-CRED-01..12

| Critério | Nesta fatia | Observação |
|---|---|---|
| SEC-CRED-01 envelope + AAD | sim | teste PG: marcador ausente em todas as colunas; trocar `provider_id` da linha faz a decifragem falhar |
| SEC-CRED-02 KEK fora + 503 | sim | teste com KEK ausente e com KEK errada |
| SEC-CRED-03 migração + modo | sim | boot cifra linhas antigas e anula `secret`; segunda migração adiciona o `CHECK` |
| SEC-CRED-04 `Debug` redigido | sim | teste `format!("{:?}")` com fixture |
| SEC-CRED-05 logs sem segredo | sim (teste) | checklist PG de log de parâmetros vai para W1-03 |
| SEC-CRED-06 auditoria append-only | parcial | evento sem segredo com `principal = "admin-token"` até P1; principal real depende de P1 |
| SEC-CRED-07 autorização | não | W0-01 (503 sem auth) e P1 (owner + step-up) |
| SEC-CRED-08 rotação | sim | rotação de KEK (re-cifrar DEKs) e de segredo (nova `version`); fallback env só em dev |
| SEC-CRED-09 menor privilégio PG | parcial | teste `has_table_privilege` quando houver role de leitura; criação das roles é ops (W1-03) |
| SEC-CRED-10 listagem sem segredo | sim | fingerprint HMAC |
| SEC-CRED-11 chaves de exchange | não | fora desta tabela; prod bloqueado; inventário em W1-03 |
| SEC-CRED-12 redação central | parcial | erros de store genéricos aqui; redator de erro de exchange em fatia própria |

## Seams públicos para acordo antes do TDD

| Seam | Proposta |
|---|---|
| Formato da linha | `version INT`, `alg TEXT` (`A256GCM`), `kek_id TEXT`, `nonce BYTEA` (12 bytes), `wrapped_dek BYTEA`, `dek_nonce BYTEA`, `secret_ciphertext BYTEA` |
| Origem da KEK | `PROVIDER_CREDENTIALS_KEK_FILE` ou `PROVIDER_CREDENTIALS_KEK` (32 bytes base64) + `PROVIDER_CREDENTIALS_KEK_ID`; KEK anterior opcional durante rotação (`…_PREVIOUS…`) |
| Trait | `KekProvider { fn wrap(&self, dek) -> Wrapped; fn unwrap(&self, kek_id, wrapped) -> Result<Dek, KekError> }` (local hoje, KMS depois) |
| Erro HTTP sem KEK | 503, `code = "provider_credentials_kek_unavailable"` |
| `/meta` e `/healthz` | `provider_credentials_encryption = "envelope-v1"`; `provider_credentials_source = "postgres" \| "env"` |

## Dependências

- W0-01: SEC-CRED-07 e o CRUD sem auth dependem dele.
- W0-02: os testes PG precisam falhar alto sem PG.
- Decisão do ADR 0001. Habilita W1-03 (runbook e drill de rotação).

## Riscos

- Perder a KEK = perder as credenciais cifradas; exige backup separado da KEK (ADR 0001).
- Boot que cifra linhas antigas precisa ser idempotente e atômico (uma TX); falha no meio faz rollback e deixa as linhas como estavam.
- Backups antigos do PG guardam as chaves em claro; só a rotação das chaves nos providers resolve.
- Dependência criptográfica nova: revisar versão e manutenção.

## Validação

- `backend/scripts/verify-backend-gates.sh`; `backend/scripts/run-pg-integration-tests.sh` em PG 18 descartável com os testes novos no manifesto.

## Rollout / rollback

- Rollout: 1) migração de colunas + encoder + backfill no boot (cifra e anula `secret`); 2) depois de verificado, migração com `CHECK (secret IS NULL)`; 3) rotação das chaves nos providers. Nenhum deploy autorizado.
- Rollback: reverter o código não devolve o texto claro. O código antigo não lê linhas com `secret` nulo; o caminho é apagar as linhas cifradas e recadastrar as chaves pelo CRUD admin (são poucas: `typesafe`, `openai`, `nvidia`, `ngc`). Não existe ferramenta de decifrar para texto claro, de propósito.
