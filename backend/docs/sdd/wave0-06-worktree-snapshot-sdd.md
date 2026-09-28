---
title: SDD W0-06 — snapshot seguro do working tree
description: Proposta de allowlist e captura estável do working tree backend para testes isolados.
tags:
  - sdd
  - backend
  - security
  - testing
  - wave0
status: proposed
---
# SDD W0-06 — snapshot seguro do working tree

**Status:** PROPOSTO — ciclo G1 anterior encontrou três blockers; esta revisão os endereça, mas aguarda re-review G1 independente. Não implementar nem executar até aprovação. Este item altera materialmente o contrato de snapshot rastreado do T-W0-06, cujo G1 APROVADO COM FOLLOW-UP (`1d215962826470071a9954ec2885035e22959bae`) permanece histórico vigente para os demais seams; esta proposta não o amplia por si só.

## Contexto e problema

O wrapper compila e executa dentro do container Linux pinado a partir de snapshot isolado. O coletor atual enumera arquivos com `git ls-files`, então usa bytes do working tree para arquivos rastreados, mas omite fontes novas não rastreadas. A exceção que copia recursivamente `scripts/test-isolation` pode incluir harness e `__pycache__` não necessários. No checkout analisado há fonte não rastreada necessária; um snapshot incompleto pode compilar uma árvore diferente daquela em edição.

## Objetivo e seam

Produzir, antes de Cargo, snapshot determinístico que represente alterações rastreadas e entradas novas não ignoradas do backend, sem copiar credenciais, estado gerado, harness do host ou arquivos fora do escopo. O seam operacional continua sendo o helper chamado por `backend/scripts/verify-test-isolation.sh`; não se propõe uma rota de execução nova.

## Allowlist e precedência

A seleção usa regras estáticas e versionadas pelo projeto, nunca uma regra genérica como `**/*.toml`, `**/*.json`, `**/*config*` ou “copiar tudo que Cargo mencionar”. Um path só entra quando uma regra positiva explícita o seleciona e todos os testes de canário daquela classe passam.

Precedência normativa, da primeira condição à última:

1. **Negativas absolutas:** excluir antes de avaliar qualquer allowlist `.env` e `.env.*` em qualquer diretório; `.git`; `target`; `logs`; `graphify-out`; `docs/graphify-out/cache`; `__pycache__`; `.DS_Store`; `scripts/test-isolation` (harness já está baked na imagem); nomes contendo `credential`, `secret` ou `private-key`; extensões `.pem`, `.key`, `.p12` e `.pfx`. Nenhuma regra positiva pode reintroduzir path negado.
2. **Validação do objeto:** caminho relativo deve permanecer abaixo de `backend/`; rejeitar symlink em qualquer componente, arquivos não regulares, hardlink com identidade ambígua, socket/FIFO/dispositivo, permissões ou tipo desconhecido. Rejeitar também nomes/case-collisions que não sejam reproduzíveis na plataforma do container.
3. **Allowlist positiva exata:**

| Classe permitida | Seletor de path e tipo | Canário positivo obrigatório | Canário negativo de precedência |
|---|---|---|---|
| Manifest/lock raiz | `Cargo.toml` e `Cargo.lock`, nomes exatos e arquivos regulares | fixture de cada arquivo aparece byte-a-byte | `.env` e `Cargo.local.toml` adjacentes permanecem ausentes |
| Manifest de crate vendorizada | `vendor/**/Cargo.toml` apenas sob diretório de crate Rust | manifest canário em crate vendorizada entra | `.env.private` no mesmo diretório é excluído |
| Fonte/build Rust | `build.rs` em raiz de crate e `src/**/*.rs`, `tests/**/*.rs`, `benches/**/*.rs`, `examples/**/*.rs`, `vendor/**/*.rs` | um arquivo alterado/novo por cada raiz incluída entra com bytes atuais | `.env.test` e arquivo `.key` sob raiz permitida permanecem ausentes |
| Migration | `migrations/**/*.sql` | migration canário dentro da raiz entra | `.env.local` e `private-key.sql` permanecem ausentes |
| Fixture de teste JSON | `tests/fixtures/**/*.json` somente; JSON de configuração fora desta raiz não entra | fixture não sensível sob a raiz entra | `.env` e arquivo JSON fora de `tests/fixtures/` não entram |

Essas classes são o conjunto inicial. Qualquer nova extensão ou raiz (inclusive TOML/JSON/YAML de configuração, scripts, certificados ou fixture fora dos seletores acima) exige alteração explícita do SDD/allowlist e canários positivos e negativos próprios antes de ser usada. Arquivos de configuração genéricos não entram; não inferir segurança a partir de extensão. Conteúdo/canários nunca devem conter ou imprimir credenciais reais.

## Captura segura e estabilidade

Descoberta, abertura e cópia são relativas a file descriptors. Abrir a raiz `backend/` como diretório confiável; percorrer cada componente com `openat`/equivalente relativo ao descritor pai, `O_DIRECTORY|O_NOFOLLOW`, validar `fstat` e comparar identidade com `fstatat(..., AT_SYMLINK_NOFOLLOW)`. Não concatenar path e reabrir pelo caminho absoluto. Abrir o arquivo final com `O_NOFOLLOW|O_CLOEXEC`, confirmar que é arquivo regular e que device/inode/tipo coincidem com a identidade enumerada; copiar os bytes lidos daquele mesmo descritor validado para arquivo temporário novo no destino.

Manter os descritores de diretório/arquivo abertos até publicar o snapshot. Registrar `fstat` inicial (device, inode, modo, nlink, tamanho, mtime e ctime com precisão disponível); ler até EOF do descritor validado, copiar esses bytes para arquivo temporário criado exclusivamente e calcular contagem/digest. Antes de aceitar, reler o conteúdo desde offset zero do mesmo descritor e comparar contagem e digest com o primeiro stream e o temporário; repetir `fstat` e exigir identidade e metadados estáveis. Revalidar pelo descritor pai que cada nome ainda aponta para a identidade do diretório/arquivo aberto. Se o filesystem não fornece identidade/estabilidade suficiente, se qualquer metadado mudar, leituras divergirem, ou não for possível completar/verificar o segundo passe, descartar todo o destino temporário e abortar antes de Cargo. Nunca retentar via path-following, checkout completo ou host.

A publicação do snapshot é atômica: só tornar o diretório disponível ao wrapper depois de comparar o inventário final com o manifesto filtrado. O manifesto registra paths, tipo, tamanho e digest, sem dados de arquivo sensível. Logs mostram somente classe/código de erro e path relativo sanitizado, nunca bytes ou sentinelas.

## Achados do G1 anterior e resposta desta revisão

- **Allowlist vaga e exclusões sem precedência:** substituída por classes estáticas de path/tipo, exclusões absolutas antes da inclusão e canários positivos/negativos por formato/raiz; configuração genérica explicitamente excluída.
- **TOCTOU na travessia/cópia:** definido percurso por descritor sem seguir symlink em cada componente, validação de identidade, cópia do mesmo file descriptor, protocolo de estabilidade e abort fechado. Acrescentadas fixtures de substituição de componente e mutação do arquivo aberto.
- **Contradição com addendum macOS:** o addendum [W0-06/macOS](./wave0-06-macos-test-entrypoint-sdd.md) mantém seam oficial pendente da decisão do owner. Esta proposta não escolhe nem aprova esse seam. Ela apenas especifica como representar working tree no wrapper se a rota for aprovada; a exceção de plataforma macOS continua condicional ao aceite do owner. O target runner Linux aprovado pelo T-W0-06 não muda.

## Alternativas e trade-offs

1. **Somente rastreados:** simples, mas omite fontes novas e não representa o working tree ativo; não serve para evidência de código local não staged.
2. **Working tree com allowlist — recomendado para novo G1:** representa código alterado/novo e exclui dados não pertinentes; custa mais validação e requer canários a cada classe.
3. **Exigir stage/commit:** reduz enumeração, mas pode testar estado staged diferente do working tree e não resolve arquivos unstaged; só adotar por decisão explícita do owner.

## Provas comportamentais G3

Sem iniciar Cargo até o snapshot passar:

- fonte Rust não rastreada e modificada rastreada entram com o conteúdo atual;
- para cada classe da allowlist, canário positivo entra e canários de exclusão (incluindo `.env*` nas mesmas raízes) não entram; formatos/path fora da tabela são recusados ou excluídos conforme regra;
- diretórios e harness excluídos permanecem ausentes;
- symlink interno/externo falha sem seguir; substituir adversarialmente componente de diretório entre enumeração e `openat` aborta com zero comando Cargo; substituir arquivo após enumeração não copia o novo objeto por path;
- alterar/truncar/regravar o mesmo inode enquanto o descritor está sendo copiado causa falha de estabilidade e zero comando Cargo;
- sucesso e erro não expõem valores canário nos logs; o snapshot publicado corresponde exatamente ao manifesto;
- mock do wrapper confirma pre-req inválido e qualquer falha de snapshot causam zero Cargo/test commands.

Fixtures são temporárias, têm apenas sentinelas não secretas e não acessam Docker, DB, exchange ou rede.

## Riscos, rollout e rollback

Allowlist estreita pode omitir novo formato de teste; inclusão é deliberada e exige canários. Inventário do working tree não confiável pode conter troca concorrente; descritores fixos, comparação de identidade e abort em mudança mitigam a corrida. Nenhuma implementação pode enfraquecer exclusões para permitir que uma fixture passe.

Até G1 aprovado e fixtures G3 revisadas, manter explicitamente o coletor atual como estado histórico e não alegar que o wrapper verificou fontes não rastreadas. Após aprovação, implementar RED/GREEN no coletor e somente então aceitar evidência do wrapper para o working tree. Se qualquer proof de estabilidade não estiver disponível numa plataforma, falhar fechado antes de Cargo; rollback suspende o aceite pelo wrapper em vez de recorrer ao snapshot incompleto ou ao host.

## Gate e dependências

Status atual: proposta revisada após blockers G1, re-review independente ciclo 2 pendente; sem autorização de implementação. A aprovação existente de T-W0-06 continua cobrindo o desenho anterior e demais seams, mas não esta expansão do snapshot. O addendum macOS segue separado e aguarda decisão do owner sobre o entrypoint. Nenhuma etapa aqui autoriza PG/Neo4j, Binance, produção ou banco persistente.