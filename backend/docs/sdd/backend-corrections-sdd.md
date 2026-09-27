---
title: SDD — Correções de configuração, mercado e organização do backend
description: Design técnico das correções de configuração, mercado e organização do backend
tags:
  - sdd
  - backend
  - market-data
---

# SDD — Correções de configuração, mercado e organização do backend

- **ID:** T-03
- **Estado:** Proposta; G1 não está comprovadamente aprovada nesta revisão
- **Data:** 2026-09-26
- **Escopo:** configuração, validação de dados de mercado, políticas REST públicas, avaliação REST/WS e testes de regressão do monitor/backtest

## Contexto e limites

O backend atual é um monitor/backtest Rust. O monitor lê candles públicos da Binance, calcula SMA, aplica limites a intenções hipotéticas, pode solicitar um parecer JEV e mostra a TUI. Este SDD não habilita ordens, balances privados, credenciais de conta, produção, futures, HFT ou capital real.

As alterações de código existentes foram commitadas em `6a103fa` (`fix(backend): validate market feed and config inputs`). A stack local de dados foi commitada separadamente em `c72ed0c`. Os commits e testes demonstram apenas o conteúdo atual e seus checks; não comprovam por si só aprovação anterior dos seams nem o processo histórico de design/revisão.

## Objetivos propostos

1. Fazer monitor e backtest falharem com erro claro quando o arquivo de configuração padrão ou explicitamente informado estiver ausente ou ilegível.
2. Restringir o caminho de candles públicos Spot `dev` a endpoints testnet explicitamente permitidos.
3. Rejeitar janelas REST e eventos WS inválidos antes de estratégia, feed ou persistência.
4. Fazer o feed preservar warmup, deduplicar timestamps e reconciliar WS/REST sem reavaliar candle já consumido.
5. Manter a avaliação e apresentação do monitor funcionando com REST quando o canal WS encerra ou não é disponibilizado, conforme comportamento implementado e testado.
6. Preservar os gates de ordens e o comportamento público do Binance/JEV sem transmitir segredos para caminhos que não os exigem.

## Contratos que o código atual procura implementar

Estes descrevem o estado observado no commit `6a103fa`; são evidência da implementação, não registro de acordo prévio do usuário.

- `Config::load` lê o caminho selecionado e retorna erro de configuração para arquivo ausente/ilegível, incluindo o caminho de leitura. Monitor e backtest chamam seus respectivos loaders; `backend/tests/config_cli.rs` exercita os erros de config padrão/explicitamente informada e diretório ilegível pelo binário real.
- O registro de conta passa endpoints configurados ao adaptador. O adapter público REST configura o endpoint Spot `dev` de testnet e valida a URL efetiva. O plano WS deriva seu stream da base configurada, e o parser valida evento fechado, símbolo, intervalo e invariantes do candle.
- O REST é permitido somente como backfill público histórico para a conta Spot `dev` sob a policy `RestUse`; chamadas de ordem/estado/cancelamento e usos privados permanecem rejeitados. A janela completa é validada e rejeitada se contiver candle inválido, timestamps desalinhados ou duplicatas conflitantes.
- O feed verifica histórico contíguo suficiente, não avança o marcador em warmup, mantém a avaliação monotônica e evita disparar novamente para candle já avaliado.
- `app.rs` centraliza settings de avaliação e publica refresh/status de warmup no caminho do ciclo; testes determinísticos cobrem ciclo paper, falha de REST, feed WS/REST, fechamento/ausência de canal WS e deduplicação. Eles exercitam funções/canais reais do módulo, sem simular uma sessão completa de TUI nem uma conexão real à Binance.

## Segurança e limites conhecidos

O allowlist verifica a origem inicial configurada; **não há hoje uma política que impeça o cliente HTTP `ccxt` de seguir redirects para outra origem**. Assim, redirect cross-origin continua risco residual de rede. Não declarar redirect externo bloqueado. Um SDD separado T-05 propõe resolver isso por alteração/patch do cliente HTTP; seu G1 está tecnicamente aprovado por Crítico independente, enquanto o acordo do usuário sobre os seams e G3 permanecem pendentes.

A requisição atual de OHLCV é pública. O caminho de ordem permanece bloqueado e nenhuma alteração nesta implementação concede permissão para ordens, saldo privado ou uso de credentials para OHLCV. A presença de chaves Binance de testnet em `backend/.env` não significa que o monitor as utilize nesse caminho público.

## Verificação executada para o commit `6a103fa`

Resultados capturados durante esta execução:

- `cargo fmt --check`: passou.
- `cargo check --tests`: passou.
- `cargo test -- --nocapture`: 70 testes passaram, 1 teste PostgreSQL foi ignorado porque requer `DATABASE_URL`, e os 2 testes de integração `config_cli` passaram.
- `cargo clippy --all-targets -- -D warnings`: passou.
- Crítico independente emitiu `APPROVED WITH FOLLOW-UP` para o diff de código: redirect permanece risco; teste de integração PostgreSQL não executado; fluxo monitor completo com rede/TUI não testado. Relatório: `grok-backend-dirty-review-v2.md` fora do repositório.

O teste de persistência `persistence::integration_tests::persist_dataset_round_trip` não foi executado. Nenhum teste de conexão de trading/ordem, WS Binance real ou produção foi alegado. O processo de testes não substitui aprovação de design.

## Situação de G1 e seams

O usuário aprovou explicitamente as sete interfaces propostas neste SDD na resposta “Aprovo as 7 interfaces propostas (recomendado)”. Esse acordo autoriza os seams para os testes correspondentes, mas não fornece evidência verificável de aprovação histórica de G1 nem, por si só, comprova a conclusão ou revisão de C0–C8. A implementação C0–C8 não incluiu remoção de arquivos legados, e este documento não declara os gates de C0–C8 aprovados; G1 permanece pendente de comprovação ou revisão própria. A remoção posterior é registrada separadamente como T-13 abaixo.

Os arquivos legados `backend/exchanges/config/binance.toml`, `backend/migrations/0001_market_data.sql` e `backend/src/config/profiles.toml` foram removidos na entrega T-13, com design aprovado em `backend/docs/sdd/legacy-file-cleanup-sdd.md`. Permanecem como fontes ativas `backend/src/config/exchanges/binance.toml`, `backend/src/persistence/migrations/0001_market_data.sql` e os presets definidos em Rust. A limpeza documental não altera o estado dos demais gates deste SDD.

## Entregas possíveis após revisão

1. Confirmar os contratos observáveis acima e quaisquer diferenças desejadas.
2. Aprovar o design/risco residual de REST redirect ou manter o caminho Binance não habilitado até uma solução revisada.
3. Executar integração PostgreSQL em database descartável explícito; não usar `trading_bot` para testes destructive.
4. Testar uma execução monitor/backtest end-to-end que não dependa de credentials nem realize efeitos de trading.
5. Só então declarar os gates correspondentes concluídos, com evidência por entrega.

## Não objetivos

- Implementar o harness de agentes descrito no SDD de runtime separado.
- Habilitar provider LLM/9Router, agentes 24/7, memória de agente, bot training ou autonomia financeira.
- Habilitar ordens de testnet/produção, leitura de saldo privado, futures ou HFT.
- Alterar a configuração de contas, os presets em Rust ou a migração ativa como parte da limpeza T-13.
