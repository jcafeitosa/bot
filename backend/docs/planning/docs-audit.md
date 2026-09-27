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

- Auditoria OpenKnowledge: **50 documentos, zero problemas de links**.
- Grafo: **zero links mortos e zero órfãos**.
- Conflitos rastreados: **zero**.
- Índice canônico: `backend/docs/index.md`.
- Categorias canônicas: arquitetura, operações, planejamento, propostas, referência, pesquisa, fontes externas e SDD.

## Estrutura canônica

- `architecture/`: visão de módulos, catálogo completo e integrações.
- `operations/`: runbook operacional.
- `planning/`: roadmap, plano de execução, auditorias e lacunas de módulos.
- `proposals/`: propostas arquiteturais ainda sujeitas a decisão.
- `reference/`: CLI/configuração e matriz de testes.
- `research/`: pesquisa de capacidades de agentes.
- `sdd/`: especificações de design, gates e correções.
- `.codex/skills/graphify/`: skill operacional, mantida fora da documentação de produto.
- `external-sources/`: capturas preservadas de fontes externas.
- `AGENTS.md`: governança do projeto.
- `.codex/skills/graphify/`: skill operacional, mantida fora da documentação de produto.

## Artefatos verificados

### Fontes externas

As capturas preservadas de Grok Bot, OpenBot, OpenClaw e Meta Model API estão conectadas ao índice por links sob `external-sources/`. O HTML da Meta é um artefato de fonte preservada, não uma página de produto do backend.

### Duplicidade física (resolvida)

A árvore acidental `backend/docs/backend/docs/` (cópias de pesquisa, proposta, fontes externas e HTML duplicado da Meta) foi removida. As capturas Grok/OpenBot/OpenClaw foram consolidadas em `backend/docs/external-sources/`; links do índice apontam para essa pasta canônica.

## Correções aplicadas nesta auditoria

- Conectadas as fontes preservadas ao índice.
- Corrigidos os caminhos das fontes para a árvore canônica.
- Sincronizados os status de T-05, T-07, T-10 e T-15 no roadmap.
- Atualizada a contagem da documentação para 50 documentos.
- Corrigidas sete referências quebradas em README, scripts, `.env`, providers e SDD HTTP admin.
- Confirmada a remoção da árvore duplicada `backend/docs/backend/docs/`.
- Mantidas as referências entre catálogo, matriz de testes, integrações, runbook, SDDs e planejamento.

## Critério de saúde

A documentação está saudável quando:

1. todo documento canônico aparece no índice ou é alcançado por um documento indexado;
2. não há links mortos;
3. não há órfãos;
4. SDD, roadmap, catálogo e matriz de testes têm o mesmo status;
5. fontes externas preservadas mantêm proveniência e não são confundidas com implementação;
6. cópias físicas fora da árvore canônica são removidas por operação autorizada e verificável.
