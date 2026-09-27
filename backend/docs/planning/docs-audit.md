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

## Auditoria do backend (2026-09-27)

- OpenKnowledge auditou 50 documentos e encontrou zero links quebrados; um primeiro escopo explicitado incorretamente foi incompleto e não foi contado como resultado.
- Graphify foi atualizado após identificar grafo desatualizado; a extração cobre código suportado, mas deixou 10 arquivos SQL de fora porque `tree_sitter_sql` não está instalado.
- `./scripts/verify-backend-gates.sh` **falha ao compilar** no snapshot auditado: falta declarar/reexportar `monitor_bootstrap` em `src/core/database/mod.rs`; `provider_credentials_admin.rs` chama `ApiState::with_admin`, que não existe; imports não usados também aparecem sob `-D warnings`.
- Parecer de segurança: `BOT_HTTP_ADMIN_TOKEN` é opcional por contrato. Quando ausente, rotas mutantes permitem requests sem bearer; isso está explicitamente documentado como comportamento dev/local, mas implantação exposta precisa configurar o token ou restringir o listener por rede.
- Risco alto de idempotência live: `submit_order_http` executa primeiro e grava `client_order_id` em PostgreSQL depois. Uma falha na gravação pode devolver erro após aceitação do executor; um retry pode repetir a ordem. A chamada ao executor e o banco não formam uma transação distribuída.
- Risco médio de integridade do dataset: `Database::persist_dataset` usa `ON CONFLICT DO NOTHING` no manifesto e nas candles e retorna sucesso sem conferir que as linhas existentes correspondem ao conteúdo recebido.
- O gate padrão não executa os 15 testes PostgreSQL ignorados; `run-pg-integration-tests.sh` apenas avisa quando o nome na URL não contém `trading_bot` e prossegue. A aplicação rejeita outros databases, mas o script pode confundir alvo de execução.
- A contagem do README/artefatos precisa sincronização: o script enumera 15 testes PG, enquanto a documentação da auditoria/registros anteriores contém contagens diferentes em alguns trechos. Atualizar números apenas após um run real.
- As rotas `/api/v1/admin/provider-credentials*` estão registradas no router/OpenAPI, mas são stubs 501; a documentação operacional e SDD dizem isso corretamente. `provider_credentials.secret` fica em texto no banco; criptografia em repouso está registrada como follow-up, portanto exige controle de acesso e backup protegidos.
- O seed SQL de exemplo executa upsert substituindo valores existentes; o arquivo alerta para não executar literalmente no CI e usar placeholders locais.
- Cobertura auditada: 709 arquivos Git sob `backend` (inclui 104 arquivos vendor e 309 artefatos Graphify); código de aplicação e documentos foram analisados por categorias. Vendor não foi revisado linha por linha, e Graphify não analisou as migrations SQL.
- Nenhum arquivo de produto foi alterado pela auditoria; alterações locais preexistentes foram preservadas. Gate foi capturado em `{SCRATCH}`.
