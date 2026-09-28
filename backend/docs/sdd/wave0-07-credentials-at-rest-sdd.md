---
title: SDD W0-07 — credenciais de provider cifradas no PostgreSQL (07a e 07b)
description: Delta da Onda 0 sobre provider-credentials-db-sdd, dividido em 07a (carregamento que falha fechado) e 07b (cifragem envelope, backfill por CLI e remoção do texto claro), com critérios SEC-CRED-01..13
tags:
  - sdd
  - backend
  - security
  - credentials
  - wave0
status: draft
---

# SDD W0-07 — credenciais de provider cifradas no PostgreSQL [SEGURANÇA]

- **Estado:** draft, ciclo 2 do G1 (ciclo 1: REPROVADO). Nenhum gate aprovado. Precisa de Critic independente (G1), revisão do bot Segurança contra [provider-credentials-plaintext](../security/provider-credentials-plaintext.md) e decisão do Julio sobre o [ADR 0001](../adr/0001-credentials-kek.md) antes do primeiro teste.
- **Plano:** W0-07 em [master-plan](../planning/master-plan.md) §4.1 e D5. **A fatia foi dividida em W0-07a e W0-07b** (seções abaixo, com aceites separados); o master plan precisa refletir isso.
- **SDD base:** [provider-credentials-db-sdd](./provider-credentials-db-sdd.md). Este arquivo é um **delta**: substitui a seção "Encryption-at-rest (follow-up, not this slice)" daquele SDD. Foi escrito em arquivo separado porque o SDD base tinha mudanças não commitadas de outra sessão no momento da redação. Quando o base estiver limpo, ele deve ganhar um link para cá.
- **Threat model:** do bot Segurança (F-CRED-01..08, SEC-CRED-01..13); não repetido aqui.
- **Fatia vizinha:** [wave0-14-mask-secret-panic-sdd](./wave0-14-mask-secret-panic-sdd.md) é dona de F-CRED-08 / SEC-CRED-13 e toca o mesmo arquivo (`store.rs`). Ordem de implementação serializada: **W0-14 → W0-07a → W0-07b**.

## Contexto (evidência no código, HEAD `d42b71a5`)

- `0007_provider_credentials.sql:6`: `secret TEXT NOT NULL`, em claro.
- `core/providers/credentials/mod.rs:5`: `PROVIDER_CREDENTIALS_ENCRYPTION_MODE = "none"`, exposto em `/meta` (`routes/meta.rs:52`) e `/healthz` (`routes/health.rs:56`).
- `core/providers/credentials/cache.rs`:
  - `:12` cache estático `Mutex<Option<HashMap<…, String>>>` com os segredos em claro;
  - `:14-31` `reload_from_pool` decodifica `secret` como `String`: uma linha com `secret` NULL faz o `SELECT` inteiro falhar;
  - `:41-63` `lookup_secret`: se o cache não tem a chave (inclusive cache nunca carregado), usa env (`TYPESAFE_API_KEY` etc.) sem dizer a origem.
- `core/database/postgres.rs:99-103`: `Database::migrate` chama `reload_from_pool` e, se falhar, só emite `warn` e devolve `Ok`.
- **Rollback hoje:** com `secret` NULL, o código atual falha no decode, o reload falha, `migrate` só avisa, o cache fica vazio e `lookup_secret` cai em silêncio para o env. Isso viola SEC-CRED-03 ("código antigo, ao encontrar `secret IS NULL`, falha fechado (503), sem fallback para env").
- `core/config/mod.rs:381-388`: `Environment::Prod` é recusado no `validate`; todo runtime é `Dev`. Portanto "fallback env só em dev" não restringe nada.
- `core/providers/credentials/store.rs`: listagem seleciona o segredo inteiro (`:90-97`); `upsert` (`:114-142`) e `delete` (`:144-164`) chamam `reload_from_pool` (`:134`, `:162`); `mask_secret` (`:11-21`) corta por byte (F-CRED-08, dono W0-14).
- `lookup_secret` é síncrono e chamado pela config de providers (`core/config/providers/file.rs:10-11, 65-66`).
- `Database::connect_from_url` (`postgres.rs:55`) é o ponto comum de quem abre o PG (`core/database/bundle.rs:52-54`, `monitor_bootstrap.rs`, CLIs).
- **Dependências já no `Cargo.lock`, só transitivas:** `ring 0.17.14` e `aws-lc-rs 1.18.1` (as duas pelos backends de `rustls`/`quinn-proto`), `zeroize 1.9.0`, `hkdf`, `hmac`, `sha2`. Nenhuma é dependência direta no `Cargo.toml`.

## Decisões comuns

1. **Envelope (SEC-CRED-01):** por linha, uma DEK aleatória de 256 bits cifra o segredo com AES-256-GCM. A DEK é cifrada ("wrap") pela KEK, também com AES-256-GCM. **Cada escrita (create ou replace) gera DEK nova e nonces novos** (96 bits, de `SystemRandom`); nunca se reusa nonce com a mesma chave.
2. **AAD canônico (C3 do TM), dois contextos separados.** `lp(x)` = `u32be(len(x))` ‖ `x`. Todos os campos são bytes UTF-8.
   - Segredo: `"bot/provider_credentials/v1"` ‖ `lp(provider_id)` ‖ `lp(key_name)` ‖ `u32be(version)` ‖ `lp(kek_id)` ‖ `lp(alg)`.
   - Wrap da DEK: `"bot/provider_credentials/dek-wrap/v1"` ‖ `lp(provider_id)` ‖ `lp(key_name)` ‖ `u32be(version)` ‖ `lp(kek_id)` ‖ `lp(alg)`.
   - Rótulos diferentes impedem usar um ciphertext no lugar do outro. Não há concatenação simples nem separador em texto.
   - **Vetor fixo** (`provider_id="openai"`, `key_name="api_key"`, `version=1`, `kek_id="k1"`, `alg="A256GCM"`), a versionar no repositório:
     - segredo (69 bytes): `626f742f70726f76696465725f63726564656e7469616c732f7631000000066f70656e6169000000076170695f6b657900000001000000026b31000000074132353647434d`
     - wrap (78 bytes): `626f742f70726f76696465725f63726564656e7469616c732f64656b2d777261702f7631000000066f70656e6169000000076170695f6b657900000001000000026b31000000074132353647434d`
   - **Par colidente:** (`provider_id="ab"`, `key_name="c"`) e (`"a"`, `"bc"`) dariam o mesmo `abc` numa concatenação simples. Com o formato acima (mesmos `version`, `kek_id` e `alg` do vetor), geram AAD diferentes:
     - (`ab`, `c`): `626f742f70726f76696465725f63726564656e7469616c732f7631000000026162000000016300000001000000026b31000000074132353647434d`
     - (`a`, `bc`): `626f742f70726f76696465725f63726564656e7469616c732f7631000000016100000002626300000001000000026b31000000074132353647434d`
3. **KEK** (ADR 0001, status `draft` (proposta)):
   - por env `PROVIDER_CREDENTIALS_KEK` (32 bytes em base64) + `PROVIDER_CREDENTIALS_KEK_ID`, ou por arquivo `PROVIDER_CREDENTIALS_KEK_FILE`. Definir env e arquivo juntos é erro de config.
   - **Lida e validada num único lugar:** `core::providers::credentials::kek::KekRing::load_from_env()` (módulo novo), chamada uma vez em `Database::connect_from_url` (`postgres.rs:55`), antes de conectar. O resultado fica num `OnceLock` do módulo `credentials`; ninguém mais lê essas variáveis.
   - `_FILE` exige arquivo regular com modo `0400` ou mais restrito (`mode & 0o377 == 0`); senão, falha com erro de config que cita o caminho, nunca o conteúdo.
   - KEK e DEK ficam em `zeroize::Zeroizing<[u8; 32]>`; o segredo decifrado no cache fica em tipo com `Debug` redigido e `Zeroizing<String>`.
   - **Rotação:** `PROVIDER_CREDENTIALS_KEK_PREVIOUS` (ou `…_PREVIOUS_FILE`) + `…_PREVIOUS_ID` serve **só para decifrar** (unwrap). Toda escrita usa a KEK atual. A variável anterior pode ser removida quando `bot provider-credentials verify-kek-retired --kek-id <antigo>` confirmar zero linhas com o `kek_id` antigo.
4. **Fingerprint da listagem (SEC-CRED-10):** HMAC-SHA256 truncado (8 bytes, hex) do segredo, calculado na escrita e guardado em coluna própria. A chave do HMAC é **derivada da KEK por HKDF-SHA256** com rótulo `"bot/provider_credentials/fingerprint/v1"`. Efeito na rotação: `rewrap` recalcula o fingerprint, então o valor exibido muda depois de trocar a KEK. Ele serve para distinguir linhas e detectar troca, não como id estável.
   - Alternativa registrada: chave de HMAC separada (`PROVIDER_CREDENTIALS_FINGERPRINT_KEY`). O fingerprint fica estável entre rotações, mas é um segredo a mais para guardar. Rejeitada agora.
5. **Fallback env:** com PG configurado, **nunca**; sem PG configurado (processo sem `DATABASE_URL`), o fallback continua como hoje. Isso substitui o "só em dev" e fica alinhado com a regra "sem KEK → 503, sem fallback para texto claro".
6. **Crates diretas:** `ring = "0.17"` (AES-256-GCM em `ring::aead`, HKDF, HMAC e `SystemRandom` numa crate só, já na árvore, sem build C) e `zeroize = "1"`. O Builder confirma com `cargo tree -i ring` que é a mesma versão já compilada. Limitação: o key schedule interno do `ring` não é zeroizado; os bytes crus da chave são.
   - Alternativa registrada: `aws-lc-rs` (API compatível com `ring` e opção FIPS), mas com build C (`aws-lc-sys`). Rejeitada agora; troca simples se FIPS for exigido.

**Alternativa considerada (arquitetura):** cifrar com a KEK direto, sem DEK por linha. Trocar a KEK obrigaria re-cifrar todos os segredos, e ir para KMS exigiria uma chamada ao KMS por leitura. Rejeitada.

## W0-07a — carregamento que falha fechado

**Objetivo:** antes de qualquer cifragem, garantir que "não consegui carregar credenciais do PG" nunca vira "uso o env em silêncio". É o que torna o rollback de 07b para 07a seguro.

- O cache passa a ter estado explícito: `NoPostgres` (env permitido), `Loaded(map)`, `Unavailable(motivo)`.
- `Database::connect_from_url` marca o estado como `Unavailable(not_loaded)` antes de conectar. Só um `reload_from_pool` bem-sucedido muda para `Loaded`. Processo que nunca chama `connect_from_url` fica em `NoPostgres`.
- `reload_from_pool` lê `secret` como `Option<String>`. Linha com `secret` NULL (cifrada por 07b, ilegível para 07a) → `Unavailable(unsupported_row_format)`.
- `Database::migrate` continua devolvendo `Ok` (o PG segue útil para os outros módulos), mas o cache fica `Unavailable` e o erro vai para o log sem valores.
- Com `Unavailable`: `lookup_secret` devolve "indisponível" (não consulta env); clientes de provider não sobem; rotas `/provider-credentials` respondem **503** `provider_credentials_unavailable`; `/meta` expõe `provider_credentials_state`.
- Migração na **próxima sequência livre depois da `0012` (reservada para W0-12)**: `ALTER TABLE provider_credentials ALTER COLUMN secret DROP NOT NULL`.

**Aceites 07a**

- A-07a-1. PG configurado + falha em `reload_from_pool` (injetada) → 503 `provider_credentials_unavailable` nas rotas e `lookup_secret` indisponível, **mesmo com `TYPESAFE_API_KEY` no env** (teste com env definido).
- A-07a-2. Linha com `secret` NULL → 503, sem env, sem pânico.
- A-07a-3. Sem PG configurado → env continua funcionando (comportamento atual).
- A-07a-4. `PROVIDER_CREDENTIALS_ENCRYPTION_MODE` continua `"none"` (07a não cifra).

## W0-07b — cifragem, backfill por CLI e remoção do texto claro

- **Migração 1** (próxima sequência livre depois da de 07a): colunas `version INT`, `alg TEXT`, `kek_id TEXT`, `nonce BYTEA`, `dek_nonce BYTEA`, `wrapped_dek BYTEA`, `secret_ciphertext BYTEA`, `fingerprint TEXT`, todas anuláveis.
- **Escrita:** todo create e replace grava só as colunas cifradas, com `secret = NULL`.
- **Leitura:** linha cifrada → decifra com a KEK do `kek_id` (atual ou anterior). Linha legada em claro (antes do backfill) → aceita, com `warn` e `/meta` `provider_credentials_encryption = "envelope-v1-pending-backfill"`.
- **Backfill por CLI explícita, não no boot:** `bot provider-credentials encrypt-existing` cifra as linhas em claro e anula `secret`, numa TX, de forma idempotente. Imprime só contagens.
- **Rotação:** `bot provider-credentials rewrap` re-cifra as DEKs (e recalcula o fingerprint) com a KEK atual; `verify-kek-retired --kek-id` conta linhas restantes.
- **Migração 2**, em release seguinte, depois do backfill verificado: `CHECK (secret IS NULL)`. Só então o modo passa a `"envelope-v1"`.
- **Sem KEK ou KEK errada (SEC-CRED-02):** clientes de provider não sobem; CRUD → **503** `provider_credentials_kek_unavailable`; nenhum fallback.
- **Listagem:** não seleciona `secret` nem ciphertext; devolve metadados + `fingerprint`. O formato de `secret_masked` é decidido em W0-14; 07b não o redefine.
- **Rollback 07b → 07a:** o down da migração 2 remove o `CHECK` e mantém o ciphertext; nunca decifra de volta para `secret`. O binário 07a encontra `secret` NULL e responde 503 (A-07a-2).
- **Rollback para antes de 07a depois do backfill: sem suporte.** O binário antigo lê `secret` como `String`, falha no decode, `migrate` só avisa (`postgres.rs:101-103`) e o cache cai no env em silêncio. Não há como impedir isso em código que já foi publicado; a regra entra no runbook de W1-03 ("nunca voltar para binário anterior a 07a depois do backfill").
- Depois do backfill, **toda chave existente é rotacionada no provider**, porque backups, WAL e réplicas antigos ainda têm o texto claro (SEC-CRED-03).

**Aceites 07b**

- A-07b-1. PG isolado, upsert de segredo-marcador: nenhuma coluna contém os bytes do marcador.
- A-07b-2. Trocar `provider_id`, `key_name`, `version`, `kek_id` ou `alg` da linha faz a decifragem falhar (segredo e wrap).
- A-07b-3. Vetor fixo de AAD (hex acima) igual ao produzido pelo código; par colidente gera AAD diferentes.
- A-07b-4. Duas escritas do mesmo segredo geram `nonce`, `dek_nonce` e `wrapped_dek` diferentes.
- A-07b-5. up → down → up em PG isolado: em nenhum momento alguma coluna contém o marcador em claro.
- A-07b-6. Binário na versão 07a contra linha cifrada → 503 (é o A-07a-2 rodado contra a tabela migrada por 07b).
- A-07b-7. KEK ausente, KEK errada e `_FILE` com modo `0644` → falha fechada (503 ou erro de boot), sem texto claro.
- A-07b-8. `rewrap` + `verify-kek-retired` → zero linhas com o `kek_id` antigo; depois disso, subir sem `_PREVIOUS` funciona.
- A-07b-9. `encrypt-existing` rodado duas vezes não muda nada na segunda.
- A-07b-10. `format!("{:?}")` dos tipos com segredo contém `<redacted>`; subscriber em `TRACE` não vê o marcador em upsert, list, reload e rewrap.

## Mapa de achados e critérios

| Critério | Onde | Observação |
|---|---|---|
| SEC-CRED-01 envelope + AAD | 07b | AAD canônico com rótulo, `u32be`, `kek_id` e `alg`; vetor e par colidente |
| SEC-CRED-02 KEK fora + 503 | 07b | A-07b-7 |
| SEC-CRED-03 migração sem rematerializar | 07a + 07b | 07a dá o 503 do código anterior; 07b faz backfill por CLI, down sem decifrar, rotação no provider |
| SEC-CRED-04 `Debug` redigido | 07b | A-07b-10 |
| SEC-CRED-05 logs sem segredo | 07b (teste) | checklist PG de log de parâmetros vai para W1-03 |
| SEC-CRED-06 auditoria append-only | 07b, parcial | evento sem segredo com `principal = "admin-token"` até P1 |
| SEC-CRED-07 autorização | fora | W0-01 (503 sem auth) e P1 |
| SEC-CRED-08 rotação + fallback | 07a + 07b | fallback env nunca com PG (07a); `rewrap` e `_PREVIOUS` só para decifrar (07b) |
| SEC-CRED-09 menor privilégio PG | 07b, parcial | teste `has_table_privilege` quando houver role de leitura; roles são ops (W1-03) |
| SEC-CRED-10 listagem sem segredo | 07b | fingerprint HMAC derivado por HKDF |
| SEC-CRED-11 chaves de exchange | fora | prod bloqueado; inventário em W1-03 |
| SEC-CRED-12 redação central | 07b, parcial | erros de store genéricos; redator de erro de exchange em fatia própria |
| **F-CRED-08 / SEC-CRED-13** pânico de `mask_secret` | **W0-14** | dono é [W0-14](./wave0-14-mask-secret-panic-sdd.md), com os vetores `"sk-SECRET\u{200B}ab"` (corte no byte 10, dentro de U+200B) e `"é"` + 3 ASCII (corte no byte 1, dentro de `é`). W0-07 não mexe em `mask_secret`; as duas fatias tocam `store.rs` e são serializadas (W0-14 antes) |

## Seams públicos para acordo antes do TDD

| Seam | Proposta |
|---|---|
| Estado do cache (07a) | `NoPostgres` \| `Loaded` \| `Unavailable(not_loaded \| reload_failed \| unsupported_row_format \| kek_unavailable)` |
| `lookup_secret` (07a) | passa a devolver `Result<Option<Zeroizing<String>>, CredentialsUnavailable>`; chamadores em `core/config/providers/file.rs` tratam `Err` como "provider não configurado", mantêm o valor zeroizável até o cliente e o boot falha fechado |
| Erros HTTP | 503 `provider_credentials_unavailable` (07a), 503 `provider_credentials_kek_unavailable` (07b) |
| Formato da linha (07b) | `version INT`, `alg TEXT` (`A256GCM`), `kek_id TEXT`, `nonce BYTEA` (12 bytes), `dek_nonce BYTEA` (12 bytes), `wrapped_dek BYTEA`, `secret_ciphertext BYTEA`, `fingerprint TEXT` |
| Origem da KEK | ver Decisões comuns, item 3 |
| Trait | `KekProvider { fn wrap(&self, dek, aad) -> Wrapped; fn unwrap(&self, kek_id, wrapped, aad) -> Result<Zeroizing<[u8; 32]>, KekError> }` (local hoje, KMS depois) |
| CLI (07b) | `bot provider-credentials encrypt-existing`, `rewrap`, `verify-kek-retired --kek-id <id>` |
| `/meta` e `/healthz` | `provider_credentials_state` (07a); `provider_credentials_encryption` = `none` → `envelope-v1-pending-backfill` → `envelope-v1` (07b) |

## Dependências

- W0-14 antes (mesmo arquivo `store.rs`).
- W0-01: SEC-CRED-07 e o CRUD sem auth dependem dele.
- W0-02: os testes PG precisam falhar alto sem PG.
- 07b depende de 07a e da decisão do ADR 0001. 07b habilita W1-03 (runbook, drill de rotação, regra de rollback).

## Riscos

- Perder a KEK = perder as credenciais cifradas; exige backup separado da KEK (ADR 0001).
- Rollback para antes de 07a depois do backfill não tem proteção em código (declarado sem suporte).
- Backups antigos do PG guardam as chaves em claro; só a rotação no provider resolve.
- A mudança de assinatura de `lookup_secret` atravessa a config de providers; conferir todos os chamadores (`rg -n "lookup_secret\("`).
- O down da migração 2 precisa de migração reversível; se o migrator do sqlx não aceitar misturar com as migrações simples existentes, o down vira script SQL testado no teste PG. O Builder confirma.

## Validação

- `backend/scripts/verify-backend-gates.sh`; `backend/scripts/run-pg-integration-tests.sh` em PG 18 descartável, com os testes novos no manifesto.

## Rollout / rollback

- Rollout, em releases separadas: 1) 07a; 2) 07b migração 1 + encoder + CLI; 3) `encrypt-existing` + verificação; 4) 07b migração 2 (`CHECK`); 5) rotação das chaves nos providers. Nenhum deploy autorizado.
- Rollback: 07b → 07a é suportado e falha fechado (503). Para antes de 07a, depois do backfill, sem suporte. Não existe ferramenta que decifre para texto claro na API; exportação em claro só por break-glass fora da API (SEC-CRED-03), fora desta fatia.

## Revisão G1 proposta — escopo de cifragem em repouso para dev local

**Estado:** proposta documental para G1, aguardando Critic independente e decisões do owner abaixo. O status draft do SDD e do ADR 0001 permanece. O Critic anterior bloqueou G1 por retorno lookup_secret não zeroizável e falta dos gates do runner PG; este WIP incorpora ambas as correções para novo review. Esta revisão limita a implementação proposta ao desenvolvimento local; não declara cifragem implementada, segredo configurado, banco validado ou G1 aprovado. As regras desta seção prevalecem para o escopo e gates desta revisão. Nenhuma execução com provider, DB ou credencial real faz parte dela.

### Sistema atual observado

- A migração 0007 define provider_credentials.secret como TEXT NOT NULL em claro.
- core/providers/credentials/cache.rs lê a coluna secret para um cache de String em processo; core/providers/credentials/store.rs lista e escreve esse campo. O CRUD HTTP está em presentation/http/routes/provider_credentials_admin.rs e modules/http_bridge/provider_credentials.rs.
- core/providers/credentials/mod.rs expõe PROVIDER_CREDENTIALS_ENCRYPTION_MODE = "none"; /meta e /healthz publicam esse valor. Não há encoder/decoder de aplicação, colunas ciphertext nem KEK loader no código observado.
- core/config/providers/file.rs obtém chaves para clientes Jev/NIM do cache e ainda tem fallback ambiental legado. O W0-07a deve preservar o contrato de falhar fechado quando PG foi configurado e a leitura do cache falha.
- O código e o SDD base confirmam que um backup/dump do PG atual contém segredos em claro. A presença de autenticação HTTP ou máscara na resposta não cifra o dado em disco.

O escopo desta revisão são as chaves de API dos providers configurados pelo CRUD local (typesafe/openai/nvidia/ngc). Chaves Binance, credenciais de produção, implementação de secret manager de produção e configuração real de .env ficam fora. Nenhum secret manager de produção está provisionado/aprovado para este backend; production permanece bloqueado e não pode anunciar modo cifrado nem aceitar credenciais de produção por este desenho.

### Objetivo e seams propostos

Para o operador de desenvolvimento local: armazenar credenciais de provider no PG local somente em ciphertext, protegendo dumps/cópias offline contra leitura sem a KEK. O operador deve poder iniciar o backend local depois de configurar a origem de KEK aprovada pelo owner. A proteção não cobre comprometimento do processo em execução, root/administrador local, memória, swap, logs antigos, WAL ou backups feitos antes de backfill/rotação.

O formato envelope e AAD descritos neste SDD continuam proposta técnica: DEK aleatória por registro; AEAD; AAD canônica incluindo provider_id, key_name, version, kek_id e alg; DEK wrapped por KEK fora do banco; nonce novo a cada escrita. O storage persiste apenas metadados e ciphertext depois do backfill. O contrato observável de status continua none antes da habilitação, envelope-v1-pending-backfill durante a transição e envelope-v1 somente depois de leitura, escrita e constraint cifradas estarem ativas.

| Seam público | Responsabilidade proposta |
|---|---|
| KekProvider / KekRing | carregar e selecionar chave por kek_id; wrap/unwrap de DEK; nenhuma chave vem do PG |
| Database::connect_from_url | PG/migrations e disponibilidade geral do banco não dependem da KEK; registrar KEK ausente/inválida como estado de credenciais indisponível, sem abortar o pool |
| credentials cache e lookup_secret | distinguir sem-PG de carregado e indisponível; retornar Result<Option<Zeroizing<String>>, CredentialsUnavailable>; manter o segredo em tipo zeroizável até a fronteira do cliente provider, sem clonar para String comum; não cair para env após falha de reload com PG configurado |
| credentials store | encrypt no create/replace, decrypt em lookup, listagem sem segredo/ciphertext; remoção somente após backfill verificado |
| provider lookup e CRUD | com KEK ausente/inválida ou ciphertext não decifrável, provider lookup e mutações de credencial retornam indisponível/503, sem fallback para plaintext ou env |
| /meta e /healthz | expor modo real; envelope-v1 só quando todas as leituras e escritas usam cifragem |
| CLI de manutenção | encrypt-existing idempotente, rewrap e verify-kek-retired; operações explícitas, transacionais quando necessário e com logs contendo apenas contagens |

**Precedência em falha da KEK:** falta, formato inválido, ambiguidade de fonte ou erro ao ler o arquivo não podem impedir conexão/migração do PG usado por outros módulos. O processo registra estado sanitized de KEK indisponível; rotas/provider clients dependentes de credencial cifrada respondem indisponível/503. Uma linha ciphertext sem KEK nunca usa fallback em claro. Não esconder a causa: erro estruturado e redigido fica observável, sem material de chave ou segredo. O comportamento de leitura de linhas legadas ainda em claro durante pending-backfill depende da decisão de rollout abaixo; não habilitar novos writes em claro após o primeiro rollout de cifragem.

Esta proposta supersede explicitamente a frase anterior em Decisões comuns, item 3, que chamava KekRing::load_from_env() antes de Database::connect_from_url. A ordem proposta é: conectar/migrar PG para os módulos gerais; carregar/validar KEK como estado separado; provider paths exigem KEK válida para operação cifrada. KEK inválida não aborta o pool geral; uma operação que precise de ciphertext sem a chave falha fechado em 503. A decisão de colunas/migrações permanece limitada ao esquema já descrito no SDD base e SEC-CRED-01/03; este amendment não escolhe nova migração, down migration ou numeração. Detalhes de tipos e códigos HTTP continuam sujeitos a G1 independente. A API pública de CRUD, autenticação e autorização segue os SDDs próprios; cifragem não substitui auth. Backfill nunca roda no boot nem em servidor da API.

### Decisões do owner ainda abertas

1. **Custódia local da KEK (ADR 0001 / D-SEC-KEK):** escolher env injetado pelo operador, arquivo protegido fora do checkout (modo 0400 ou mais restrito), ou outro mecanismo local que o owner aceite. O ADR atual recomenda env com arquivo opcional, mas está draft; esta revisão não adota essa recomendação como decisão. KMS/secret manager não está disponível no ambiente atual e não pode ser presumido. Critério comum: KEK fora do PG/repositório, fonte única e ambiguidade entre fontes causa erro. Sem KEK válida, nenhum fallback para texto claro.
2. **Charset do segredo (SEC-CRED-13b):** aceitar ASCII visível somente (recomendação do Segurança) ou manter UTF-8 válido integral. É mudança de contrato de entrada com HTTP 400 novo; owner decide antes do teste/implementação. Em ambas, nenhum valor rejeitado pode aparecer em response, log ou audit.
3. **Máscara HTTP (D-CRED-MASK-FORMAT / W0-14):** confirmar **** fixo ou máscara com sufixo. Este SDD não escolhe resposta pública; formato não altera ciphertext nem autorização.
4. **Ponto irreversível do backfill (SEC-CRED-03):** owner confirma antes de executar encrypt-existing que binários anteriores a 07a não serão usados após linhas ficarem secret=NULL/ciphertext-only, que todas as chaves já persistidas serão rotacionadas nos providers e que backups/WAL antigos serão retidos ou destruídos segundo política explícita. Não haverá rollback que decifre ciphertext para secret em claro. Down migration mantém ciphertext e a versão anterior compatível falha fechado.
5. **Linhas legadas no modo pending:** decidir se lookup pode usar uma linha ainda em claro até o backfill, ou se provider fica indisponível até o operador configurar KEK e concluir a migração. Em qualquer opção, writes são bloqueados sem KEK e env não pode substituir uma linha PG ou ciphertext.
6. **Resíduo em memória:** aceitar explicitamente que ring mantém key schedule interno que não é zeroizado, ou solicitar revisão/alternativa criptográfica que dê garantias diferentes. Bytes crus de KEK/DEK, plaintext temporário e cache devem usar tipos zeroizáveis e escopo curto; ainda assim isso não promete apagar cópias internas da biblioteca, allocator ou swap. A decisão técnica não deve ser apresentada como garantia total de zeroization.

Itens 1–5 bloqueiam implementação de 07b/backfill até decisão registrada. Item 6 requer avaliação do Critic de segurança; implementação não pode afirmar proteção contra comprometimento de memória.

### Alternativas consideradas

- **Manter somente env e remover CRUD/PG:** evita ciphertext local, mas abandona credenciais no banco e reload sem restart já usados; pode ser proposta separada, não a escolha tácita desta revisão.
- **KEK local em env:** configuração simples, com risco de exposição por inspeção de processo/crash dump; aceitável somente com fonte local aprovada e sem valor em logs.
- **Arquivo protegido fora do checkout:** reduz herança ambiental, exige controle de owner/mode e procedimento local de backup. É opção ao owner, não padrão decidido aqui.
- **Secret manager/KMS:** boa separação operacional e auditoria, mas requer conta/serviço, cliente e autoridade de acesso. Não há secret manager de produção no backend; adiar implementação cloud para desenho e aprovação próprios.
- **Criptografia só do volume/disco:** reduz acesso a mídia offline mas não evita que dumps PG revelem a coluna nem atende ao contrato de envelope no banco; não substitui a proposta.

### TDD e validação por fases

- **G3 offline:** fake KekProvider permite teste RED/GREEN de AAD, wrap/unwrap, alteração de metadado, nonce novo, chave errada/ausente, redaction de Debug/logs, estado de cache e ausência de fallback quando PG configurado. Fixtures são valores sintéticos marcados como teste. Testes de charset e máscara aguardam decisões 2 e 3.
- **G4 PG local descartável:** só depois de G1 aprovado, G3 do T-W0-06b e do W0-02b aprovados, autorização operacional e runner PG isolado disponível. O runner valida target efêmero e identidade do container/database pelo manifest; fornece exclusivamente BOT_PG_TEST_DATABASE_URL via handoff privado; helper valida marker antes de SQLx; cada teste emite o marcador de conclusão somente depois das assertions. É proibido executar testes PG por cargo test direto ou usar DATABASE_URL como seletor/fallback. Então upsert de valor sintético seguido de inspeção da linha prova que nenhuma coluna guarda o marcador em claro; reinício/reload recupera o mesmo valor via fake provider; tamper em ciphertext/AAD/KEK falha fechado; listagem não busca ciphertext; encrypt-existing repetido é idempotente; rotação/rewrap e verify-kek-retired mostram zero linhas com kek_id anterior.
- **Negativos obrigatórios:** KEK ausente, encoding/tamanho inválido, fonte env+file simultânea, permissão de arquivo frouxa, decrypt/tag inválido, metadata alterada, reload falho com env legado presente, cache não carregado com PG, e tentativa de iniciar cliente sem credencial decifrável. Cada caso deve retornar erro definido sem plaintext em log/HTTP/audit. Teste da precedência verifica que falha de KEK deixa PG disponível para módulos não relacionados e que operações de provider falham com 503 sem connector fallback.
- **G5 não pertence a esta proposta:** nenhuma implantação, migração de banco persistente, rotação de provider externo ou configuração de produção é autorizada aqui. O G4 descrito é apenas o caminho futuro para banco local descartável, após gates.

### Rollout e rollback propostos

1. 07a primeiro: cache tem estados explícitos; com PG, reload falho/linha NULL resulta indisponível e sem fallback env. 07a não cifra e modo continua none.
2. 07b código e migration aditiva: encoder/decoder e testes com KEK sintética; modo permanece pending até backfill.
3. Operador configura KEK local aprovada e testa restauração/backup; depois de owner confirmar irreversibilidade, encrypt-existing cifra em transação idempotente e imprime somente contagens.
4. Verificação de todas as linhas sem plaintext, leitura de cada linha, rotação de chaves nos providers e política de retenção de backup/WAL antigos. Só então migration/constraint final remove o caminho em claro e modo pode anunciar envelope-v1.
5. Rewrap de DEKs antes de retirar KEK anterior; verify-kek-retired comprova zero linhas antes de apagar a chave antiga.

Rollback de aplicação após backfill volta no máximo a uma versão compatível com 07a, que falha fechado sobre ciphertext-only. Não há rollback para binário anterior a 07a nem exportação automática de segredo em claro. Se verificação falhar, interromper rollout, manter ciphertext/KEK e modo pending; recuperar pela versão compatível, sem decifrar para coluna em claro.

**Gates para sair de draft:** ADR 0001 e decisões owner 1–5 resolvidas; decisão do Critic de segurança sobre zeroization; seam lookup_secret zeroizável aprovado; W0-14 ordenado antes de 07a por compartilhar store.rs; G1 independente aprovado; então dividir em 07a/07b com plano e critérios. G4 PG segue bloqueado até aprovação G3 do runner T-W0-06b e do guard/completion protocol W0-02b. Produção continua bloqueada até um projeto aprovado de secret manager/custódia e rollout independente.
