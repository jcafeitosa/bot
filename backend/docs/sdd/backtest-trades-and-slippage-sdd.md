---
title: SDD — Fixture do backtest e custo da saída por sinal
description: Design da fixture de trades e do custo de saída do backtest
tags:
  - sdd
  - backend
  - backtest
---

# SDD — Fixture do backtest e custo da saída por sinal

- **ID:** T-07 (design para implementação posterior)
- **Autor:** System Designer Builder T-07
- **Revisor:** Crítico de Arquitetura independente
- **Estado:** G1 aprovado e seams públicos aprovados pelo usuário em 2026-09-26; C12 G3 aprovado por /root/c12_critic em 2026-09-26; C13 implementado, revisão G3 pendente
- **Data:** 2026-09-26

## Contexto e problema

O subcomando `backtest` gera candles sintéticos de 1 minuto, agrega-os ao timeframe configurado e executa SMA crossover. A geração atual usa `slow_period + 2` barras agregadas e altera o preço em um índice fixo de minuto (`70`). Isso satisfaz o mínimo de aquecimento e uma abertura seguinte, mas não assegura um cruzamento de compra e um de venda *após* o aquecimento em timeframes de 1m a 4h. O comando pode emitir `trades: 0`, de modo que o exemplo principal não demonstra um trade fechado. Em `run_sma_crossover`, a saída por sinal `Sell` ocorre na abertura da próxima barra, porém chama `close_position` com slippage `0.0`, enquanto compra, stops e liquidação final aplicam `config.slippage_rate`.

O objetivo é tornar o exemplo determinístico e corrigir a contabilização da saída por sinal sem alterar execução real, stops, formato do relatório ou estratégia.

## Objetivos e critérios de aceite

1. Com a configuração padrão válida e com cada timeframe permitido para os modos suportados (scalper 1m/3m/5m, day trader 5m/15m/30m, swing trader 1h/4h), o dataset sintético deve conter histórico suficiente para `run_sma_crossover` produzir ao menos um trade **fechado por sinal Sell**, com `metrics.trades >= 1` e `wins + losses == metrics.trades`.
2. O sinal é calculado somente em candles fechados e qualquer compra/venda por sinal continua a preencher na **abertura da barra seguinte**. O exemplo não depende de stops nem conta a marcação da posição aberta na última barra como trade fechado.
3. A venda por sinal aplica `config.slippage_rate` como preço adverso ao open da barra de execução seguinte. `BacktestReport.total_costs_quote` inclui taxas e slippage; teste do seam `run_sma_crossover` usa uma trade totalmente controlada para calcular explicitamente slippage da venda, taxa na base efetiva pós-slippage, proceeds e resultado total. A comparação com slippage zero deve considerar que a taxa calculada sobre proceeds também muda; não usar `gross_profit_quote` isoladamente como oráculo de custo. Stops, take profit, trailing stop e marcação final conservam sua política atual.
4. O tamanho da fixture e os timestamps são calculados com aritmética verificada. Entrada que não possa ser representada ou alocada dentro do limite explícito da fixture retorna erro de configuração, sem saturação silenciosa nem overflow. Os timeframes atuais cabem no limite.
5. A alteração não muda campos/tipos de `BacktestReport` nem o JSON resumido do CLI; o resumo atual não expõe `total_costs_quote`. A mudança de custo é observável pelo report/API em teste, não pelo JSON do CLI. O README descreve a fixture, `trades` e a limitação de custo no resumo do CLI após implementação.

## Seams públicos propostos para acordo do usuário antes dos testes

1. `backend` binário, `backtest --config <arquivo>`: mesma interface e JSON atual; uma configuração válida de preset/timeframe produz `trades >= 1` sobre a fixture sintética determinística. `--persist` continua opcional e grava exatamente a fixture usada no run.
2. `pub fn run_sma_crossover(&HistoricalDataset, &StrategyDefinition, Timeframe, OperationMode, BacktestConfig, RunId) -> Result<BacktestReport, BacktestError>`: mesma assinatura. Um `Sell` detectado no fechamento da barra `i-1` encerra uma posição na abertura da barra `i`, com preço de venda `open[i] * (1 - slippage_rate)`, taxa sobre proceeds e custo de slippage contabilizado. `BacktestReport` permanece o contrato de observação (trades, custos, lucro e curva).
3. `BacktestConfig`/`ExitPolicy`/`BacktestError`: sem novos campos ou variantes para a correção comum. Falhas de tamanho da fixture no CLI usam `BotError::Configuration`; `run_sma_crossover` preserva seus erros públicos para dados/aritmética inválidos.

`synthetic_dataset` e a função de cálculo de tamanho continuam privadas em `backtest_cli.rs`; seu desenho interno pode mudar sem introduzir um novo seam público. O usuário aprovou as interfaces deste e dos outros três SDDs em 2026-09-26; os seams acima passaram a autorizar os testes de C12/C13.

## Desenho

### Fixture em barras agregadas

Construir um percurso em **barras do timeframe alvo**, depois expandir cada barra para `timeframe.minutes()` candles contíguos de 1 minuto, alinhados a zero. Uma forma suficiente para SMA `fast < slow` é:

- `slow + 1` barras constantes no preço base positivo (por exemplo, 100): ambas as médias existem e empatam no primeiro ponto elegível.
- Uma barra de impulso maior que a base (por exemplo, 110): no seu fechamento, a SMA rápida fica acima da lenta e emite `Buy`.
- `fast + 1` ou mais barras na base: a compra é executada na abertura da primeira; quando o impulso sai da janela rápida e ainda está na lenta, ocorre `Sell`; há uma barra **adicional** para executar a venda na sua abertura.

Derivar e validar índices contra o loop real de `run_sma_crossover`: no índice de loop `i`, a estratégia recebe `bars[..i]`, então o sinal gerado pelo fechamento da barra `j` só é conhecido no passo de execução `i = j + 1`, e esse passo preenche no open de `bars[i]`. Para uma fixture com impulso em `p`, primeiro localizar por teste a primeira compra e o fechamento Sell efetivamente detectados pela função `evaluate` existente; em seguida assegurar que há uma barra de execução seguinte disponível para cada sinal. Não presumir a contagem `slow + fast + 3` sem provar todos os índices, inclusive o primeiro sinal elegível. Testar o menor número de barras que efetivamente produz compra e Sell fechado, e parametrizar nos timeframes permitidos. A expansão gera OHLCV finitos, coerentes, preços positivos e timestamps `i * 60_000`; não usa fonte externa.

Usar `checked_add`, `checked_mul` e conversões verificadas para a quantidade de barras/minutos e timestamp máximo. Aplicar um teto documentado para a fixture CLI (suficiente para o máximo atual, `73 * 240 = 17.520` candles de 1m com SMA 20/50) antes de alocar. Um erro claro substitui a saturação atual. Escolher o teto no Builder de modo proporcional, por exemplo 20.000 candles, e cobrir seu limite. O dataset continua `HistoricalDataset::from_1m(..., "synthetic-cli", ...)`, e `resample(timeframe)` valida os buckets completos.

### Venda por sinal

No ramo `Signal::Sell && base_position > 0`, passar `config.slippage_rate` ao `close_position` existente, como já ocorre para exits intrabar. `close_position` já calcula `fill_price = exit_price_before_slippage * (1 - slippage)`, aplica a taxa e soma a diferença ao custo total. Essa alteração é localizada e evita duplicar cálculo. Preservar a ordem do loop: sinal e preenchimento na abertura; depois gestão de risco dentro da mesma barra. Não mexer na precedência atual de triggers intrabar neste item.

## Alternativas e decisões

- **Aumentar apenas o número de minutos da curva atual:** rejeitado. A inflexão fixa em minuto 70 muda de posição relativa ao aquecimento e pode desaparecer na agregação de 4h; não garante venda.
- **Gerar um ciclo por timeframe com barras agregadas e expansão 1m:** escolhido. Permite demonstrar o contrato de resample, warmup e next-open com dados pequenos e determinísticos.
- **Forçar `trades = 1` ou contar liquidação final como trade:** rejeitado. Mascara a ausência de sinal fechado e altera métricas.
- **Adicionar uma flag nova de slippage para sinal:** rejeitado. `BacktestConfig::slippage_rate` já exprime o custo da execução, e a distinção atual é um defeito de aplicação.

## Riscos, limites e custos

- A fixture é ilustrativa e sintética, não representa mercado, retorno esperado ou validade da estratégia. Os valores de P&L mudam conforme a correção do slippage e o hash do dataset muda com novos candles; documentar no README. A persistência `--persist` passa a armazenar a nova fixture sob outro `dataset_id`, sem migração de schema.
- O tempo e memória da execução CLI passam a escalar com `(slow + fast + 3) * timeframe.minutes()`, limitados pelo teto. O máximo dos presets atuais é 17.520 candles, além das barras e curva em memória.
- `close_position` acumula custos em `f64`; um teste deve verificar por tolerância numérica e isolar a venda por sinal para não confundir taxa, compra ou stop. Nenhuma alegação de precisão financeira em produção decorre deste reparo.
- `metrics.trades` representa posições encerradas durante o loop. A marcação hipotética de inventário aberto no último close não incrementa esse campo. Esse comportamento permanece.

## Validação e sequência TDD após G1 e acordo dos seams

1. **Fixture (C12):** teste comportamental do CLI com configuração padrão e cada timeframe permitido, usando entrypoint real do binário e sem `--persist`, sem exigir banco. Verificar JSON `trades >= 1`; por teste de `run_sma_crossover` com a fixture, provar que a saída fechada ocorreu por sinal `Sell` (não liquidação final) e verificar next-open. Derivar o menor tamanho da fixture com base em `bars[..i]`, produzir inicialmente um teste que falha e então implementar o mínimo. Testar limites e overflow por seam/helper real. `--persist` conecta a PostgreSQL, executa migrações e grava dados; qualquer teste desta opção é um teste de integração separado com uma instância/database PostgreSQL descartável, dedicada e isolada, configurada explicitamente para o teste. É proibido apontar testes para databases compartilhadas, de desenvolvimento, `trading_bot` operacional ou produção. O harness deve validar o nome/host/opt-in antes de executar migrations; o teste não será descrito como verificado sem executar e registrar esse setup isolado.
2. **Slippage de Sell (C13):** dataset pequeno com crossover Buy/Sell, stops desabilitados, fee controlada e preços de abertura distintos dos fechamentos. Rodar `run_sma_crossover` com slippage zero e positivo; verificar custo, proceeds/lucro, número de trades e abertura da barra seguinte por equação calculada no teste. O teste inicial falha porque o ramo Sell usa zero (**red**); a correção mínima é passar `config.slippage_rate` (**green**). Um teste de regressão confirma que saída por stop/take profit conserva slippage e timing atuais. Evitar teste que replique `close_position` linha a linha.
3. Atualizar README junto de C12/C13, revisar o diff, `cargo fmt --check`, `cargo test --locked`, `cargo clippy --all-targets -- -D warnings` e `git diff --check`. Registrar resultados observados, não presumidos. A integração PostgreSQL exige banco descartável e pertence a gate separado quando disponível.

Cada CL tem Builder e Crítico independentes, até três ciclos, conforme `AGENTS.md`. Segurança de trading: nada neste design habilita ordens ou produção. Jev: não aplicável — sinais SMA e custos são regras determinísticas verificáveis, sem decisão classificatória útil.

## Rollout e rollback

O rollout é apenas código local do subcomando e documentação; não há serviço nem deploy previsto. Reverter C12 restaura a fixture anterior e seu `dataset_id`; reverter C13 restaura os cálculos antigos, portanto regressa o erro de custo na venda por sinal. Não há migração de banco nem mudança de formato persistido; datasets sintéticos antigos e novos podem coexistir por hash. Executar backtest com config padrão após ambos para registrar `trades`, `wins`, `losses` e custos reais antes de considerar a entrega concluída.

## Evidência de C12 (G3 aprovado por /root/c12_critic)

- O teste do binário `backtest --config` falhou em red com `scalper/1m` e `trades: 0`; após a mudança passou nos oito pares permitidos de operação e timeframe, com `trades >= 1` e `wins + losses == trades`.
- A fixture final tem `(slow + fast + 3) * timeframe.minutes()` candles de 1m. A análise e o teste observam Buy no open da barra 22 e Sell no open da barra 27 para SMA 5/20; sem taxas, slippage ou stops, `run_sma_crossover` fecha exatamente um trade com P&L zero. Ao mudar somente a última barra para `open=95`, mantendo `close` anterior em 100, o mesmo sinal Sell fecha um trade com P&L -5; isso distingue preenchimento no próximo open de execução no fechamento do sinal. Sem a última barra agregada, nenhum trade é fechado.
- O teste de limite cobre 17.520 candles para 4h/SMA 20/50, o teto inclusivo de 20.000 e erro de configuração acima dele ou em overflow. O comando padrão 15m retornou `trades: 1`, `wins: 0`, `losses: 1`, `dataset_id: fnv1a64:4c0b6491c373e08c`.
- Na conclusão de C12, o slippage da venda por sinal permanecia para C13; a evidência de C12 não aprova C13 nem representa teste de persistência PostgreSQL.

## Evidência de C13 (revisão G3 pendente)

- O teste público `run_sma_crossover` com Buy no open 101 e Sell no open 95, após fechamento anterior em 100, falhou antes da correção: com slippage de 1%, lucro bruto observado -7,05813155572983 versus -7,9875502401725385 calculado com a venda adversa. A taxa de 0,2% foi calculada sobre proceeds efetivos. Após passar `config.slippage_rate` ao fechamento do ramo Sell, o mesmo teste passou para slippage zero e 1%, conferindo trades, lucro líquido e bruto, custos agregados e equity final por tolerância de 1e-9.
- Um teste separado de stop loss e take profit forçados na barra 23 confirmou saída na própria barra de trigger, slippage de 1%, taxa sobre proceeds pós-slippage e custo agregado; ambos passaram. O Sell por sinal da fixture continua na barra 27, de modo que esse teste distingue o exit intrabar do ramo corrigido.
- O resumo JSON do CLI continua sem `total_costs_quote`. O custo está disponível em `BacktestReport`; a correção altera o P&L do comando padrão, sem alterar o contrato JSON nem o identificador da fixture.
- Após C13, `cargo run --locked -- backtest --config src/config/bot.toml` retornou `trades: 1`, `wins: 0`, `losses: 1`, `net_pnl_quote: -0.09982519980019333`, `dataset_id: fnv1a64:4c0b6491c373e08c`. `cargo fmt --check`, Clippy com `--all-targets -- -D warnings`, `git diff --check` e a suíte sem testes HTTP locais passaram (74 unitários, um de fixture, dois de configuração e três de política de origem; um PostgreSQL ignorado). A suíte completa falhou apenas nos dois testes HTTP locais de C9 porque o sandbox negou a criação de sockets (`Operation not permitted`); esse gate permanece separado de C13.
