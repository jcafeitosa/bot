---
type: proposal
description: Decidir separar capacidades compartilhadas em core e módulos de domínio em src/modules, usando MVC quando aplicável
status: draft
authors:
  - user
  - assistant
created: 2026-09-27
tags:
  - proposal
  - architecture
  - backend
  - rust
---
# Proposta 0001: separar `core` e módulos de domínio no backend Rust

## Motivation

**Beneficiário:** quem desenvolve e mantém o backend Rust.

**Mudança observável:** o código de domínio fica sob `src/modules/`; capacidades realmente compartilhadas ficam sob `src/core/`; os módulos que têm fluxo de entrada/apresentação expõem uma organização MVC compreensível, sem forçar `Controller` ou `View` a módulos de cálculo puro e adapters.

**Decisão forçada:** adotar ou não essa separação arquitetural, incluindo a regra de dependência de que o `core` não dependa de módulos de domínio e serviços compartilhados não sejam duplicados por módulo.

O backend atual é um binário Rust cujo `main.rs` declara módulos de domínio e infraestrutura no mesmo nível. O catálogo documenta `app` como orquestrador do monitor, `config`, `error`, `logging`, `persistence`, `ui`, `exchanges`, `market`, `strategy`, `risk`, `portfolio`, `backtest` e `jev`. A inspeção dos imports também encontrou dependências transversais que atravessam domínios: configuração importa períodos da estratégia; tipos de domínio importam configuração e estratégia; logging consome configuração; e o monitor (`app`) coordena UI, exchanges, market, strategy, risk, Jev e persistência. Isso torna pouco evidente o que é núcleo compartilhado, domínio e composição da aplicação.

A pessoa que mantém o backend deve conseguir localizar uma capacidade pelo domínio ou pelo papel transversal, e as dependências comuns devem possuir um único dono. Uma reorganização sem alteração funcional pode tornar a localização e a direção das dependências verificáveis na árvore e nos imports; não se presume melhoria de desempenho nem mudança da autoridade de execução.

### Não objetivos

- Alterar regras de trading, risco, estratégias, integração com exchanges, persistência ou comportamento da TUI.
- Habilitar ordens, credenciais privadas ou operação em produção.
- Dividir o crate em vários crates Rust nesta proposta.
- Reescrever o monitor para um framework web ou impor MVC clássico a lógica sem camada de apresentação.
- Fazer migração de banco ou alterar schemas SQL.
- Implementar novos recursos ou reorganizar módulos de `agents`.

## Design

### Forma da árvore proposta

```text
backend/src/
├── main.rs                    # ponto de entrada e composição
├── core/                      # capacidades compartilhadas, sem regras de domínio
│   ├── mod.rs
│   ├── config/                # configuração de processo e validação transversal
│   ├── error.rs               # tipos de erro comuns apenas quando há uso transversal
│   ├── logging.rs             # inicialização/configuração de tracing
│   ├── persistence/           # conexão, migrações e transações comuns
│   └── ...                    # adicionar apenas serviço usado por mais de um módulo
├── modules/
│   ├── market/
│   │   ├── mod.rs
│   │   ├── models.rs
│   │   ├── services.rs
│   │   └── adapters/          # fontes REST/WS quando forem parte do domínio de mercado
│   ├── strategy/
│   │   ├── mod.rs
│   │   ├── models.rs
│   │   └── services.rs
│   ├── risk/
│   ├── portfolio/
│   ├── backtest/
│   ├── exchanges/
│   │   ├── mod.rs
│   │   ├── models.rs
│   │   ├── services.rs
│   │   └── adapters/          # Binance, REST, WS e registry conforme ownership final
│   ├── monitor/
│   │   ├── mod.rs
│   │   ├── controllers/       # coordenação de casos de uso do monitor
│   │   └── services.rs
│   └── jev/
├── presentation/
│   └── terminal/              # TUI: views e tradução de input em comandos
└── ...
```

Essa árvore é uma hipótese revisável, não um mapa final de cada arquivo. `main.rs` é o composition root: lê argumentos e configurações, constrói adapters e dependências, e inicia o módulo/caso de uso apropriado. Regras de domínio permanecem nos módulos que as possuem.

### Regra do `core`

Um serviço pertence a `core` somente quando mais de um módulo de domínio o usa com a mesma semântica e quando ele não contém política específica de um domínio. Exemplos candidatos: leitura de configuração transversal, inicialização de tracing, tipos de erro realmente comuns, conexão PostgreSQL e suporte transacional comum.

`core` não deve importar `modules::*`. Se uma abstração compartilhada precisar de um conceito de domínio, a interface comum deve ser independente desse conceito ou o serviço deve permanecer no módulo proprietário. Serviços específicos de domínio, como autorização de uso REST de exchanges ou gravação de candles, permanecem com o domínio/adaptador proprietário mesmo que usem o pool PostgreSQL do `core`.

Compartilhamento será decidido por uso real, não por antecipação: código usado por um único módulo continua nesse módulo. O `core` não se torna um depósito genérico de helpers.

### MVC aplicado conforme o fluxo

MVC é uma organização interna dos módulos com uma entrada e uma saída identificáveis:

- **Model:** tipos e regras de domínio do módulo, sem dependência da TUI/CLI.
- **Controller:** coordena um caso de uso, converte entrada em chamadas de domínio e encaminha o resultado; evita concentrar lógica de negócio.
- **View:** apresenta estado e coleta entrada. A TUI existente pode residir em `presentation/terminal` e chamar comandos/casos de uso expostos pelo módulo de monitor.

Módulos de cálculo como estratégia, risco e backtest podem conter `models` e `services` sem pastas vazias de controllers/views. Adapters de infraestrutura não são Views: exchanges e PostgreSQL são adapters nas interfaces apropriadas. A proposta usa MVC como convenção onde há interação e separa adapters onde há integração técnica.

### Direção de dependências

```text
main (composition)
  ├── core
  ├── presentation/terminal ──> interfaces/casos de uso do monitor
  └── modules/* ──> core (quando necessário)

modules/* não importam presentation/*
core não importa modules/*
```

Integrações específicas dependem do módulo dono do caso de uso. O módulo de monitor pode coordenar módulos de mercado, estratégia, risco, portfólio, Jev e persistência sem transferir a política desses domínios para `core`.

### Mapeamento inicial dos módulos atuais

| Código atual | Destino candidato | Observação |
|---|---|---|
| `logging` | `core/logging` | Compartilhado pela aplicação; deixar o tipo de configuração em core ou injetar configuração sem ciclo. |
| `config` | `core/config` somente para opções transversais; configurações de domínio podem ficar no módulo dono | Hoje `config` depende de `strategy::periods_for_mode`; essa dependência precisa ser removida/reformulada, não apenas movida de pasta. |
| `error` | `core/error` para erros transversais; erros especializados locais aos módulos | Evitar enum global que conhece todos os domínios e cresce a cada módulo. |
| `persistence` | `core/persistence` para pool/migrações/suporte transacional | Repositórios e consultas com semântica de domínio ficam nos respectivos módulos. |
| `exchanges` | `modules/exchanges` | Adapters e regras de capacidade REST/WS pertencem à integração de exchange; revisar tipos genéricos compartilhados. |
| `market`, `market_feed` | `modules/market` como hipótese | Decidir se o feed híbrido pertence ao domínio de mercado ou ao caso de uso monitor. |
| `strategy`, `risk`, `portfolio`, `backtest` | módulos homônimos | MVC não exige controller/view quando são serviços de domínio sem interação própria. |
| `app`, `monitor_startup`, `persistence_health` | `modules/monitor` ou composição de `main` | Separar controller/caso de uso de inicialização e política operacional. |
| `ui` | `presentation/terminal` | View e tradução de eventos/comandos; sem regra de domínio. |
| `jev` | `modules/jev` | Adapter/serviço consultivo; não passa a ser núcleo compartilhado por ser externo. |
| `domain.rs` | redistribuir pelos módulos proprietários ou criar um módulo de domínio explícito | Evitar `domain` universal que importa `config` e `strategy`. |

### Migração em alto nível

1. Fixar os seams de import e propriedade por tipo, aprovando a árvore e os nomes antes dos testes de migração.
2. Criar `core` e `modules` com interfaces de reexportação deliberadas; mover um pequeno módulo de baixo acoplamento por etapa.
3. Corrigir ciclos/relações impróprias antes de mover os módulos dependentes, especialmente `config → strategy` e tipos compartilhados de `domain`.
4. Migrar adapters/persistência e o monitor em fatias verticais pequenas, mantendo comportamento e testes existentes.
5. Migrar TUI para `presentation/terminal`, verificar fluxos de monitor e backtest, e remover caminhos antigos apenas após uso zero demonstrado.
6. Atualizar catálogo, README e guias depois de cada etapa aceita.

A proposta não especifica ainda APIs Rust públicas, nomes definitivos de todos os arquivos, nem a divisão exata de config/persistence; essas escolhas pertencem à revisão e à especificação derivada.

## Alternatives

### Alternative: manter a estrutura atual e documentar ownership

Manter arquivos e módulos como estão, atualizar o catálogo e estabelecer regras de import/ownership sem mover diretórios.

**Por que não escolher como destino:** tem o menor risco e custo imediato, mas não atende à mudança observável de localizar módulos de domínio em `src/modules/` nem torna visível a separação do núcleo. Continua sendo uma alternativa válida se o custo da migração superar o benefício.

### Alternative: somente mover módulos para `src/modules/`, sem `core`

Agrupar todo o código atual sob `modules/`, mantendo configuração, logging e banco dentro de módulos ou de uma pasta genérica de infraestrutura.

**Por que não escolher:** agrupa domínios, mas deixa serviços compartilhados sem proprietário explícito ou incentiva dependências duplicadas. Não satisfaz a regra confirmada pelo usuário para serviços usados por vários módulos.

### Alternative: MVC estrito em cada módulo

Criar `models/`, `controllers/` e `views/` para cada módulo, independentemente de haver interface/presentação.

**Por que não escolher:** módulos de cálculo e adapters ganhariam camadas vazias ou pass-through, aumentando navegação e manutenção sem comportamento correspondente. A proposta limita MVC aos módulos com entrada/apresentação relevante.

### Alternative: dividir em múltiplos crates

Criar crates Rust para core, módulos de domínio e aplicação, com dependências verificadas pelo Cargo.

**Por que não escolher nesta proposta:** oferece isolamento de compilação mais forte, mas aumenta configuração, fronteiras de tipos e custo de migração. Deve ser reavaliado se a equipe precisar de versionamento/reuso independente ou se as regras de dependência não puderem ser mantidas dentro de um crate.

### Alternative: não fazer a reorganização agora

Manter a árvore até uma necessidade funcional exigir mudança.

**Por que não escolher:** evita risco de movimentação no curto prazo. Mantém o problema de localização e ownership identificado e posterga a separação pedida; pode ser preferível se não houver disponibilidade para revisar e migrar incrementalmente.

## Drawbacks

- Quem mantém o backend pagará um custo temporário de navegação, revisão e resolução de imports durante a migração.
- Alguns nomes propostos — `market`, `monitor`, `presentation` — podem não corresponder aos limites reais; movimentos prematuros consolidariam ownership incorreto.
- Um `core` amplo pode virar dependência global e concentrar políticas distintas. A regra “compartilhado por uso real e sem política de domínio” requer revisão contínua.
- MVC parcial exige uma explicação clara para novos contribuidores; alguns módulos terão `controllers`/`views`, outros não.
- Mudanças de caminho podem quebrar testes de integração, comandos, documentação e tooling que referenciam arquivos diretamente.
- Reexports temporários reduzem o custo de migração, mas podem virar uma segunda API se não houver remoção e prazo verificáveis.
- A reorganização não resolve automaticamente a concentração de responsabilidades de `app`; a extração de casos de uso precisa ser desenhada sem alterar concorrência, cancelamento e semântica de pausa/retomada.

## Unresolved questions

- **`market_feed` pertence ao módulo `market` ou ao caso de uso `monitor`?** O que resolveria: mapear quem possui watermark, merge REST/WS e quais consumidores existem; decisão por arquitetura/backend.
- **Quais tipos de `config` são opções da aplicação e quais são regras de domínio?** O que resolveria: inventário campo-a-campo dos consumidores e política de validação; decisão por arquitetura e owners de módulos.
- **`core/persistence` deve expor somente pool/migrações ou também transações e traits de repositório?** O que resolveria: listar adapters e consumidores atuais/futuros e comparar acoplamento; decisão por banco de dados e arquitetura.
- **Como `BotError` será dividido entre erros de módulo e erros de processo?** O que resolveria: grafo de conversões/propagação e erros exibidos na CLI/TUI; decisão por backend/readability.
- **A UI terminal deve chamar um controller de `monitor` ou uma interface de aplicação separada?** O que resolveria: definir a direção de comandos, eventos e estado compartilhado com testes de comportamento; decisão por arquitetura e QA.
- **A migração precisa de reexports temporários para preservar imports?** O que resolveria: plano de fatias e resultado de `cargo test` por etapa; decisão por implementação após aceitação da proposta.
- **MVC deve ser nome literal das pastas ou uma convenção de responsabilidades com nomes idiomáticos Rust?** O que resolveria: revisão de legibilidade do protótipo estrutural e acordo com usuários do código; decisão por readability/arquitetura.

## Referências locais

- [Catálogo completo de módulos do backend](../architecture/module-catalog.md)
- [Referência de módulos do backend](../architecture/backend-module-reference.md)
- [Integrações do backend](../architecture/integrations.md)
- [Documentação do backend](../index.md)
