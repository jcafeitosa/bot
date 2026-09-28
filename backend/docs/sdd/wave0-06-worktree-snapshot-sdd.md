---
title: SDD W0-06 — snapshot seguro do working tree
description: Propõe snapshot completo e filtrado do working tree backend para testes isolados.
tags:
  - sdd
  - backend
  - security
  - testing
  - wave0
status: proposed
---
# SDD W0-06 — snapshot seguro do working tree

**Status:** PROPOSTO; exige G1 independente antes de implementação. Este desenho é um item separado do G1 já aprovado-com-follow-up do T-W0-06: amplia materialmente o contrato atual de snapshot somente de arquivos rastreados. Não autoriza implementação nem altera o seam pendente do addendum [ponto de entrada macOS](./wave0-06-macos-test-entrypoint-sdd.md).

## Contexto e problema

O wrapper prepara uma cópia isolada do backend antes de compilar e executar testes no container Linux pinado. O contrato atual enumera arquivos com `git ls-files` e copia seus conteúdos do working tree, logo inclui modificações em arquivos rastreados, mas omite arquivos novos não rastreados. O helper hoje também copia recursivamente o diretório do harness como exceção local. No working tree analisado há fonte Rust não rastreada necessária ao projeto, então o snapshot pode omitir código atual e ainda executar Cargo sobre uma árvore incompleta ou obsoleta. A cópia recursiva do harness inclui arquivos gerados não necessários, como `__pycache__`.

## Objetivo e seam

Definir um único snapshot determinístico do backend para o wrapper. O coletor deve unir arquivos rastreados e seus bytes atuais do working tree com arquivos não rastreados e não ignorados de `backend/` que sejam entradas de source, build, migration ou teste. O resultado deve representar as alterações locais sem incluir segredos, estado gerado, repositório Git ou ferramentas do host.

O seam público permanece `backend/scripts/verify-test-isolation.sh`; a política de snapshot pertence ao helper chamado por esse wrapper e não muda o comando de entrada proposto para macOS. O processo de teste continua sem checkout, `.env`, Docker socket ou rede externa.

## Contrato proposto do snapshot

- Enumerar arquivos rastreados, modificados ou não, e não rastreados não ignorados apenas dentro de `backend/`; copiar bytes atuais do working tree para ambos os grupos.
- Excluir explicitamente `.env`, `.env.*` e quaisquer diretórios de segredo; nunca abrir, imprimir ou propagar seus valores. Excluir também `.git`, `target`, `logs`, `graphify-out`, `docs/graphify-out/cache`, `__pycache__` e `.DS_Store` em qualquer profundidade.
- Não copiar recursivamente o harness `scripts/test-isolation`: seus arquivos de runtime já são incorporados à imagem pinada. Entradas do harness só entram se forem arquivos rastreados e explicitamente necessárias ao source snapshot.
- Copiar somente arquivos regulares dos tipos de entrada suportados (manifestos/lockfiles Cargo, código Rust, scripts de build/teste e migrations/configurações de teste necessárias). Symlinks, sockets, FIFOs, dispositivos, permissões ambíguas, caminhos fora de `backend/` e formatos não suportados abortam o snapshot antes de Cargo.
- Fazer descoberta e cópia de modo resistente a troca TOCTOU: validar caminho relativo e componentes, rejeitar symlink com `lstat`/abertura sem seguir links, conferir identidade/tipo do arquivo aberto e falhar se o conteúdo ou metadado mudar durante a captura. Manter o resultado read-only no container.
- Verificar após cópia que o conjunto de destinos corresponde exatamente ao manifesto filtrado e que arquivos excluídos não aparecem; qualquer divergência aborta antes de Cargo. Não imprimir conteúdo nem valores de sentinelas em logs.

## Alternativas e decisão proposta

1. **Somente arquivos rastreados (contrato G1 atual):** simples e previsível, mas deixa de fora fontes novas e não representa o working tree local; no checkout observado produz teste incompleto. Não é suficiente como evidência da versão que o desenvolvedor está editando.
2. **Working tree filtrado — recomendado para novo G1:** inclui modificações rastreadas e arquivos de implementação não rastreados, preservando exclusões estritas de segredo, estado gerado, symlink e artefatos de controle. Tem maior superfície de inventário, por isso toda entrada precisa ser validada e tipos desconhecidos devem falhar fechado.
3. **Exigir stage/commit antes do snapshot:** foi considerado, mas não recomendado como contrato principal; pode representar estado staged diferente do working tree, incentiva mudanças de índice apenas para testar e ainda exige uma regra para arquivos não staged. Continua como fallback operacional somente se o owner preferir deliberadamente testar o snapshot do índice em vez do working tree.

## Provas comportamentais G3

Escrever cada fixture primeiro e observar RED no helper atual, depois GREEN na implementação aprovada:

- arquivo `.rs` novo e não ignorado aparece no snapshot com bytes idênticos;
- alteração local a arquivo rastreado aparece com os bytes atuais do working tree;
- `.env` e `.env.local` de fixture contêm sentinela canário não secreta, e nenhum caminho/valor canário aparece no destino ou nos logs;
- diretórios `target`, `logs`, `graphify-out`, `docs/graphify-out/cache`, `__pycache__`, `.git` e `.DS_Store` ficam ausentes;
- symlink para arquivo dentro e fora da raiz é rejeitado antes de leitura/cópia; FIFO/socket/dispositivo e extensão não suportada também falham;
- manifest pós-cópia é exato; qualquer mudança concorrente entre descoberta e cópia aborta antes de iniciar Cargo;
- mock Docker confirma que snapshot e registry têm mounts read-only, Cargo/target têm mounts dedicados, sem build/pull implícitos, `.env`, checkout ou variáveis de opt-in herdadas;
- pré-requisitos de imagem/digest, arquitetura, versão do auditor e registry offline inválidos abortam sem executar Cargo/testes.

Essas fixtures usam arquivos temporários não secretos e mocks; não iniciam Docker, bancos, exchange, rede externa ou serviço.

## Riscos, rollout e rollback

O maior risco é incluir entrada não confiável do working tree; mitigam-no exclusões explícitas, enumeração backend-only, allowlist de tipos, validação de caminhos/arquivos e falha antes de Cargo. Exclusões excessivas podem omitir um novo tipo de build/teste, por isso o formato deve ser adicionado deliberadamente com fixture. Mudança concorrente ou ambiguidade de filesystem deve abortar, nunca recorrer ao checkout ou host.

Até G1 aprovado e fixtures G3 revisadas, não substituir a política existente nem alegar que o wrapper testa fontes não rastreadas. Após aprovação, habilitar o novo coletor no wrapper e exigir a evidência dos testes de snapshot antes do uso como gate. Para rollback, desabilitar o wrapper como fonte de aceite de um working tree incompleto; não voltar silenciosamente a compilar Cargo no host nem tratar o snapshot rastreado como equivalente.

## Gates e dependências

O G1 independente deve aprovar o contrato de working-tree, exclusões, allowlist de arquivos e defesa contra symlink/TOCTOU. G3 implementa com RED/GREEN comportamental no helper e no wrapper. G4 executa somente os testes unitários offline pelo entrypoint aceito depois dos gates; esta proposta não autoriza integração PG/Neo4j, Testnet, produção ou banco persistente. A decisão owner sobre wrapper oficial em macOS permanece separada e pendente conforme o addendum W0-06/macOS.