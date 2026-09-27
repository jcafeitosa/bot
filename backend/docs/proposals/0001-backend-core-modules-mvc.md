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

**Mudança observável:** ownership explícito de cada módulo atual, direção de imports verificável, interfaces de aplicação tipadas entre TUI e monitor, sem alteração de comportamento.

A inspeção observada do mapa de código identifica `OperationMode` compartilhado entre config/CLI/monitor e strategy/domain; `Signal` produzido/consumido por strategy, risk, domain/backtest e app; `market_feed` concentra REST/WS e invariantes de deduplicação, ordenação e watermark; `persistence_health` é um check operacional PostgreSQL consumido pelo monitor; a UI Ratatui comunica por `AppEvent`/`UiCommand`/`Dashboard`. Esses usos sustentam ownership mais preciso que uma pasta genérica `core`.

### Não objetivos

- Mudar regras de trading, risco, estratégia, persistência ou comportamento da TUI.
- Alterar schema SQL, habilitar operações ou dividir o crate.
- Impor MVC a módulos sem interação/presentação.

## Design

### Destino e seams de tipos

```text
backend/src/
├── main.rs                         # composition root
├── core/
│   ├── config/                     # parsing/configuração de processo; sem política de domínio
│   ├── error.rs
│   ├── logging.rs
│   └── persistence/                # pool, migrations e transações genéricas
├── modules/
│   ├── application_contracts.rs    # contratos neutros somente para conceitos compartilhados
│   ├── config/                     # OperationMode e resolução da configuração de processo
│   ├── strategy/                   # cálculo de períodos e produção de sinais
│   ├── risk/
│   ├── market/                     # market_feed e invariantes dos streams
│   ├── exchanges/
│   ├── portfolio/
│   ├── backtest/
│   ├── monitor/                    # casos de uso/estado/eventos operacionais
│   └── jev/
└── presentation/terminal/          # adapter Ratatui: Dashboard, input e rendering
```

A árvore é uma organização por responsabilidade Rust (`models`, `services`, `commands`, `events`, `views` quando úteis), não uma exigência de pastas MVC vazias. `main.rs` constrói dependências; apresentação não compõe domínio diretamente.

**Seams concretos e acíclicos:**

- `config` é dono de `OperationMode`, pois representa opção selecionável da aplicação e CLI. `strategy` expõe `periods_for_mode(mode: OperationMode) -> Vec<Period>` como serviço de estratégia e importa o tipo por contrato estável. `config` não importa `strategy`: validação/configuração não calcula política de estratégia; `main` ou monitor compõe ambos. O seam é `modules::config::OperationMode`.
- `Signal` é contrato de aplicação/domain realmente compartilhado por strategy, risk, backtest e app. Movê-lo para `modules::application_contracts::Signal` (tipo neutro de dados, sem dependência de strategy/config) evita que `domain` importe `strategy` e evita ciclo. Strategy o produz; risk e backtest o consomem. Não colocar regras de geração no contrato. Alternativa: strategy ser dono exigiria que risk/backtest/app dependessem da estratégia produtora, acoplando consumidores e aumentando risco de ciclos; contrato neutro custa uma abstração central e só se justifica por esses consumidores observados.
- `OperationMode` não é duplicado como tipo separado no contrato: `domain`/strategy importa `modules::config::OperationMode`. Configuração é raiz conceitual do modo, embora isso faça módulos de domínio dependerem de um tipo de opção da aplicação; evitar o ciclo mantendo config sem import de domínio e limitar o tipo a enum simples, sem parser/IO/config loader. Se um contrato de modo de domínio divergir no futuro, separar mediante mudança explícita e conversão no composition/application layer.

### Configuração e persistência

`core::config` limita-se a leitura, parsing, defaults e validação estrutural de parâmetros de processo. Configuração específica de exchange/estratégia permanece no módulo dono; políticas de domínio não migram para `core`. `core::persistence` oferece pool PostgreSQL, migrations e primitivas/transações neutras; queries, repositórios, schema e políticas de escrita/leitura ficam com módulos consumidores. Nenhum módulo de domínio é importado por `core`.

`market_feed` pertence a `modules::market`: sua responsabilidade observada é combinar REST/WS e manter invariantes de deduplicação, ordenação e watermark; monitor é consumidor, não proprietário dessas invariantes. `persistence_health` pertence a `modules::monitor`: é política/check operacional PostgreSQL apresentado pelo monitor e usa pool/interface de `core::persistence`, sem política SQL ou dependência inversa do core.

### Monitor e TUI

`modules::monitor` expõe interface de aplicação pública pequena e tipada (nomes orientados a responsabilidades; sem `Controller` genérico):

- `MonitorCommand` contém ações como `Pause`, `Resume`, `Refresh` e `Shutdown` (e seleção explícita de backtest quando aplicável).
- `MonitorEvent` informa `Paused`, `Resumed`, snapshots/atualizações de estado, resultado/erro operacional e conclusão.
- `MonitorHandle::send(command)` e `MonitorHandle::subscribe() -> Receiver<MonitorEvent>` são o seam de coordenação; o monitor mantém `MonitorState` e traduz comandos em chamadas aos módulos.
- `presentation::terminal::Dashboard` renderiza `MonitorState`/eventos e traduz teclas em `MonitorCommand`. Não importa `strategy`, `risk`, `market`, `persistence` nem tipos de domínio diretamente. `AppEvent`/`UiCommand` existentes podem ser adaptados para estes contratos durante a migração; a UI é view/adapter, não dona do estado de execução.

Nomes e assinaturas finais de canal (mpsc/broadcast) são definidos na implementação, preservando cancelamento, concorrência e semântica atuais.

### Direção e admissão observável do core

```text
main (composition) -> core, modules/*, presentation/terminal
presentation/terminal -> modules::monitor application API only
modules::monitor -> core + modules/* (orchestration)
modules/* -> core (when needed) + neutral contracts
core -> external libraries only; never modules/* or presentation/*
modules/* never -> presentation/*
```

A admissão de `core` é verificada por matriz módulo-consumidor: cada item registra consumidor(es), semântica comum e ausência de política de domínio. Uma capacidade usada por um único módulo não entra no core. `OperationMode` e `Signal` são exceções de seam com ownership explícito acima, não autorização geral para `core` conhecer domínio.

Cada slice inclui `cargo check` e `cargo test` dos alvos afetados; slices de fluxo incluem testes de integração observáveis. Executar também suites existentes e regressões de pause/resume/backtest: resultado de pausa e retomada, cancelamento/ordenação de eventos, resultado semântico de backtest permanecem iguais. Sem ciclos de dependências: verificação do grafo de módulos/imports em script customizado ou dependency-check e `cargo deny` conforme suporte e política do repositório; `cargo deny` cobre dependências de crates; script ou dependency-check cobre direção interna Rust, não presumir que um só substitui o outro.

### Migração e gates por fatia

1. Inventariar imports e consumidores e registrar a matriz de core/ownership; testar script de direção em fixture com dependência proibida (deve falhar) e permitida (passar). Gate: matriz cobre todos os módulos raiz atuais.
2. Extrair `OperationMode` para config e `Signal` para contrato neutro; adaptar `periods_for_mode`; testes unitários dos tipos/serviços e `cargo check -p bot` (ou `cargo check` do workspace real). Gate: nenhum import circular e consumidores compilam.
3. Mover `market_feed` para market e check de saúde para monitor. Testes determinísticos para deduplicação/ordenação/watermark e resultado saudável/indisponível de persistence health; teste de integração do consumidor monitor. Gate: invariantes anteriores preservadas.
4. Extrair interface monitor e adapter TUI em slice vertical; testes de comandos Pause/Resume/Shutdown e eventos/estado, além de smoke/integration de terminal sem alterar semântica observada. Gate: terminal só importa API monitor; suites pause/resume e backtest passam.
5. Mover restantes módulos e remover estrutura antiga; `cargo check`, todas as suites `cargo test`, script de direção/ciclos e `cargo deny` passam. Gate final: nenhuma mudança de comportamento; atualizar referências/documentação.

Reexports de caminho antigo são ponte temporária apenas durante a slice em andamento; removê-los antes do merge daquela slice. Exceção máxima: um conjunto enumerado no plano da slice seguinte, com proprietário, lista exata e remoção no final dessa única slice; sem API de compatibilidade indefinida. Cada gate verifica ausência de reexports expirados.

### Critério de escolha: reorganizar ou manter

Comparar baseline e candidato com a mesma matriz dos módulos raiz atuais. Reorganizar somente se, no protótipo/migração planejada: (1) 100% dos módulos raiz tiverem um único owner; (2) todos ciclos/imports proibidos forem removidos e bloqueáveis automaticamente; (3) nenhum diretório MVC vazio for introduzido; (4) todos os testes existentes e regressões de pause/resume/backtest passarem sem mudança semântica; e (5) não houver dependência de reexports além do prazo acima. Caso qualquer métrica falhe, manter a árvore atual e documentar ownership/import rules é preferível até correção; custos de movimentação sem esses resultados não justificam reorganização.

## Alternatives

- **Manter a árvore e documentar ownership:** menor risco. Preferir se não alcançar todas as métricas de aceitação; não exige mover só para obter regra de imports verificável.
- **MVC estrito em cada módulo:** rejeitado por criar camadas vazias/pass-through; adotar nomes Rust por responsabilidade.
- **Vários crates:** isolamento mais forte, mas maior custo de configuração e fronteiras; reavaliar se regras internas não puderem ser verificadas no crate.

## Drawbacks e riscos

- Mover caminhos causa churn e pode afetar testes/tooling; gates por slice e reexports com prazo reduzem, mas não eliminam esse risco.
- Contrato neutro `Signal` centraliza um tipo usado por vários consumidores; mantê-lo sem lógica para evitar core de domínio.
- Módulos de domínio importarem `OperationMode` de config é dependência conceitual deliberada e limitada; sem loader/parser no tipo compartilhado.
- Interface monitor precisa preservar concorrência, cancelamento e semântica de pause/resume; falha de regressão bloqueia merge.

## Referências locais

- [Catálogo completo de módulos do backend](../architecture/module-catalog.md)
- [Referência de módulos do backend](../architecture/backend-module-reference.md)
- [Integrações do backend](../architecture/integrations.md)
- [Documentação do backend](../index.md)
