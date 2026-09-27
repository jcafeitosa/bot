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

- OpenKnowledge auditou **51 documentos** depois da atualização desta auditoria; nenhum link quebrado foi encontrado. A auditoria de lint individual retornou `ran: []`, portanto só há evidência de links.
- Graphify foi atualizado após identificar grafo desatualizado. A extração Rust foi refeita; 10 arquivos SQL ficaram fora do grafo porque `tree_sitter_sql` não está instalado.
- Gate final `./scripts/verify-backend-gates.sh`: **passou** — 389 testes do binário aprovados, 17 ignorados e as cinco suítes de integração do script aprovadas. A primeira execução durante atualização concorrente falhou por imports ausentes; a segunda execução, no snapshot atual, passou. Log: `/var/folders/hx/gqkw19cj03981h7xryfqk0c80000gn/T/grok-goal-63232b08cc11/implementer/backend-audit-gates-current.log`.
- Parecer de segurança: `BOT_HTTP_ADMIN_TOKEN` é opcional por contrato. Sem token, as rotas mutantes permitem requests sem bearer; isso está documentado como comportamento dev/local, mas deployments acessíveis por rede precisam configurar o token ou restringir o listener.
- **Alto — idempotência de ordem pode falhar depois da execução** (`src/presentation/http/state.rs:425-433`): o port recebe e executa a ordem antes de gravar `client_order_id` no PostgreSQL. Se a gravação falhar, a chamada retorna erro depois da aceitação; retry pode executar novamente. A transação PostgreSQL não cobre o efeito externo.
- **Médio — persistência de dataset pode confirmar conteúdo divergente** (`src/core/persistence/mod.rs:51-82`): manifesto e candles usam `ON CONFLICT DO NOTHING`; não há comparação de valores existentes antes de retornar sucesso.
- **Médio — falha de persistência é classificada como entrada inválida** (`src/modules/orders/adapters/pg_idempotency.rs:24-41` + `src/presentation/http/error.rs:184-189`): falhas de lookup/gravação PG viram `OrdersError::InvalidRequest`/HTTP 400, apesar de serem falhas de dependência; no cenário de ordem já enviada podem ocultar o estado de execução.
- O gate padrão não roda os 15 testes PostgreSQL ignorados. `run-pg-integration-tests.sh:13-15` apenas avisa quando a URL não contém `trading_bot` e continua; a conexão Rust rejeita outros databases, mas o script deveria falhar antes para proteger o alvo.
- A documentação de contagem permanece inconsistente em trechos históricos: o script enumera 15 testes PG; conferir README e matrizes antes de atualizar números. Os gates atuais não executaram os 15 testes PG.
- As rotas `/api/v1/admin/provider-credentials*` estão registradas em router/OpenAPI, mas cada handler responde 501 após auth; a documentação operacional e SDD registram isso. `provider_credentials.secret` é armazenado como texto; criptografia em repouso segue como follow-up e exige proteção de acesso/backup.
- O seed SQL de exemplo executa upsert substituindo segredos existentes; as instruções avisam que é arquivo de exemplo e deve ser executado somente após substituir placeholders localmente.
- Inventário: **709 arquivos Git** sob `backend` (inclui 104 arquivos vendor e 309 itens Graphify). Código próprio e documentação foram analisados por categorias; vendor não foi revisado linha a linha. Graphify não processou SQL.
- Alterações locais já existentes foram preservadas; esta auditoria atualizou apenas o documento via OpenKnowledge e os artefatos Graphify gerados. Nenhuma correção de código foi aplicada.
