---
type: proposal
description: Decidir separar capacidades compartilhadas em core e módulos de domínio em src/modules, usando responsabilidades idiomáticas Rust
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

**Mudança observável:** o código de domínio fica sob `src/modules/`; capacidades técnicas realmente compartilhadas ficam sob `src/core/`; fluxos de entrada e apresentação têm responsabilidades explícitas, sem impor MVC clássico a cálculo puro ou adapters.

**Decisão forçada:** adotar ou não essa separação arquitetural, incluindo ownership dos seams, regra de dependências acíclicas e critério para admitir capacidades em `core`.

O backend atual é um binário Rust cujo `main.rs` declara módulos de domínio e infraestrutura no mesmo nível. O catálogo documenta `app` como orquestrador do monitor, `config`, `error`, `logging`, `persistence`, `ui`, `exchanges`, `market`, `strategy`, `risk`, `portfolio`, `backtest` e `jev`. Imports transversais incluem configuração que consulta períodos da estratégia, tipos comuns que dependem de configuração e estratégia, logging que consome configuração e monitor que coordena vários domínios. A proposta torna ownership e direção de dependências localizáveis e revisáveis; não presume ganhos de desempenho nem altera autoridade de execução.

### Não objetivos

- Alterar regras de trading, risco, estratégias, exchanges, persistência ou comportamento da TUI.
- Habilitar ordens, credenciais privadas ou operação em produção.
- Dividir o crate em vários crates Rust nesta proposta.
- Reescrever o monitor para framework web ou impor MVC clássico a lógica sem apresentação.
- Fazer migração de banco ou alterar schemas SQL.
- Implementar recursos ou reorganizar módulos de `agents`.

## Design

### Forma da árvore proposta

```text
backend/src/
├── main.rs                    # composition root: construir adapters e iniciar a aplicação
├── core/                      # somente capacidades técnicas transversais admitidas
│   ├── config/                # parsing e configuração operacional/processo
│   ├── logging.rs             # inicialização de tracing
│   └── ...                    # somente conforme regra de admissão
├── modules/
│   ├── market/                # domínio de mercado, casos de uso e ports/adapters próprios
│   ├── strategy/              # política e tipos de estratégia
│   ├── risk/
│   ├── portfolio/
│   ├── backtest/
│   ├── exchanges/             # integração/adapters de exchanges
│   ├── monitor/               # casos de uso de monitor e orquestração da aplicação
│   └── jev/
└── presentation/
    └── terminal/              # TUI: entrada, rendering e tradução para comandos
```

A árvore é uma hipótese, não um mapa arquivo-a-arquivo. Cada módulo de domínio pode conter `domain/`, `application/` (ou `use_cases/`), `ports/` e `adapters/` somente onde isso esclarece responsabilidades. `main.rs` é o composition root: constrói implementações concretas, injeta dependências e inicia o caso de uso apropriado. Não contém regras de negócio.

### Responsabilidades idiomáticas (em vez de MVC literal)

- **Domain (`modules/<domínio>/domain`)**: entidades, value objects, invariantes e políticas próprias; sem dependência de TUI, configuração operacional ou adapters concretos.
- **Application/use cases (`modules/<domínio>/application`)**: coordena operações orientadas a objetivos do usuário/processo, aplica política de fluxo e depende de ports, não de detalhes concretos. No módulo `monitor`, fica a coordenação que hoje cabe a `app`.
- **Ports/adapters**: ports são traits/types definidos pelo consumidor que expressam o contrato necessário; adapters implementam esses contratos para PostgreSQL, exchanges ou feeds. Adapters não são Views e não transferem ownership da política de domínio.
- **Presentation**: `presentation/terminal` é a interface terminal/TUI. Coleta teclas/comandos, converte-os em chamadas ao caso de uso e renderiza estado/resultados; não implementa regra de trading nem acessa diretamente persistência. Seu fluxo é presentation → application interface/command → domain/ports, com resultados/eventos retornando para renderização.

Essas funções correspondem a uma separação de Model/Controller/View quando há interação, mas os nomes Rust refletem domínio, casos de uso, ports/adapters e presentation, não pastas MVC obrigatórias. Serviços de cálculo podem expor domínio e casos de uso sem controller ou view artificiais.

### Ownership dos seams e dependências acíclicas

Ownership concreto para os acoplamentos conhecidos:

1. **Períodos por modo**: `strategy` é dono da tabela/política que mapeia modo de estratégia a períodos. `strategy::domain` publica um tipo/valor estável `StrategyMode` e uma função pura `periods_for_mode(StrategyMode) -> &'static [Period]` (ou coleção equivalente definida na especificação). Configuração não importa `modules::strategy`; parsing de configuração somente valida/decodifica o modo e entrega `StrategyMode` ao composition root/caso de uso. O wiring escolhe/invoca a política da estratégia. Não copiar a tabela para `core/config`.
2. **`OperationMode`**: o conceito que seleciona comportamento operacional de estratégia pertence a `modules::strategy::domain` como `StrategyMode` (o nome final pode ser `OperationMode` se inventário comprovar que o conceito é mais amplo que estratégia, mas nunca em `domain` universal dependente de config). Config parser pode produzir um tipo bruto/config DTO; o composition root converte-o para o tipo de domínio. Domain não importa `core::config`.
3. **`Signal`**: pertence a `modules::strategy::domain`, pois é produzido/consumato pela política de estratégia. Domínios consumidores, como monitor/risk, importam o tipo público do módulo strategy ou um contrato público deliberado; strategy não depende de monitor, risk ou presentation. Se futura evidência mostrar semântica compartilhada independente, uma interface mínima poderá ser movida por decisão explícita, nunca para encerrar ciclo por conveniência.
4. **`domain.rs` atual**: não permanece como módulo universal que importe config e strategy. Tipos são redistribuídos ao domínio proprietário; contratos entre domínios são expostos pelo proprietário produtor/consumidor com dependência unilateral. `core` não hospeda tipos de domínio.

Grafo permitido de alto nível: `presentation -> application/use cases -> domain + ports`; adapters implementam ports; composition root liga interfaces; domínios podem depender de tipos públicos de outro domínio somente em direção unilateral declarada. `core` pode ser usado por aplicação/adapters/domínios quando tecnicamente neutro, mas nunca importa `modules/*` ou `presentation`. Nenhum par de módulos de domínio pode formar ciclo. Tipos de configuração operacional não são dependência de domain.

Para o futuro `Period`/`StrategyMode` compartilhado, a estratégia define a interface; config limita-se a parsing/validação de sintaxe e o composition root faz o mapeamento. A direção de dependência segue o tipo proprietário, não o caminho atual do import.

### Regra verificável de admissão em `core`

Mover capacidade para `core` exige evidência cumulativa em revisão:

1. Pelo menos dois módulos consumidores identificados por código/testes ou chamada de aplicação real (não consumidores hipotéticos).
2. Mesma semântica e invariantes para todos os consumidores; não decide política, vocabulário, persistência ou erro específico de um domínio.
3. Interface pública independente de tipos de domínio e de adapters concretos; grafo de imports permanece sem caminho `core -> modules/*`.
4. Um dono explícito e teste que demonstre uso/contrato compartilhado.

Falhando qualquer condição, manter a capacidade no módulo consumidor/proprietário e injetar a interface necessária. `core` contém somente mecanismo técnico compartilhado. Configuração de domínio, repositórios/queries com semântica de domínio, transações que codificam política de domínio e erros de domínio pertencem ao módulo dono. Se aplicável, `core/persistence` limita-se a mecanismo de conexão/pool e plumbing genérico de transação/migração; adapters/repositórios e política de persistência ficam no domínio. Erros transversais de processo podem ser comuns, mas enums globais não devem conhecer erros de todos os domínios. `logging` pode inicializar infraestrutura, recebendo parâmetros resolvidos no composition root sem fazer `core` depender de configuração de domínio.

### Mapeamento inicial de módulos

| Código atual | Destino candidato | Ownership/regra de decisão |
|---|---|---|
| `logging` | `core/logging` | Infraestrutura transversal; não possuir política de domínio. |
| `config` | `core/config` para parsing/configuração de processo; config de domínio no módulo dono | Não importar strategy. Conversão para tipos do domínio no composition root. |
| `error` | `core` apenas para falhas técnicas realmente transversais; erros de domínio locais | Conversões na fronteira de aplicação/presentation. |
| `persistence` | mecanismo técnico comum em `core`, se cumprir admissão; adapters e queries em cada domínio | Persistência pertence ao domínio quanto a schema, repositórios e política. |
| `exchanges` | `modules/exchanges` | REST/WS e regras de capacidade da integração; mecanismo compartilhado somente se cumprir admissão. |
| `market` | `modules/market` | Dono de conceitos e política de mercado. |
| `market_feed` | `modules/market` se mantém estado/semântica de feed (watermark, merge REST/WS, gaps) ou se seus consumidores são exclusivamente casos de uso de mercado; caso seja adapter sem política/estado do domínio, fica como adapter do consumidor/application owner | Confirmar por inventário de estado persistido, consumidores atuais e quem define política de merge/recovery; não classificar só pelo nome. Hipótese inicial: `modules/market`, sujeita a essa verificação antes da migração. |
| `strategy`, `risk`, `portfolio`, `backtest` | módulos homônimos | Donos de tipos/regras próprios; sem camadas vazias. `Signal`/modo pertencem a strategy conforme seams acima. |
| `app`, `monitor_startup` | composição em `main` para wiring; casos de uso/orquestração em `modules/monitor/application` | Separar construção e execução; preservar semântica operacional. |
| `persistence_health` | `modules/monitor/application` se representa readiness/health como política consumida pelo monitor/UI; adapter sob `core` só se for probe técnico neutro consumido por pelo menos dois módulos e cumprir admissão | Decisão baseada em estado que lê/escreve, consumidores e política de readiness; health não é automaticamente core por ser transversal/operacional. Hipótese: monitor/application para a agregação/política e probes no adapter dono. |
| `ui` | `presentation/terminal` | Interface TUI e tradução de entradas/saídas, sem lógica de domínio. |
| `jev` | `modules/jev` | Serviço/adapters consultivos; ser externo não o torna core. |
| `domain.rs` | tipos redistribuídos nos domínios proprietários | Eliminar imports de config/strategy no módulo universal; nenhuma migração mecânica sem ownership por tipo. |

### Migração e gates de aceitação

A migração será feita em fatias pequenas, cada uma com gate antes de avançar:

1. **Baseline**: registrar comandos de build/test existentes, snapshots/saídas observáveis relevantes da TUI/monitor/backtest e grafo atual de imports. Acordar seams públicos antes de escrever testes de migração.
2. **Seam e ownership**: para cada fatia, declarar tipos/traits públicos e módulo proprietário; eliminar o ciclo antes de mover arquivos. Nenhuma fatia pode introduzir `core -> modules/*`, dependência domain → config/presentation/adapter concreto ou ciclo entre domínios.
3. **Teste por fatia (TDD)**: criar/ajustar teste comportamental antes da mudança, observar falha quando a nova fronteira estiver ausente, mover/implementar o mínimo, e exigir green. Preservar testes existentes. Cobrir ao menos um fluxo público afetado por fatia e comparar comportamento/saídas da baseline, não apenas compilação.
4. **Dependency checks automatizados/revisáveis**: CI executa build/test e verificação do grafo de módulos/imports (script/check configurado para rejeitar as relações proibidas acima). Mudanças de allowlist exigem justificativa revisável e dono; revisão de diff confirma imports e ownership. Se ferramenta automatizada não conseguir analisar o crate/binário, gate exige relatório de grafo gerado/revisado e regra executável equivalente antes da fatia seguinte.
5. **Compatibilidade temporária**: reexports antigos são opt-in por fatia, marcados `deprecated` com destino e data/versão-limite de remoção. Cada PR registra inventário dos consumidores remanescentes; CI verifica que o inventário diminui/é zero para remoção. Não aceitar reexport sem owner, prazo e critério de expiração; remover no primeiro release interno em que consumidores migrarem, ou no máximo duas fatias após a migração do módulo. Reexports não devem perpetuar ciclo.
6. **Aceite da fatia**: build, testes específicos e suíte aplicável passam; checks de dependência passam; regressões comportamentais são comparadas; documentação/catálogo atualizado. Uma regressão ou exceção sem justificativa bloqueia a próxima fatia.
7. **Finalização**: remover caminhos/reexports antigos quando o check mostrar zero consumidores; reexecutar a suíte e auditoria do grafo completo; atualizar catálogo e guias em `backend/docs`.

Rollout é interno e incremental, sem deploy/alteração de dados; rollback de uma fatia consiste em reverter a mudança mantendo a última fronteira aprovada, sem mudanças de schema. Esta proposta não autoriza publicação ou execução em produção.

## Alternatives

### Manter a estrutura atual e documentar ownership

Manter arquivos e módulos, atualizar catálogo e adotar regras de import/ownership sem mover diretórios. É a opção de menor risco imediato e preserva a mudança incremental; não entrega a localização explícita em `src/modules/` nem separação visual de `core`. Permanece válida se a migração não superar seu custo.

### Agrupar somente em `src/modules/`, sem `core`

Agrupar domínios e deixar config/logging/banco dentro de módulos ou infraestrutura genérica. Reduz movimentos, mas deixa capacidades genuinamente compartilhadas sem proprietário único e não atende à intenção confirmada de centralizar somente serviços compartilhados. Uma implementação futura ainda pode preferir isso se nenhum serviço passar pela regra verificável de admissão.

### MVC estrito em cada módulo

Criar models/controllers/views para todo módulo. Rejeitada como convenção obrigatória por gerar camadas vazias ou pass-through; responsabilidades idiomáticas da proposta são domain, application/use cases, ports/adapters e presentation.

### Dividir em múltiplos crates

Oferece isolamento mais forte via Cargo, porém aumenta configuração, fronteiras de tipos e custo. Reavaliar se regras de dependência não puderem ser mantidas/revisadas no crate atual ou houver necessidade de versionamento/reuso independente.

### Não reorganizar agora

Adiar até necessidade funcional. Evita risco imediato e é preferível se não houver capacidade para migração com testes/gates. O custo é manter localização e ownership pouco evidentes.

**Critério de decisão:** adotar a reorganização se (a) inventário confirmar pelo menos um fluxo de desenvolvimento recorrente prejudicado pela localização/ownership atual e (b) as fronteiras propostas puderem ser mantidas por testes comportamentais e checks de dependência revisáveis sem alteração funcional. Caso contrário, escolher manter e documentar ownership (ou adiar), registrar donos e dependências no catálogo e reavaliar quando surgir evidência. Não reorganizar é uma alternativa válida, não uma falha por si só.

## Drawbacks

- Custo temporário de navegação, revisão e resolução de imports.
- Nomes como `market`, `monitor` e `presentation` podem não corresponder aos limites reais; os gates de ownership precedem os movimentos.
- `core` pode virar dependência global; admissão exige evidência e revisão contínua.
- Fronteiras sem MVC literal precisam de explicação consistente; nomes refletem responsabilidades Rust.
- Mudanças de caminho podem afetar testes, comandos, docs e tooling.
- Reexports temporários podem virar API paralela; gates de prazo, inventário e remoção controlam esse risco.
- A extração de casos de uso de `app` deve preservar concorrência, cancelamento e semântica de pausa/retomada.

## Unresolved questions

- Qual é o inventário real de estado, consumidores e política de `market_feed`? Confirmar se watermark/merge REST-WS/gap recovery pertencem ao domínio `market` ou se é adapter sem estado/política. Decisão de arquitetura/backend antes da fatia correspondente.
- `persistence_health`: quais dados consulta, quem consome o resultado e quem define readiness? Separar probe técnico de política/agregação do monitor conforme o mapeamento acima; confirmar antes de mover.
- Quais campos de `config` são operacionais e quais são política de domínio? Resolver por inventário campo-a-campo de consumidores e validações.
- `core/persistence` precisa oferecer apenas pool/migrações/transações genéricas ou há mecanismo adicional com uso real? Repositórios/queries de domínio permanecem nos donos; decisão com inventário de consumers.
- Como `BotError` se divide entre erros técnicos de processo, domínio e erros apresentados? Inventariar propagação/conversões e fronteira de exibição.
- Quais checks de grafo são viáveis com a estrutura atual do crate? Selecionar ferramenta/regra executável na especificação de migração; gate mínimo é relatório reproduzível e revisável.

## Referências locais

- [Catálogo completo de módulos do backend](../architecture/module-catalog.md)
- [Referência de módulos do backend](../architecture/backend-module-reference.md)
- [Integrações do backend](../architecture/integrations.md)
- [Documentação do backend](../index.md)
