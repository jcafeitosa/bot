---
title: SDD T-16 — Mapa de módulos do backend no README
description: Design do mapa de módulos do backend no README
tags:
  - sdd
  - backend
  - architecture
---

# SDD T-16 — Mapa de módulos do backend no README

- **Autor:** System Designer Builder `/root/module_map_designer`
- **Crítico designado:** `/root/module_map_design_critic` (instância independente)
- **Estado:** G1 e G3 aprovados por `/root/module_map_design_critic`; seção `Architecture` de `backend/README.md` concluída
- **Data:** 2026-09-26

## Contexto e objetivo

O usuário pediu uma explicação do que o backend faz, a responsabilidade de cada módulo e se a organização corresponde ao código. A seção `Architecture` de `backend/README.md` enumera alguns arquivos, mas não cobre todos os 16 módulos declarados em `src/main.rs` nem os 13 submódulos de `src/exchanges/mod.rs`. A lista atual também aproxima modelos e catálogos do caminho operacional, sem distinguir claramente o que realmente executa no monitor.

**Objetivo:** substituir essa seção por um inventário navegável e completo dos módulos declarados, seus limites e os caminhos de dados efetivamente implementados. A documentação deve representar a árvore atual, não o design ainda pendente. **Não objetivos:** mover arquivos, alterar API/comportamento, implementar designs pendentes, afirmar prontidão de negociação ou de produção.

## Superfície e desenho documental

O único artefato de implementação de T-16 será a seção `Architecture` de `backend/README.md`. Não há seam público de código ou configuração novo; comandos, interfaces e estrutura de arquivos permanecem iguais. Organizar a seção em:

1. **Entrada e fluxos reais.** `main.rs` escolhe monitor ou `backtest`. No monitor: `config` → `app` → `exchanges/bootstrap` e `registry` → `binance` REST e, apenas em `1m`, `live` WS → `market_feed` → `strategy` → `risk` e `jev` opcional → `ui`; `market`/`persistence` guardam candles quando o opt-in atual está ativo. No backtest: `backtest_cli` cria dataset sintético 1m → `market` valida/agrega → `backtest` simula SMA, posições e custos → resumo JSON; `persistence` é opcional via `--persist`. Sinal do monitor não vira ordem real.
2. **Tabela de módulos raiz (16).** Uma linha por `app`, `backtest`, `backtest_cli`, `config`, `domain`, `error`, `exchanges`, `jev`, `logging`, `market`, `market_feed`, `persistence`, `portfolio`, `risk`, `strategy`, `ui`, cada qual com caminho e responsabilidade curta. `domain` contém identidades, métricas/rankings e `BotSignal`; o monitor usa `BotSignal`, mas ranking não executa no loop. `portfolio` contém modelos/validação e snapshot de papel; o monitor apenas cria e consulta um snapshot de exemplo, sem gerir carteira ou enviar ordens. `risk` calcula limites e gate de intenção de sinal, sem execução externa. `error` concentra `BotError`/`BotResult`.
3. **Tabela dos 13 submódulos de `exchanges`.** `account_file` carrega/valida TOML e origem Spot; `bootstrap` monta registro e seleciona conta; `registry` mantém contas; `mod.rs` define IDs, tipos, transportes e erros compartilhados; `binance` busca e valida candles REST; `market_data` define o trait; `live` planeja e mantém WS de kline fechado; `rest` autoriza usos REST; `router` mapeia necessidades para transportes; `preflight` apenas registra plano/catálogos e amostras de diagnóstico; `capabilities` descreve capacidades; `resources` descreve recursos gerenciados; `stream` e `ws` modelam eventos, inscrições e parâmetros WS. Distinguir os modelos de inscrições/capacidades do único feed WS efetivamente iniciado (`live` em 1m). Futures pode estar registrado, mas o monitor seleciona Spot; rotas e catálogos de ordens não significam que envio esteja ligado.
4. **Configuração e dados.** Referenciar os caminhos ativos `src/config/bot.toml`, `src/config/exchanges/binance.toml` e `src/persistence/migrations/`; não ressuscitar as cópias legadas removidas.
5. **Limites conhecidos e trabalho pendente.** Manter explícito que REST pode seguir redirect fora da origem inicial (T-05), a fixture CLI pode produzir zero trades e a saída Sell omite slippage configurado (T-07), a pausa/retomada pode avaliar candles antigos (T-10), e o opt-in PostgreSQL ainda segue semântica best effort (T-15). Apontar para os quatro SDDs respectivos e marcar cada proposta como não implementada. Não apresentar os contratos desses SDDs como comportamento atual.

As tabelas são documentação de responsabilidade, não interfaces prometidas. Preferir links relativos ao README para módulos e SDDs existentes. Se um arquivo tiver papel misto, descrever sua função no fluxo e a parte apenas declarativa, sem inferir uso por nome.

## Alternativas e decisão

| Alternativa | Decisão | Motivo |
|---|---|---|
| Manter a lista parcial atual | Rejeitada | Não responde à pergunta sobre todos os módulos e confunde modelo com execução. |
| Criar inventário em arquivo separado e só linkar | Rejeitada por ora | A seção `Architecture` já é a entrada natural do leitor; duplicaria navegação para um mapa curto. |
| Tabelas completas no README, agrupadas por raiz e exchanges | Escolhida | Cobre cada declaração, preserva caminhos e permite verificar omissões. |

## Validação e gates

**G1:** Critic independente confronta este desenho com `main.rs`, `exchanges/mod.rs`, `app.rs`, `backtest_cli.rs` e os módulos citados; corrige alegações de execução excessivas. **G2:** Orquestrador atribui Builder e Critic separados para a edição do README. **G3:** Builder atualiza apenas `Architecture`; Critic compara o inventário final com as 16 declarações `mod` e as 13 `pub mod`, confirma caminhos e links existentes, confronta as descrições com os call sites e verifica que T-05/T-07/T-10/T-15 permanecem pendentes. Executar `git diff --check` e inspeção do diff. Não há teste red/green de código: a entrega não muda comportamento executável; a validação observável é o conjunto exato de nomes declarados igual ao conjunto documentado, mais links resolvidos e revisão factual independente. Não alegar que `cargo test` comprova um mapa documental.

**Riscos e reversão:** o mapa pode ficar desatualizado quando módulos forem adicionados ou os SDDs implementados. Uma alteração futura de `main.rs` ou `exchanges/mod.rs` deve atualizar a tabela na mesma entrega. A reversão é a da seção documental; não há migração, deploy ou mudança de runtime. Jev não acrescenta valor à checagem determinística de nomes e caminhos (`Jev: não aplicável`).

**Resultado G3 (2026-09-26):** `/root/module_map_design_critic` aprovou a seção `Architecture` do README. A verificação encontrou 16/16 módulos raiz e 13/13 submódulos `exchanges`, sem faltas ou extras; os links relativos da seção resolvem e `git diff --check -- backend/README.md` passou.
