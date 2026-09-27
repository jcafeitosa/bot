---
title: SDD W0-14 — mask_secret sem panic e sem vazamento
description: Fatia pequena da Onda 0 que remove o pânico de mask_secret com entrada multibyte, que hoje imprime o segredo em stderr (F-CRED-08)
tags:
  - sdd
  - backend
  - security
  - credentials
  - wave0
status: draft
---

# SDD W0-14 — `mask_secret` sem panic e sem vazamento [SEGURANÇA]

- **Estado:** draft. Nenhum gate aprovado. Precisa de Critic independente (G1) e threat model do bot Segurança.
- **Plano:** W0-14 em [master-plan](../planning/master-plan.md) §4.1. Sem dependência de execução; G4 fecha depois de W0-02, porque o teste HTTP A5 pula calado sem PG até lá. Toca o mesmo arquivo que W0-07 ([wave0-07-credentials-at-rest-sdd](./wave0-07-credentials-at-rest-sdd.md)); ordem de implementação serializada, igual ao W0-07 L19: **W0-14 → W0-07a → W0-07b**.
- **Achado:** F-CRED-08 (Médio; Alto se stderr for para agregador compartilhado) e critério SEC-CRED-13 em [provider-credentials-plaintext](../security/provider-credentials-plaintext.md).

## Contexto (código lido em HEAD `d42b71a5`)

- `backend/src/core/providers/credentials/store.rs:11-21`:
  - `trim()`; vazio → `"****"`; `trimmed.len() <= 4` (bytes) → `"****"`;
  - senão `let suffix = &trimmed[trimmed.len() - 4..];` (L19) e `format!("****{suffix}")` (L20).
- A fatia é por **byte**. Se o índice `len - 4` cai dentro de um caractere multibyte, o Rust entra em pânico com "byte index N is not a char boundary; it is inside … of `<string>`", e a mensagem traz a própria string (truncada em 256 bytes). Não há panic hook nem `CatchPanicLayer` no crate, então isso vai para stderr.
- **Vetor que entra em pânico, derivado do código (L16-19):** o corte é o índice de byte `i = trimmed.len() - 4`, e só é alcançado se `trimmed.len() > 4`. O pânico acontece quando o byte `i` é um byte de continuação UTF-8 (`10xxxxxx`), ou seja, está no meio de um caractere de 2–4 bytes.
  - Vetor: `"sk-SECRETVALUE\u{200B}xyz"`. Bytes: `sk-SECRETVALUE` = 14 bytes ASCII (0–13); U+200B (espaço de largura zero) = 3 bytes `E2 80 8B` (14–16); `xyz` = 3 bytes (17–19). `len = 20`, `i = 16`.
  - O byte 16 é `8B`, terceiro byte de U+200B (que começa no 14). Não é limite de `char` (os limites vizinhos são 14 e 17), então `&trimmed[16..]` entra em pânico com "byte index 16 is not a char boundary; it is inside '\u{200b}' (bytes 14..17) of `sk-SECRETVALUE<U+200B>xyz`" (o U+200B aparece cru, invisível): a mensagem traz o segredo inteiro.
  - `trim()` não altera o vetor: U+200B não é espaço Unicode para `str::trim`, e está no meio, não na borda.
  - Regra geral: há pânico sempre que um caractere multibyte começa em um dos bytes `len-6 … len-5` (3 bytes) ou `len-7 … len-5` (4 bytes) ou em `len-5` (2 bytes), isto é, atravessa o índice `len-4`.
- **Por que o caso fixo `"abcdefgh\u{200B}"` (SEC-CRED-13) não entra em pânico:** `len = 8 + 3 = 11`, `i = 7`; o byte 7 é `h` (ASCII, limite de `char`), então a fatia é válida e devolve `h` + U+200B. O mesmo vale para `"abcdefgh\u{FEFF}"`. No threat model esse caso aparece na coluna de teste de SEC-CRED-13 como expectativa "sem pânico"; como ele já passa com o código atual, não serve de RED. A condição descrita em F-CRED-08 ("4º byte a partir do fim no meio de um caractere multibyte") está correta; só o exemplo não a satisfaz. Registrado para o bot Segurança acrescentar o vetor acima em SEC-CRED-13.
- Uma verificação descartável fora do repositório, feita antes desta revisão, confirmou a mensagem acima; o raciocínio vale sem ela.
- `str::trim` não remove U+200B nem U+FEFF, que costumam vir de copiar/colar.
- Efeitos: vazamento do segredo em log; `upsert` grava a linha e depois entra em pânico (`:139`); a listagem (`:107`) entra em pânico a cada chamada enquanto a linha existir.
- A máscara atual também expõe os 4 últimos bytes em toda resposta (F-CRED-05, Baixo).

## Seam público

- `pub fn mask_secret(secret: &str) -> String` em `store.rs:11`. O módulo `store` é privado (`credentials/mod.rs:8`) e `credentials/mod.rs:14-17` reexporta `delete`, `list_masked`, `upsert`, `ProviderCredentialMasked` e `ProviderCredentialsStoreError`, **não** `mask_secret`; na prática é interno ao crate. Por isso o RED unitário (A1) fica dentro de `store.rs`, e o seam público verificado é o HTTP (A5).
- Chamadores (`rg` + `graphify query "who calls mask_secret"`): `list_masked` (`store.rs:107`) e `upsert` (`store.rs:139`), mais o teste `mask_secret_never_returns_full_value` (`store.rs:171-174`). `list_masked` é chamado por `http_bridge/provider_credentials.rs:44-47` e pelo teste PG `pg_integration.rs:21`.
- O contrato visível é o campo `secret_masked` do DTO HTTP (`http_bridge/provider_credentials.rs:14`), devolvido por `POST /api/v1/admin/provider-credentials`, `PUT /api/v1/admin/provider-credentials/{provider_id}/{key_name}` e `GET /api/v1/admin/provider-credentials`. Testes HTTP esperam `"****e-xy"` para o segredo de fixture `"integration-test-secret-value-xy"` (`http_integration_tests.rs:1460, 1483, 1511`); o teste PG só exige que não contenha o segredo e contenha `****` (`pg_integration.rs:26-27`).
- **Decisão do owner `D-CRED-MASK-FORMAT`:** formato de `secret_masked` (contrato público do POST/PUT/GET admin). Precisa estar decidida antes do primeiro teste (A1 afirma a saída acordada). Opções e saídas esperadas abaixo.

## Alternativas

1. **Máscara fixa, sem expor nada:** sempre `"****"`, para qualquer entrada. Não há fatia, então não há pânico. Não vaza nada: hoje a saída revela se o segredo tem até 4 bytes (`"****"`) ou mais de 4 (`"****"` + sufixo), além dos 4 últimos bytes (F-CRED-05); com a alternativa 1 as duas coisas somem. Custo: o operador perde o sufixo para distinguir chaves; muda o valor do DTO, os dois asserts HTTP e a doc de `provider-credentials-db-sdd.md:56` (A4).
2. **Sufixo por `char`, com mínimo e sem caractere invisível.** Regra fixada aqui para a comparação:
   - Sobre o texto depois de `trim()`, se algum `char` estiver fora do ASCII visível (U+0021–U+007E), a saída é `"****"`. Isso cobre U+200B, U+FEFF, caracteres de controle, espaço interno e qualquer não-ASCII.
   - Senão, se a contagem de `char` for menor que **N = 12**, a saída é `"****"`.
   - Senão, a saída é `"****"` + os 4 últimos `char`. Como nesse ponto tudo é ASCII, 4 `char` = 4 bytes e não há corte dentro de caractere.
   - Mantém `"****e-xy"` para a fixture atual (32 `char` ASCII). Custo: continua expondo 4 caracteres e o fato de o segredo ter pelo menos 12 `char` ASCII visíveis.

### Saídas esperadas

| Entrada | Bytes / `char` | Hoje (HEAD `d42b71a5`) | Alternativa 1 | Alternativa 2 (N = 12, só ASCII visível) |
|---|---|---|---|---|
| `"sk-SECRETVALUE\u{200B}xyz"` (RED) | 20 / 18 | pânico (corte no byte 16, dentro de U+200B, bytes 14–16) | `"****"` | `"****"` (tem U+200B) |
| `"sk-SECRET\u{200B}ab"` (RED) | 14 / 12 | pânico (corte no byte 10, dentro de U+200B, bytes 9–11) | `"****"` | `"****"` (tem U+200B) |
| `"éabc"` (RED) | 5 / 4 | pânico (corte no byte 1, dentro de `é`, bytes 0–1) | `"****"` | `"****"` (tem não-ASCII; e 4 < 12) |
| `"abcdefgh\u{200B}"` (caso do TM, regressão) | 11 / 9 | `"****h\u{200B}"`, sem pânico (corte no byte 7, em `h`) | `"****"` | `"****"` (tem U+200B; e 9 < 12) |
| `"integration-test-secret-value-xy"` (fixture HTTP) | 32 / 32 | `"****e-xy"` | `"****"` | `"****e-xy"` |

## Recomendação

Alternativa 1 (máscara fixa). É a mais simples, não tem aritmética de índice e é a única que não vaza nada (nem sufixo, nem faixa de tamanho). Registrada como recomendação para `D-CRED-MASK-FORMAT`; a decisão é do owner. Se o owner quiser distinguir chaves na listagem, a saída futura é um fingerprint não reversível (SEC-CRED-10), não o sufixo do segredo.

- **Ordem no `upsert`:** hoje o `upsert` grava (`store.rs:121-132`), recarrega o cache (`:134`) e só então mascara (`:139`); um pânico na máscara deixa a linha gravada e o cliente sem resposta. Nesta fatia a máscara passa a ser calculada antes do INSERT (A6). Na alternativa 1 ela é constante, mas a ordem vale para qualquer formato.
- **Sem panic hook nesta fatia:** depois da correção, nenhum ponto conhecido do caminho de credenciais formata o segredo. `rg` em `core/providers/credentials/` e `http_bridge/provider_credentials.rs` não acha `format!`, `tracing`, `{:?}` nem fatiamento aplicados ao segredo (os que existem, `store.rs:57, 71` e `cache.rs:53-57`, formatam nome de campo, `provider_id` e nome de env var); o único outro fatiamento de texto sensível no caminho admin é `admin_auth.rs:135` (`trimmed[prefix.len()..]`), que só roda depois de `starts_with("Bearer ")`, prefixo ASCII de 7 bytes, então o índice é sempre limite de `char`. Os DTOs de entrada (`http_bridge/provider_credentials.rs:23-32`) derivam `Debug` com o campo `secret`, mas nada os formata hoje. A camada contra pânico fica como defesa em profundidade no SEC-CRED-13c.

### Divisão do SEC-CRED-13

No threat model ([provider-credentials-plaintext](../security/provider-credentials-plaintext.md) L66) o SEC-CRED-13 é um critério só: máscara sem pânico, 400 `invalid_secret_charset` no upsert e nenhum segredo em stderr (`CatchPanicLayer` com 500 genérico e teste que captura stderr). Este SDD o divide em três:

| Parte | Conteúdo | Dono | Por que separado |
|---|---|---|---|
| **13a** | `mask_secret` nunca entra em pânico, com as saídas acordadas em `D-CRED-MASK-FORMAT` | **W0-14** (esta fatia) | é a correção do F-CRED-08 |
| **13b** | `upsert` rejeita segredo com caractere de controle, formato ou não-ASCII com **400** `invalid_secret_charset`, sem ecoar o valor e sem gravar a linha | linha própria no master plan | código de erro público novo; precisa de decisão do owner |
| **13c** | panic hook ou `CatchPanicLayer` que responde 500 genérico sem payload nas rotas de segredo, com teste que captura stderr/tracing | linha própria no master plan | exige a feature `catch-panic` do `tower-http` (hoje `Cargo.toml:38` só tem `["trace"]`): mudança de dependência |

- 13b e 13c vão virar linhas próprias no master plan (atualização feita pelo coordenador, não por esta fatia).
- **Divergências para o bot Segurança** (sem editar o TM): (1) o TM trata o SEC-CRED-13 como critério único e o W0-07 (L19, L125) diz que o W0-14 é dono de tudo; este SDD só é dono do 13a. (2) O TM pede teste de propriedade com `proptest`, que não está no `Cargo.toml`; este SDD usa tabela ampla de entradas (A2), e `proptest` fica como acordo de dependência se alguém o quiser.

## Critérios de aceite

- A1. Teste RED primeiro, unitário, dentro de `store.rs`: afirma `mask_secret(v) == <saída acordada>` (tabela de saídas esperadas, alternativa escolhida em `D-CRED-MASK-FORMAT`) para `"sk-SECRETVALUE\u{200B}xyz"`, `"sk-SECRET\u{200B}ab"` e `"éabc"`. Com a função atual o teste falha por pânico (corte nos bytes 16, 10 e 1, dentro de caractere multibyte); depois da correção passa. Não é `#[should_panic]`: o teste descreve a saída correta, não o defeito. Não usar `"abcdefgh\u{200B}"` como RED, porque não entra em pânico (entra só como regressão em A2).
- A2. Depois da correção, nenhuma entrada gera pânico. Tabela ampla de entradas, sem `proptest`: vazia, só espaços, curta (1–4 bytes), exatamente 4 e 5 bytes, multibyte de 2, 3 e 4 bytes em cada posição que atravessa `len-4`, U+200B e U+FEFF no início, no meio e no fim, `"abcdefgh\u{200B}"` e `"abcdefgh\u{FEFF}"`, e segredo longo.
- A3. Para cada entrada de A1 e A2 a saída é exatamente a da regra da alternativa escolhida (na alternativa 1, sempre `"****"`); nenhuma saída contém caractere do segredo além do que a regra permite.
- A4. Testes e docs existentes atualizados para o formato acordado: `http_integration_tests.rs:1483` e `:1511` (hoje `"****e-xy"`, mudam se a alternativa 1 for escolhida), `pg_integration.rs:26-27` (continua valendo) e `provider-credentials-db-sdd.md:56` ("`****` + últimos 4 chars", muda na alternativa 1); gates verdes.
- A5. Seam público via HTTP: teste com `build_router` que faz `PUT /api/v1/admin/provider-credentials/typesafe/api_key` com `{"secret": "sk-SECRETVALUE\u200bxyz"}` e depois `GET /api/v1/admin/provider-credentials`; os dois respondem **200**, `secret_masked` é o formato acordado e nenhum dos corpos contém o segredo nem `SECRETVALUE`. Com W0-01 no ar, o teste usa token de fixture. Esse teste pula calado sem PG (`http_integration_tests.rs:1443-1446`, `let Some(db) = … else { return; }`), então só vale como evidência depois de W0-02; por isso o G4 desta fatia fecha depois de W0-02. Se o 13b for aprovado, o PUT desse vetor passa a responder 400 e o A5 é revisto na fatia do 13b.
- A6. No `upsert`, a máscara é calculada antes do INSERT (`store.rs:121-132`); nenhum caminho grava a linha e depois falha ao montar a resposta por causa da máscara.
- `&str` já garante UTF-8 válido; UTF-8 inválido não se aplica ao tipo atual.

## Riscos

- Mudança do valor de `secret_masked` pode quebrar quem compara o sufixo (hoje só os testes e a doc de `provider-credentials-db-sdd.md:56`).
- Pânico anterior já pode ter ido para logs: segredos que passaram por esse caminho devem ser tratados como expostos (rotação entra no runbook de W1-03).
- Sem 13c, um pânico futuro em outro ponto das rotas de segredo ainda iria para stderr; mitigado por não haver hoje ponto conhecido que formate o segredo.

## Validação

- Testes unitários de `mask_secret` (A1–A3), teste HTTP A5 e os testes HTTP/PG de credenciais (A4); `backend/scripts/verify-backend-gates.sh`. A5 só conta com PG obrigatório no gate (W0-02).

## Rollout / rollback

- Rollout: próximo build; sem migração e sem dado persistido novo.
- Rollback: reverter o commit volta ao pânico (reabre F-CRED-08); nenhum estado a desfazer.
