# Pesquisa de capacidades para o módulo `agents`

- **Status:** pesquisa inicial — revisão independente pendente
- **Data:** 2026-09-26
- **Escopo:** identificar capacidades de referência em Grok Bot, OpenBot, OpenClaw e Meta Muse; recomendar o que cabe na identidade persistente de agentes e separar fases futuras e itens excluídos.
- **Jev:** não aplicável. A comparação é uma síntese de fatos e critérios explícitos; uma classificação automatizada não acrescentaria evidência.

## Resumo executivo

A decisão principal é manter a primeira etapa como **registro de identidades `IdentityOnly`**. Dela fazem parte cadastro por agência, papel/nível tipado, supervisor tipado, associação explícita ao owner, estados de ciclo de vida, histórico durável de alterações e operações autenticadas de owner. Essas capacidades tornam os agentes entidades persistentes e administráveis; não pressupõem inferência, autonomia, canais ou ferramentas.

Os produtos comparados confirmam padrões úteis de identidade persistente, contexto durável, controle centralizado, isolamento, política de ação, auditoria e supervisão. Esses padrões devem orientar o desenho das fases correspondentes. Eles não justificam colocar cérebro, memória de conhecimento, ferramenta, computador persistente, automação agendada, mensagens entre agentes, interface multimodal ou execução contínua no primeiro módulo.

**Gate de implementação segue bloqueado:** a documentação já identificada exige resolver autenticação confiável do owner, bootstrap inicial, schema/permissões PostgreSQL e contrato de controle local; também falta aceitação explícita do contrato público. A mesma conta de sistema operacional não autentica exclusivamente o owner num socket Unix. Esta pesquisa não aprova Gate 1 nem substitui design review.

## Limites e método

Foram consultadas fontes oficiais de produto e documentação técnica de cada projeto. Recursos anunciados pelo fornecedor são descritos como alegações do fornecedor. Não foi feita instalação, teste prático nem auditoria dos produtos. A página `ai.meta.com/muse/` retornou somente o título no leitor; para especificações de desenvolvimento, a documentação oficial de Meta Model API forneceu conteúdo técnico sobre endpoints, modelos, estado, tool calling e Muse Code [9]. Nenhuma fonte consultada demonstra que a Meta disponibiliza ao nosso projeto um runtime de agentes integrável.

A página inicial do OpenBot declara que é um template alpha para customização própria e que o modo de clone começa com `OPENBOT_SINGLE_USER=true`, admitindo todas as requisições como administrador; assim, recursos descritos são padrões de referência e precisam de validação independente antes de adoção [2]. A documentação de OpenClaw delimita explicitamente um gateway como uma fronteira de confiança, não como isolamento multi-tenant hostil [5]. Essas restrições pesam contra copiar arquiteturas inteiras.

## Comparação de capacidades e decisão

| Capacidade observada | Referência | Decisão para o módulo `agents` |
|---|---|---|
| Agente nomeado com função e contexto persistentes | Grok Bot descreve Bots nomeados com papéis e contexto acumulado [1]; OpenBot define colegas com nome, título e descrição de papel [2] | **Agora:** identidade com nome, papel tipado e agência. Texto descritivo pode ser metadado inerte, se o contrato aprovado definir isso. Não representar “contexto” como memória ou prompt executável nesta fase. |
| Relação hierárquica | A proposta do produto bot define owner → CEO → Level B → Level A → especialistas/workers | **Agora:** supervisor tipado e validação das regras da hierarquia. CEO aponta para owner humano; subordinados apontam para identidades de agente. |
| Estado de identidade e ciclo de vida | Grok Bot permite bloquear acesso sem apagar computadores [3]; OpenBot administra pessoas e computadores separadamente [2][4] | **Agora:** criar, consultar, pausar, retomar e aposentar como transições explícitas e auditáveis. Aposentar preserva histórico; sem exclusão física no fluxo normal. |
| Registro durável e histórico | OpenBot armazena dados operacionais em PostgreSQL e fornece trilha de ações [2]; OpenClaw mantém estado e credenciais localmente [5] | **Agora:** persistir registro e eventos de mudança no PostgreSQL. Separar histórico de identidade de conversas, execução e memória semântica. Definir acesso/retensão no design de dados. |
| Autenticação e autoridade do owner | OpenBot descreve OAuth/SSO, papéis administrativos e recusa de inicialização sem configuração de identidade fora do modo single-user de desenvolvimento [2]; OpenClaw exige autenticação e pareamento para conexões não locais [6] | **Agora, bloqueador:** fonte confiável da identidade do owner e autorização no serviço para cada operação. Não herdar confiança da sessão do Grok Build. |
| Registro de alterações | OpenBot registra ações permitidas, negadas e falhas, inclusive alterações administrativas [2]; Grok Bot documenta registros de auditoria de criação de Bots e mudanças de acesso [3] | **Agora:** trilha durável para criação, pausa, retomada, aposentadoria, bootstrap e decisões de autorização, com ator, alvo, horário, resultado e origem minimizados. Formato e retenção aguardam design/revisão de dados. |
| Estado `IdentityOnly` que não executa | É limite específico da proposta do produto, não promessa dos produtos comparados | **Agora:** impor invariantes observáveis: sem chamada de modelo, tools, memória operacional, tarefas em background ou concessão de autoridade executável. Um registro existir não ativa worker. |
| Separação de controle e execução | OpenClaw centraliza sessões, ferramentas e eventos num Gateway tipado [6]; OpenBot aplica política e registra auditoria antes de agir [2] | **Futuro:** quando existir runtime, manter identidade/controle separado da execução e verificar cada chamada no lado servidor. Não construir gateway de ferramentas agora. |
| Isolamento de agentes e recursos | Grok Bot dá uma microVM por usuário e informa que Bots do mesmo usuário compartilham computador e logins [3]; OpenBot oferece computador/container por Bot [2] | **Futuro, condicionado a threat model:** definir fronteiras por agência, usuário, identidade e execução antes de conceder recursos. A primeira etapa precisa evitar exposição acidental de registros entre agências e limitar acesso ao owner autorizado; não precisa criar VMs/containers. |
| Permissões de ferramenta, política fail-closed e aprovações | OpenBot documenta gateway com CEL, negação por padrão e auditoria [2]; Grok Bot descreve aprovações e regras [3]; OpenClaw descreve perfis por agente e sandbox [5] | **Fases posteriores:** requisito para capacidade de agir, definido por ferramenta/ação e contexto. **Excluir agora:** não há ferramenta nem chamada de ação a governar em `IdentityOnly`. |
| Cérebro, modelos e delegação multiagente | Meta Model API documenta Muse Spark com chamadas paralelas a ferramentas, argumentos em streaming e janela de contexto de 1.048.576 tokens. Responses API suporta raciocínio que continua entre turnos e estado gerido pelo servidor [9]. Muse Code é um agente de programação que planeja, edita e executa comandos, com aprovações e sandbox ativos por padrão [9]. Meta também descreve orquestração multiagente em Muse Spark [8]. Grok Bot coordena Bots e delega [1]. | **Fase posterior do cérebro/runtime:** avaliar separadamente GPT-Sol/Terra/Luna via 9Router e JEV, como definido na proposta do produto. Meta API e Muse Code são referências/opções de fornecedor, não uma decisão de arquitetura. Não adotar chamadas a ferramentas, estado de raciocínio ou paralelismo no cadastro de identidade. |
| Memória entre sessões | Grok Bot declara memória de preferências e resumos separados por Bot [1]; OpenClaw documenta estado/memória persistentes [5] | **Fase posterior:** projeto separado para conhecimento, autoria, revisão, compartilhamento, retenção e aposentadoria; banco de grafo conforme proposta. Nesta etapa somente histórico administrativo de identidade. |
| Rotinas, agendamento, worker e recuperação | Grok Bot oferece rotinas agendadas e trabalho em computador de nuvem [1]; OpenBot limita frequência/quantidade e desliga rotina após falhas sucessivas [2]; OpenClaw usa Gateway daemon, health e supervisão [6] | **Fase de operação contínua:** worker, execução persistente, heartbeat, recuperação, limites e observabilidade exigem SDD próprio. **Excluir agora:** `IdentityOnly` não inicia processo nem rotina. |
| Canais, voz, aplicações e computador de usuário | Grok Bot tem clientes desktop/mobile/voz e navegador [1]; OpenClaw integra canais e apps [5][6]; Muse oferece modelos e ferramentas multimodais [8][9] | **Fora da etapa atual:** só avaliar quando houver interface de interação aprovada. Não confundir identidade no domínio com canal ou dispositivo. |
| Bots executores de tarefa/mercado | A proposta de domínio do bot separa agentes gerais de bots especializados | **Projeto posterior independente:** ciclo próprio de bots versionados e avaliados. Agente persistente não se converte automaticamente em executor financeiro. |
| Compras, pagamentos, envio/publicação e ações financeiras | Grok Bot e Meta descrevem aprovações para determinadas ações [3][8] | **Excluir completamente da primeira etapa.** Identidade não concede saldo privado, capital real, ordens, pagamentos nem ação de mercado. Qualquer fase futura exige autoridade, limites, aprovações e revisão de segurança próprios. |

## Capacidades recomendadas por fase

### Etapa 1 — identidade `IdentityOnly`

1. Identificador estável e atributos mínimos aprovados: agência, nome, nível/papel, supervisor tipado, estado e timestamps necessários.
2. Regras de integridade da hierarquia: tipos válidos, supervisor existente, ausência de ciclos, CEO ligado ao owner e subordinados a agente. Regras detalhadas devem ser confirmadas no contrato público.
3. Ciclo de vida com transições válidas e idempotentes, operações de consulta e trilha de eventos durável.
4. Autenticação verificável do owner, autorização por agência e checagem no servidor. Bootstrap inicial único, seguro, recuperável e auditado.
5. PostgreSQL com schema/migrações, privilégios mínimos, constraints e política de backup/retensão aprovados no Gate 1.
6. Limite executável `IdentityOnly`: nenhum subsistema de cérebro, memória de trabalho, ferramenta ou scheduler é inicializado pelo registro.

### Fases posteriores, com design e gates próprios

- **Cérebro:** provedores/modelos, roteamento via 9Router, avaliação e papel delimitado do JEV; definir custo, falhas, privacidade e fallback. A documentação da Meta fornece superfícies de API (Responses, Chat Completions e Messages) e recursos como tool calling, tool search, reasoning e structured output [9], úteis como material de comparação nessa fase.
- **Conhecimento e memória:** proveniência, revisão humana, política de compartilhamento e aposentadoria; banco de grafo conforme proposta do produto.
- **Metas, regras, permissões e ferramentas:** identidade autenticada por chamada, menor privilégio, política fail-closed, logs e aprovações conforme risco.
- **Operação contínua:** worker, fila/outbox se apropriado, execução durável, heartbeat, lease/locks, retry limitado, idempotência, recuperação e alertas com runbook.
- **Bots especializados:** artefatos versionados, limites de autoridade, avaliação e ciclo de promoção separados.

## Recursos filtrados para a primeira etapa

Ficam fora do módulo inicial: chamadas LLM; GPT-Sol/Terra/Luna e JEV; Muse Spark como cérebro; memória conversacional, memória de longo prazo ou grafo de conhecimento; ferramentas/MCP, navegador, shell, computador/VM/container; rotinas e agendamento; worker, heartbeat e execução 24/7; mensagens diretas/grupos e delegação; interface web/mobile/voz; perfis de personalidade/prompt executável; grants de credenciais; análise de desempenho/rentabilidade; bots especializados; saldos, capital, ordens e pagamentos.

Esses recursos podem ser necessários para a visão completa de agentes “vivos”, mas sua inclusão prematura misturaria identidade com capacidade de agir, ampliaria a superfície de ataque e tornaria `IdentityOnly` enganoso. Para a primeira entrega, “vivo dentro do sistema” significa **identidade consultável, estado administrável, supervisor resolvível e histórico preservado**, sem atividade cognitiva ou operacional.

## Bloqueios e decisões pendentes

- **Gate 1 bloqueado:** fonte confiável da identidade do owner e autenticação das chamadas locais.
- **Bootstrap bloqueado:** definir como o primeiro owner é estabelecido e como evitar bootstrap repetido ou takeover.
- **Dados bloqueados:** schema PostgreSQL, permissões, constraints, índices e política de retenção precisam de revisão independente.
- **Contrato local bloqueado:** socket Unix sozinho não prova que o processo pertence ao owner humano quando outros processos compartilham a conta do sistema operacional.
- **Contrato público pendente:** operações, transições, estados terminais, erros, concorrência e semântica de aposentadoria precisam de aceitação explícita antes dos testes, segundo as regras do projeto.
- **Crítico independente pendente:** as ferramentas disponíveis não confirmaram instância independente para revisar o artefato. O relatório permanece preliminar, sem aprovação de Gate 1.

## Fontes primárias

1. xAI/Cursor, [Grok Bot overview](https://docs.x.ai/grok-bot/overview) — Bots nomeados, computador persistente, colaboração, rotinas e contexto.
2. CopilotKit, [OpenBot README](https://github.com/CopilotKit/OpenBot) — arquitetura, autenticação, política de ações, auditoria, PostgreSQL e status alpha/template.
3. OpenClaw, [README](https://github.com/openclaw/openclaw) — Gateway, estado local, canais, segurança e plugins.
4. Cursor, [Grok Bot for teams and enterprises](https://docs.x.ai/grok-bot/teams-and-enterprises) — controles de acesso, isolamento, provisionamento, regras, aprovação e auditoria.
5. OpenClaw, [Security](https://docs.openclaw.ai/gateway/security) — fronteira de confiança e controles de endurecimento.
6. OpenClaw, [Gateway architecture](https://docs.openclaw.ai/concepts/architecture) — API tipada, autenticação, pareamento e operação do Gateway.
7. Meta AI, [Muse product page](https://ai.meta.com/muse/) — consultada; o leitor textual retornou apenas o título. Recursos operacionais detalhados não foram usados como fatos neste relatório.
8. Meta AI, [Introducing Muse Spark](https://ai.meta.com/blog/introducing-muse-spark-msl/) — modelo multimodal, tool use, orquestração paralela e lacunas reconhecidas em sistemas agentivos de horizonte longo.
9. Meta, [Meta Model API overview](https://dev.meta.ai/docs/overview) — modelos/endpoints Muse, chamadas paralelas a ferramentas, argumentos em streaming, contexto, APIs de conversação, Muse Code e sandbox/aprovações.
