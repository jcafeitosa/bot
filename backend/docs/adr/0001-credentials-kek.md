---
title: ADR 0001 — origem da KEK das credenciais de provider
description: Proposta de onde vive a chave que protege as credenciais de provider cifradas no PostgreSQL (env, arquivo ou KMS)
tags:
  - adr
  - backend
  - security
  - credentials
  - wave0
status: draft
---

# ADR 0001 — origem da KEK das credenciais de provider

- **Status:** Proposed. Decisão do Julio (D5 em [master-plan](../planning/master-plan.md) §6). Este ADR não está aceito.
- **Fatia:** W0-07, SDD [wave0-07-credentials-at-rest-sdd](../sdd/wave0-07-credentials-at-rest-sdd.md).
- **Critérios de segurança:** SEC-CRED-01, 02, 03 e 08 em [provider-credentials-plaintext](../security/provider-credentials-plaintext.md).
- **Local deste arquivo:** não havia pasta nem tipo de ADR em `backend/docs` (nem em `.ok/config.yml`). `backend/docs/adr/` com numeração `NNNN-slug.md` é proposta; confirmar com o owner.

## Contexto

- As chaves de LLM ficam em texto claro em `provider_credentials.secret TEXT` (`0007_provider_credentials.sql`) e o modo é fixo `"none"` (`core/providers/credentials/mod.rs:5`).
- SEC-CRED-01 pede criptografia envelope: uma DEK por linha, cifrada ("wrapped") por uma KEK que fica **fora** do banco e do repositório (SEC-CRED-02).
- A decisão aqui é só **onde a KEK vive e como é trocada**. O algoritmo e o formato do blob estão no SDD.
- Hoje o deploy é um processo único num host (dev/testnet); não há infraestrutura de nuvem em uso pelo backend.

## Opções

Numeração própria (1–3) para não confundir com as letras de D5: 1 e 2 juntas são a opção A de D5; 3 é a opção B.

| | 1. Variável de ambiente | 2. Arquivo protegido no host | 3. KMS / secret manager externo |
|---|---|---|---|
| Como | `PROVIDER_CREDENTIALS_KEK` (32 bytes, base64) + `PROVIDER_CREDENTIALS_KEK_ID` | `PROVIDER_CREDENTIALS_KEK_FILE` aponta para arquivo fora do checkout, dono do processo, modo `0400` | DEK cifrada/decifrada por chamada ao KMS (ex.: AWS KMS, GCP KMS, Vault Transit) |
| Custo | Zero | Zero | Conta/serviço, rede, crate de cliente, credencial para o próprio KMS |
| Risco principal | Vaza em `/proc/<pid>/environ`, `env` de debug, dumps de crash, ferramentas de CI | Vaza em backup do host ou permissão errada | Indisponibilidade do KMS bloqueia o boot; credencial do KMS vira o novo segredo |
| Rotação | Subir com a KEK nova e a antiga (por `kek_id`), re-cifrar as DEKs, retirar a antiga | Igual à 1, com dois arquivos | Nativa (versões de chave no KMS) |
| Recuperação | Perder a KEK = perder as credenciais (recadastrar as chaves nos providers) | Igual à 1 | KMS guarda a chave; recuperação depende da política do KMS |
| Auditoria de uso | Nenhuma | Nenhuma além do SO | Log de cada uso da chave |

Opção descartada: **não guardar segredo no PG** (voltar a só env/arquivo). Resolve o texto claro no banco, mas desfaz o CRUD admin e o reload sem restart já entregues; é a opção C de D5.

## Recomendação

**1 ou 2 agora** (é a opção A de D5), com leitura das duas (o arquivo vence se as duas existirem; recomendo o arquivo em qualquer host compartilhado), e **blob versionado** para migrar para 3 (opção B de D5) depois sem reescrever os dados:

- cada linha guarda `version`, `alg`, `kek_id`, `nonce`, `wrapped_dek`, `secret_ciphertext` (SEC-CRED-01);
- `kek_id` identifica a KEK; um provedor de KEK por `kek_id` (local hoje, KMS depois) permite ter as duas durante a migração;
- migrar para 3 = registrar o provedor KMS, re-cifrar só as DEKs (`wrapped_dek`, `kek_id`), sem tocar em `secret_ciphertext`.

Motivos: menor dependência, nenhum serviço novo, compatível com o deploy atual; o formato não prende a escolha.

## Consequências

- Sem KEK (ou KEK errada) o processo não sobe clientes de provider e o CRUD responde **503** `provider_credentials_kek_unavailable`; nunca volta a texto claro (SEC-CRED-02).
- O operador passa a ter um segredo novo para guardar e fazer backup **separado** do backup do PG; entra no inventário de W1-03.
- Perder a KEK sem backup obriga recadastrar as chaves nos providers.

## Perguntas em aberto (owner)

1. Env (1), arquivo (2) ou os dois como padrão?
2. Local e numeração de ADR (`backend/docs/adr/NNNN-*.md`) estão ok?
