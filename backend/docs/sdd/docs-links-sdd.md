---
title: SDD D19 — Referências locais após reorganização da documentação
description: Correção de links relativos do backend após mover SDDs, plano e pesquisa
tags:
  - sdd
  - backend
  - documentation
---

# SDD D19 — Referências locais após reorganização da documentação

- **Autor:** Builder de documentação `/root/docs_links_builder`
- **Crítico:** `/root/docs_links_critic`, instância independente
- **Estado:** G1 aprovado por `/root/docs_links_critic`; G3 em revisão
- **Data:** 2026-09-26

## Contexto e objetivo

Uma reorganização externa moveu os SDDs para `backend/docs/sdd/`, o plano para `backend/docs/planning/` e a pesquisa para `backend/docs/research/`. O índice novo aponta para os destinos, mas o README ainda contém quatro links para os caminhos antigos de T-05, T-07, T-10 e T-15. Há também referências textuais a caminhos antigos em SDDs. C13 foi aprovado com o follow-up de corrigir o link de T-07 antes de G4.

**Objetivo:** fazer as referências locais de documentação apontarem para arquivos existentes, preservando o conteúdo técnico das entregas em andamento. O seam documental público é a navegação por links relativos a partir do arquivo que contém cada link. **Não objetivos:** mover arquivos, mudar comportamento de código, alterar a decisão dos SDDs ou reescrever o histórico das aprovações.

## Desenho e alternativas

1. Inventariar links Markdown locais em `backend/README.md` e `backend/docs/**/*.md`, além de menções textuais aos caminhos antigos `backend/docs/<nome>.md` e `docs/<nome>.md`.
2. Corrigir apenas referências atuais e instruções futuras com destino obsoleto. Preservar menções históricas explícitas quando o contexto pedir o caminho que existia no passado, mas esclarecer o destino atual se a frase puder orientar o leitor a abrir o arquivo.
3. Usar caminhos relativos ao arquivo de origem: `docs/sdd/` no README, `./sdd/` no índice e `../sdd/` no plano. Incluir o novo SDD D19 no índice.

| Alternativa | Decisão | Motivo |
|---|---|---|
| Desfazer a reorganização | Rejeitada | Invadiria o trabalho externo e manteria a navegação antiga. |
| Criar arquivos redirecionadores nos caminhos antigos | Rejeitada | Duplicaria conteúdo e deixaria caminhos legados como se fossem canônicos. |
| Atualizar referências na origem | Escolhida | Mantém um único destino por documento e diff pequeno. |

## Validação, risco e reversão

Este item não muda código executável e não tem teste red/green de produto. A falha observável inicial é o conjunto de links locais sem arquivo de destino; a condição green é conjunto vazio após a edição. Verificar todos os links Markdown locais no escopo contra o sistema de arquivos, buscar menções remanescentes aos caminhos antigos e executar `git diff --check`. O Critic inspeciona o diff e a contagem antes/depois. Não usar `cargo test` como prova de navegação documental.

O risco é sobrescrever edições simultâneas no README; a alteração deve ficar restrita à linha de links de SDD e ser reaplicada sobre o estado atual. O rollback é reverter apenas as linhas desta entrega; nenhuma migração ou deploy é necessário. Jev: não aplicável, porque resolução de caminhos é determinística.
