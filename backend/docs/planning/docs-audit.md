---
title: Auditoria completa de docs
description: Inventário físico e OpenKnowledge da documentação, conexões, duplicidades e artefatos
tags:
  - audit
  - documentation
  - backend
  - openknowledge
---

# Auditoria completa de `docs/`

> Revisão: 2026-09-27. A auditoria combina a árvore física de `backend/docs` com a validação do OpenKnowledge.

## Resultado

- Auditoria OpenKnowledge: **26 documentos, zero problemas de links**.
- Grafo: **zero links mortos e zero órfãos**.
- Conflitos rastreados: **zero**.
- Índice canônico: `backend/docs/index.md`.
- Categorias canônicas: arquitetura, operações, planejamento, referência, pesquisa e SDD.

## Estrutura canônica

- `architecture/`: visão de módulos, catálogo completo e integrações.
- `operations/`: runbook operacional.
- `planning/`: roadmap, plano de execução, lacunas de módulos e esta auditoria.
- `reference/`: CLI/configuração e matriz de testes.
- `research/`: pesquisa de capacidades de agentes.
- `sdd/`: especificações de design e correções.
- `external-sources/`: capturas preservadas de fontes externas.
- `AGENTS.md`: governança do projeto.
- `.codex/skills/graphify/`: skill operacional, mantida fora da documentação de produto.

## Artefatos verificados

### Fontes externas

As capturas preservadas de Grok Bot, OpenBot, OpenClaw e Meta Model API estão conectadas ao índice por links sob `external-sources/`. O HTML da Meta é um artefato de fonte preservada, não uma página de produto do backend.

### Duplicidade física detectada

Existe o arquivo físico:

```text
backend/docs/backend/docs/research/agents-capability-research.md
```

Ele é uma cópia complementar sem frontmatter, sem backlinks e com caminho duplicado `backend/docs/backend/docs`. O conteúdo canônico está em:

```text
backend/docs/research/agents-capability-research.md
```

A exclusão automática da cópia foi recusada pelo OpenKnowledge porque o caminho é ambíguo e poderia atingir a pesquisa canônica. O artefato permanece registrado como **pendência de limpeza física**. Não usar exclusão indireta ou ferramenta nativa para contorná-la; resolver por uma operação explícita de manutenção de caminho no OpenKnowledge.

## Correções aplicadas nesta auditoria

- Conectadas as fontes preservadas ao índice.
- Corrigidos os caminhos das fontes para a árvore canônica.
- Sincronizados os status de T-05, T-07, T-10 e T-15 no roadmap.
- Atualizada a contagem da documentação para 26 documentos.
- Mantidas as referências entre catálogo, matriz de testes, integrações, runbook, SDDs e planejamento.

## Critério de saúde

A documentação está saudável quando:

1. todo documento canônico aparece no índice ou é alcançado por um documento indexado;
2. não há links mortos;
3. não há órfãos;
4. SDD, roadmap, catálogo e matriz de testes têm o mesmo status;
5. fontes externas preservadas mantêm proveniência e não são confundidas com implementação;
6. cópias físicas fora da árvore canônica são removidas por operação autorizada e verificável.
