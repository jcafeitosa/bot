---
title: "Complemento de pesquisa: capacidades de agentes e fontes Meta"
description: Atualização da comparação de runtimes com fontes primárias preservadas no OpenKnowledge
tags:
  - research
  - agents
  - meta
  - openbot
  - grok-bot
  - openclaw
status: provisional
---
# Complemento da pesquisa de capacidades para o módulo `agents`

- **Status:** provisório; fontes preservadas localmente, revisão independente pendente.
- **Escopo:** complementar a pesquisa [original](./agents-capability-research.md) com a documentação de desenvolvimento Meta indicada e referências primárias atualizadas de Grok Bot, OpenBot e OpenClaw.
- **Jev:** não aplicável. É comparação documental com critérios explícitos; classificação automatizada não acrescentaria evidência.

## Conclusão executiva

A recomendação permanece: a primeira entrega deve ser um registro de identidade `IdentityOnly`. Para que agentes estejam “vivos” no sistema nessa etapa, significa existirem como identidades consultáveis, vinculadas a uma agência e a um supervisor tipado, com ciclo de vida e histórico persistentes. A identidade não executa inferência, não inicia tarefas, não recebe ferramentas nem ganha autoridade operacional.

As fontes externas mostram capacidades para fases completas de runtime — modelos, contexto, ferramentas, computadores isolados, memória, rotinas, mensagens, controle e aprovações. Esses recursos não devem ser absorvidos pelo registro inicial. A proposta existente de produto já os separa em projetos posteriores, incluindo cérebro, conhecimento/memória, metas/permissões/ferramentas, operação contínua e bots especializados.

## Matriz de capacidades e decisão

| Capacidade | Evidência em fontes primárias | Decisão para `agents` |
|---|---|---|
| Identidade persistente com nome e função | Grok Bot descreve Bots nomeados com trabalho e contexto persistente [Grok Bot overview](../external-sources/grok-bot-overview.md). OpenBot configura colegas por nome, título e papel [OpenBot README](../external-sources/openbot-readme.md). | **Etapa 1:** identidade estável, agência, papel/nível e supervisor tipados. Texto de função fica inerte, sem prompt executável. |
| Supervisor e hierarquia de autoridade | A hierarquia é requisito do domínio do produto bot: owner → CEO → Level B → Level A → especialistas/workers. Nenhum dos quatro sistemas avalia essa autoridade empresarial específica. | **Etapa 1:** aplicar a hierarquia do domínio como regra interna. CEO vinculado ao owner humano; subordinados vinculados a agentes. Verificar ciclos e referências inexistentes no design aprovado. |
| Ciclo de vida administrativo | Grok documenta controles administrativos de acesso que podem bloquear usuários sem apagar computadores [Grok Bot teams](../external-sources/grok-bot-teams-and-enterprises.md). OpenBot oferece administração de colegas/pessoas e eventos de auditoria [OpenBot README](../external-sources/openbot-readme.md). | **Etapa 1:** criar, consultar, pausar, retomar e aposentar identidades; persistir cada transição e preservar histórico. A semântica exata aguarda contrato público aprovado. |
| Persistência de identidade e histórico | OpenBot usa PostgreSQL para dados de produto, políticas e auditoria [OpenBot README](../external-sources/openbot-readme.md). OpenClaw declara que estado e credenciais vivem na instalação do operador [OpenClaw Security](../external-sources/openclaw-security.md). | **Etapa 1:** PostgreSQL é a persistência proposta para identidade e eventos administrativos. Não misturar com conversas, estado de execução ou memória semântica. Schema, permissões, retenção e recuperação aguardam Gate 1 e revisão de dados. |
| Identidade autenticada do owner | OpenBot documenta OAuth/SSO e alerta que o modo inicial `OPENBOT_SINGLE_USER=true` trata todas as requisições como um administrador [OpenBot README](../external-sources/openbot-readme.md). OpenClaw requer pareamento/autenticação segundo contexto de conexão e delimita uma fronteira de confiança por gateway [OpenClaw Security](../external-sources/openclaw-security.md). | **Bloqueador da etapa 1:** estabelecer autenticação confiável e autorização por agência/ação. Um socket Unix não diferencia o owner de processos sob a mesma conta de sistema. Bootstrap inicial precisa impedir takeover/reexecução indevida. |
| Registro de alterações | OpenBot afirma que as ações passam pela avaliação de política e ficam auditadas; Grok separa action recording e audit logs administrativos [OpenBot README](../external-sources/openbot-readme.md), [Grok Bot teams](../external-sources/grok-bot-teams-and-enterprises.md). | **Etapa 1:** eventos de criação, pausa, retomada, aposentadoria, bootstrap e negação/autorização com ator, alvo, instante e resultado minimizados. Definição append-only/retensão é decisão de dados pendente. |
| `IdentityOnly` sem execução | Trata-se de uma restrição específica do produto bot, não uma garantia dos sistemas de referência. OpenBot, Grok e OpenClaw descrevem runtimes capazes de executar ações. | **Etapa 1:** provar por teste/comportamento que cadastro não chama modelo, não abre ferramenta/computador, não agenda tarefa e não concede autoridade executável. |
| Isolamento de tenant e execução | Grok diz que há uma microVM por usuário, enquanto Bots da mesma conta compartilham computador e logins [Grok Bot teams](../external-sources/grok-bot-teams-and-enterprises.md). OpenBot descreve computador por Bot, mas é um template alpha [OpenBot README](../external-sources/openbot-readme.md). OpenClaw adverte que um Gateway não é uma fronteira multi-tenant hostil [OpenClaw Security](../external-sources/openclaw-security.md). | **Futuro, antes de ferramentas:** definir isolamento por agência, identidade, usuário, credencial e processo. Não construir VMs/containers para o registro inicial. Não assumir que separação de persona equivale a isolamento de dados. |
| Política de ferramenta e aprovações | OpenBot descreve gateway fail-closed, política CEL e auditoria anterior à ação [OpenBot README](../external-sources/openbot-readme.md). Grok descreve concessão de conectores, Auto Review, regras e aprovação de ações sensíveis [Grok Bot teams](../external-sources/grok-bot-teams-and-enterprises.md). OpenClaw descreve perfis por agente, pairing e sandbox [OpenClaw Security](../external-sources/openclaw-security.md). | **Fases posteriores de permissões/ferramentas:** necessário antes de qualquer capacidade de agir; políticas devem negar por padrão e considerar ator, agência, ação e contexto. **Excluir agora:** não existe ação executável no estado IdentityOnly. |
| Cérebro, APIs de modelo e chamadas de ferramentas | Meta Model API documenta Muse Spark, chamadas paralelas de ferramenta, argumentos em streaming, contexto de 1.048.576 tokens e Responses API com estado gerido pelo servidor [Meta Model API overview](../external-sources/meta-model-api-overview.md). Também apresenta Muse Code como agente de programação com planejamento, edição, execução de comandos, aprovações e sandbox. A publicação de Muse Spark documenta raciocínio multimodal e orquestração paralela a publicação Meta de Muse Spark — captura local pendente. | **Fase posterior de cérebro/runtime:** avaliar GPT-Sol/Terra/Luna via 9Router e JEV conforme a proposta de produto. Meta é opção/referência técnica, sem decisão de fornecedor. API de modelo não é, por si, o runtime da empresa. Não usar em cadastro de identidade. |
| Memória entre sessões e conhecimento | Grok Bot documenta preferências, resumos, arquivos e sessões persistentes [Grok Bot overview](../external-sources/grok-bot-overview.md). OpenClaw descreve estado local da instalação [OpenClaw README source](../external-sources/openclaw-readme.md) — captura pendente. | **Fase separada:** conhecimento/memória com autoria, revisão, compartilhamento e aposentadoria; o grafo proposto não faz parte do histórico administrativo da identidade. |
| Trabalho agendado e 24/7 | Grok descreve rotinas e trabalho em segundo plano [Grok Bot overview](../external-sources/grok-bot-overview.md). OpenBot descreve rotinas com limite e desligamento após falhas repetidas [OpenBot README](../external-sources/openbot-readme.md). OpenClaw descreve Gateway de longa duração e health/heartbeat na página de arquitetura [OpenClaw architecture source](../external-sources/openclaw-architecture.md) — captura pendente. | **Fase de operação contínua:** worker, execução persistente, heartbeat, recuperação, retries/idempotência, limites e observabilidade precisam de SDD próprio. Excluir da primeira etapa. |
| Comunicação entre agentes, canais e UI | Grok descreve chat em grupo, handoff e paralelismo [Grok Bot overview](../external-sources/grok-bot-overview.md); OpenClaw conecta canais de mensagem ao Gateway [OpenClaw README source](../external-sources/openclaw-readme.md) — captura pendente. | **Futuro ou não necessário conforme caso:** comunicação e interface precisam de contratos de identidade/visibilidade e não fazem parte do cadastro. |
| Bots especializados e ações financeiras | A proposta do produto separa bots executores versionados/avaliados dos agentes gerais. Aprovações de produtos de referência não demonstram rentabilidade nem autorização financeira. | **Projeto separado:** bots especializados. Capital real, saldo privado, ordem e pagamento permanecem explicitamente fora da identidade e exigem autorização, limites e revisão próprios. |

## Escopo recomendado por fase

### Etapa 1: identidade persistente

- ID imutável, agência, nome, nível/papel, supervisor tipado, estado e timestamps mínimos acordados.
- Regras de integridade da hierarquia e isolamento de consultas por agência/owner.
- Estados e transições de ciclo de vida explícitos, idempotentes e auditáveis.
- Autenticação real do owner e autorização no serviço; bootstrap único com recuperação controlada.
- PostgreSQL com constraints, privilégios mínimos e retenção aprovados no Gate 1.
- Invariante testável: estado `IdentityOnly` não carrega modelo, ferramentas, memória operacional, worker ou scheduler.

### Fases posteriores

- **Cérebro:** modelos, roteamento 9Router, avaliações, uso delimitado de JEV e gestão de custo/falha.
- **Conhecimento/memória:** proveniência, revisão, compartilhamento e aposentadoria.
- **Metas/permissões/ferramentas:** política fail-closed, grants mínimos, aprovação e auditoria.
- **Operação contínua:** filas, execução durável, heartbeat, recuperação e runbooks.
- **Bots especializados:** executores separados, versionados e avaliados.

## Recursos excluídos de `IdentityOnly`

Inferência e prompts executáveis; Muse Spark ou outros provedores de cérebro; tool calling e MCP; navegador, shell, computador ou VM; memória conversacional e grafo de conhecimento; metas e delegação; chat, voz, apps e canais; rotinas, scheduler, worker, heartbeat e operação 24/7; grants de credenciais; bots de mercado; análise de rentabilidade; saldo, capital, ordens e pagamentos.

## Riscos, pendências e gates

- **Gate 1 continua bloqueado:** autenticação do owner, bootstrap inicial, schema/permissões PostgreSQL e contrato de controle local precisam de definição e revisão.
- **Contrato público:** obter aceitação explícita dos seams, estados, erros, concorrência e semântica de aposentadoria antes de testes, conforme as regras do projeto.
- **Fontes pendentes de captura:** publicação Muse Spark, README/arquitetura OpenClaw e outras páginas Grok/OpenBot citadas em detalhes. O relatório não as trata como referências localmente preservadas.
- **Crítico independente:** nenhuma instância independente confirmou revisão deste documento. Pesquisa e Gate 1 permanecem sem aprovação.

## Fontes preservadas

- [Grok Bot overview](../external-sources/grok-bot-overview.md)
- [Grok Bot teams and enterprises](../external-sources/grok-bot-teams-and-enterprises.md)
- [OpenBot README](../external-sources/openbot-readme.md)
- [OpenClaw Security](../external-sources/openclaw-security.md)
- [Meta Model API overview](../external-sources/meta-model-api-overview.md)

## Referências externas ainda não capturadas

- Muse Spark: https://ai.meta.com/blog/introducing-muse-spark-msl/
- OpenClaw README: https://github.com/openclaw/openclaw
- OpenClaw Gateway architecture: https://docs.openclaw.ai/concepts/architecture
