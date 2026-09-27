---
title: Análise de módulos previstos ainda não desenvolvidos
description: Inventário de capacidades planejadas sem implementação completa, com evidências, dependências e próximos gates
tags:
  - planning
  - backend
  - roadmap
  - gaps
  - modules
---

# Análise de módulos previstos ainda não desenvolvidos

> Revisão: 2026-09-27. Esta análise cruza os SDDs, o roadmap, o catálogo de módulos e o código atual em `backend/src`. “Não desenvolvido” significa que não existe módulo/caminho executável correspondente ou que o design ainda não chegou ao comportamento completo descrito.

## Resumo

O backend atual implementa monitor de mercado, backtest, estratégia SMA, risco, TUI, integrações públicas Binance, persistência básica opcional, logging e Jev consultivo. Os módulos abaixo ainda não existem como capacidade completa:

1. Identidade persistente e administração de agentes.
2. Autenticação do owner, autorização por agência e bootstrap seguro.
3. Runtime de execução de agentes, worker, scheduler e recuperação.
4. Gateway de ferramentas, permissões, aprovações e sandbox.
5. Memória de conhecimento, memória entre sessões e grafo.
6. Canais de conversa, voz, aplicações e interface externa.
7. Execução financeira, saldos privados, ordens e ambiente de produção.
8. Observabilidade operacional completa.
9. Estado de persistência e recuperação do monitor conforme C17.
10. Round-trip PostgreSQL operacional conforme V18.

Essas capacidades não devem ser tratadas como módulos parcialmente prontos só porque existem tipos auxiliares, flags de configuração ou documentação de design.

## Classificação

| Capacidade prevista | Situação no código | Evidência | Próximo gate |
|---|---|---|---|
| Identidade de agentes `IdentityOnly` | Não existe módulo de identidade, schema ou API pública. | A pesquisa de agentes mantém Gate 1 bloqueado. Não há módulo correspondente em `src/`. | SDD de domínio, schema PostgreSQL, contrato público e autenticação. |
| Owner, agência e hierarquia | Não existe autenticação confiável nem autorização por agência. | A pesquisa registra que socket Unix e conta do SO não provam a identidade do owner. | Threat model, bootstrap único, autenticação verificável e revisão de segurança. |
| Runtime de agentes | Não existe cérebro, modelo, delegação ou execução de agente. | A pesquisa exclui chamadas LLM, delegação e runtime da etapa `IdentityOnly`. | SDD próprio de runtime e limites de autoridade. |
| Worker e scheduler | Não existe worker durável, agenda, heartbeat, lease ou retry de execução. | A pesquisa classifica rotinas e operação contínua como fase posterior. | SDD de execução durável, fila/outbox, recuperação e SLO. |
| Gateway de ferramentas | Não existe MCP/tool gateway, política por ação ou aprovação. | Pesquisa: ferramentas, sandbox e aprovações estão excluídos da primeira etapa. | Modelo de permissões, política fail-closed e auditoria. |
| Memória e conhecimento | Não existe memória conversacional, memória semântica ou grafo de conhecimento. | Pesquisa separa histórico administrativo de memória de agente e adia essa fase. | Proveniência, revisão, compartilhamento, retenção e aposentadoria. |
| Canais externos | Não existem canais de chat, voz, mobile, navegador ou computador persistente. | Pesquisa marca canais e dispositivos fora da etapa atual. | Contrato de interação, identidade por canal e controles de privacidade. |
| Execução financeira | Não existe ordem, saldo privado, produção ou execução real. | `exchanges/rest` autoriza somente `PublicSpotBackfill` em `dev`; `authorize_rest_use` falha para usos privados. | Projeto independente de execução, risco, custódia, aprovação e segurança. |
| Observabilidade | Há logging estruturado, mas não há catálogo completo de métricas, SLI/SLO, alertas ou runbook de incidentes. | Roadmap lista WS/REST, persistência, idade de candle, Jev e credenciais como pendências. | Definir métricas, cardinalidade, alertas, dashboards e runbooks. |
| Persistência de runtime | A camada `persistence` grava datasets, mas o estado de recuperação do monitor ainda não está completo. | SDD T-15 marca C17/G4 pendentes: `DEGRADED`, `HEALTHY`, `GAP`, suspeita de commit e recuperação. | Implementar C17 após C14/C15/C16 e revisar G4. |
| Integração PostgreSQL | Existe conexão, migração e persistência básica; a integração operacional completa não foi executada. | V18 está bloqueada por banco descartável; o teste PostgreSQL é ignorado por padrão. | Executar migração, commit, rollback, idempotência e limpeza em `trading_bot` isolado. |

## 1. Identidade persistente de agentes

### O que está previsto

A primeira etapa descrita na pesquisa é `IdentityOnly`, com:

- identificador estável;
- agência;
- nome e papel/nível;
- supervisor tipado;
- associação ao owner;
- estados de ciclo de vida;
- histórico durável de alterações;
- operações autenticadas de owner.

### O que existe

Não há módulo `agents`, entidades de identidade, migração própria, repositório, serviço ou API. Os tipos de `domain.rs` são voltados a bot, estratégia, métricas e sinais do backend de trading; não implementam identidade administrativa de agentes.

### Bloqueios

- fonte confiável da identidade do owner;
- bootstrap inicial único e recuperável;
- constraints de hierarquia e ausência de ciclos;
- permissões PostgreSQL;
- contrato público das transições;
- revisão independente de segurança.

## 2. Controle, autenticação e autorização

### O que está previsto

Toda operação de agente deve validar owner, agência, alvo, ação e estado no lado servidor. A hierarquia owner → CEO → Level B → Level A → especialistas/workers precisa ser uma regra de domínio auditável.

### O que existe

O backend possui gates de configuração e de uso REST da exchange, mas não possui autenticação de usuário, sessões, tokens, autorização por agência ou auditoria de operações administrativas.

### Decisão

Não reutilizar `Credentials` da exchange para autenticar o owner. São domínios diferentes: credenciais de exchange autorizam acesso técnico à Binance, enquanto owner/agência exigem identidade do produto.

## 3. Runtime, worker e scheduler

### O que está previsto

Uma fase posterior precisa suportar execução durável, scheduler, heartbeat, leases, retries limitados, idempotência, recuperação e observabilidade.

### O que existe

`app` possui tarefas do monitor e cancelamento por geração, mas isso não é um runtime geral de agentes. Não há fila de tarefas de agente, scheduler, worker persistente, lease ou recuperação de jobs.

### Risco

Confundir as tasks do monitor com um worker de agentes criaria autoridade implícita e dificultaria separar sinais de mercado de execução autônoma.

## 4. Ferramentas, aprovações e sandbox

### O que está previsto

A pesquisa prevê política por ferramenta/ação, negação por padrão, auditoria, aprovação humana e isolamento de recursos.

### O que existe

Jev recebe um snapshot consultivo e retorna observações textuais. Ele não chama ferramentas, não escolhe conta, não acessa exchange e não autoriza execução. Também não há gateway MCP, catálogo de ferramentas, policy engine ou sandbox.

### Dependências

Esse módulo só pode ser desenhado depois da identidade e autorização. A política precisa definir ator, alvo, ferramenta, argumentos, risco, aprovação, expiração e resultado auditado.

## 5. Memória, conhecimento e canais

Não existem módulos para:

- memória conversacional;
- memória semântica;
- grafo de conhecimento;
- proveniência e revisão de conhecimento;
- canais de chat, voz ou mobile;
- navegador ou computador persistente;
- mensagens entre agentes;
- delegação multiagente.

A pesquisa classifica todos esses itens como fases posteriores ou fora do escopo de `IdentityOnly`. Cada capacidade exige SDD próprio; não deve ser adicionada ao cadastro de identidade como campo executável ou prompt implícito.

## 6. Execução financeira e produção

O caminho de execução está deliberadamente ausente:

- sem criação de ordem;
- sem consulta ou movimentação de saldo privado;
- sem assinatura de ordens;
- sem account trading;
- sem ambiente de produção;
- sem estratégia que possa conceder autoridade financeira.

O módulo `exchanges/rest` autoriza somente backfill público Spot em `dev`. A existência de `RiskLimits`, `PortfolioSnapshot` ou sinais não representa execução financeira. Qualquer implementação futura precisa de autorização explícita, threat model, limites, aprovação humana, idempotência, reconciliação e rollback operacional.

## 7. Persistência e integração ainda incompletas

### C17 — estado de runtime e recuperação

O SDD T-15 ainda prevê:

- estado inicial `DEGRADED`;
- distinção `HEALTHY`, `DEGRADED` e `GAP`;
- suspeita de perda por overflow, pausa, desconexão ou commit incerto;
- recuperação somente após janela REST contígua e commit confirmado;
- descarte de resultados de gerações antigas;
- comportamento de timeout e retry.

C17 depende de C14, C15 e C16 e permanece pendente.

### V18 — PostgreSQL

A persistência base existe, mas a prova operacional completa não foi executada. V18 requer um banco `trading_bot` descartável e isolado para observar migração, commit, rollback após erro e idempotência. O teste ignorado não deve ser promovido a aprovação.

## 8. Observabilidade operacional

O logging atual não fecha a observabilidade necessária. Ainda precisam ser definidos:

- contadores de mensagens WS aceitas, rejeitadas e descartadas;
- latência e erro de REST;
- idade do último candle;
- estado e idade da persistência;
- tamanho e saturação da fila;
- falhas, timeout e latência de Jev;
- métricas de pausa, retomada, gaps e reconciliação;
- SLI/SLO;
- alertas;
- dashboards;
- runbooks de credenciais, banco e degradação.

## Ordem recomendada de desenvolvimento

1. Fechar C10 e o contrato/fallback do adapter REST.
2. Fechar C15 e confirmar o comportamento de backpressure do WS.
3. Fechar C16/C17 e executar V18 em banco isolado.
4. Definir observabilidade operacional e runbooks.
5. Criar SDD de identidade `IdentityOnly`.
6. Implementar autenticação, bootstrap e autorização do owner.
7. Só então criar runtime, ferramentas, memória, canais e execução financeira em projetos separados.

## Critérios para considerar um módulo desenvolvido

Um módulo previsto só deve sair desta lista quando houver:

- código presente e integrado ao fluxo correto;
- contrato público documentado;
- teste comportamental determinístico;
- tratamento de erro e falha;
- revisão independente registrada;
- evidência de integração quando houver dependência externa;
- atualização do catálogo, roadmap e matriz de testes.

## Referências

- [Catálogo completo de módulos](../architecture/module-catalog.md)
- [Matriz de testes](../reference/test-matrix.md)
- [Estado atual e planejamento](./current-state-and-roadmap.md)
- [Plano de execução](./backend-work-plan.md)
- [Pesquisa de capacidades de agentes](../research/agents-capability-research.md)
- [SDD T-10 — pausa e retomada](../sdd/monitor-pause-resume-sdd.md)
- [SDD T-15 — persistência opcional](../sdd/monitor-persistence-policy-sdd.md)
- [SDD T-05 — redirects REST](../sdd/rest-redirect-sdd.md)
