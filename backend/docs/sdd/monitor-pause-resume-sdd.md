# SDD T-10 — Pausa e retomada do monitor de candles

- **Autor:** System Designer Builder `/root/resume_designer`
- **Revisor:** Crítico de Arquitetura `/root/resume_design_critic`
- **Data:** 2026-09-26
- **Estado:** G1 técnico aprovado por `/root/resume_design_critic`; acordo do usuário sobre seams e G3 pendentes
- **Escopo:** monitor TUI do backend Rust em `dev`, `observe`/`paper`; nenhuma ordem é habilitada

## Contexto e problema

O monitor usa um canal limitado a 64 candles fechados para receber WS, com `send().await` no produtor. O loop em `app.rs` desabilita `next_market_wakeup` durante `Pause`; portanto deixa de receber do canal. Após 64 mensagens, o produtor pode ficar preso no envio e deixar de ler, responder a pings ou detectar uma queda da conexão. Em `Resume`, mensagens antigas podem chegar antes do próximo REST poll e disparar avaliações atrasadas. Além disso, a TUI alterna `Dashboard.paused` localmente, mas o loop do app não o atualiza; um `Refresh` pode mostrar estado incorreto.

**Objetivo:** uma pausa interrompe todas as avaliações, mantém a recepção WS responsiva e uma retomada só permite a primeira avaliação depois de um backfill REST válido e atual. O último timestamp avaliado continua monotônico. Um WS ausente ou desconectado não impede a retomada por REST.

**Não objetivos:** executar/reconciliar ordens, recuperar cada sinal do período pausado, persistir todo o histórico de uma pausa longa, alterar a estratégia SMA, adicionar endpoint/configuração pública, habilitar produção ou transformar o WS em fonte de verdade para a reconciliação.

## Contratos e seams para acordo do usuário antes dos testes

1. **Comando e observação existentes:** `UiCommand::Pause`, `UiCommand::Resume` e `UiCommand::Quit` continuam sendo a entrada da TUI; `AppEvent::Refresh(Dashboard)` continua informando snapshots de mercado. O estado de controle confirmado pelo loop chega à TUI por um canal interno `watch`, independente da fila de Refresh. A TUI mostra `PAUSED`, `RESUMING` ou `RUNNING · OBSERVE ONLY` conforme essa confirmação, sem alternar otimisticamente o estado local. Comandos repetidos de pausa/retomada são idempotentes. Nenhuma avaliação pode ser publicada depois que `Pause` é confirmado, embora uma avaliação concluída antes dessa confirmação possa ser vista depois pela TUI.
2. **Dados e estratégia existentes:** `MarketDataSource::candles`/`BinanceMarketData::candles`, `HybridCandleFeed` e `run_evaluation_cycle` permanecem os limites comportamentais. `Pause` impede `run_evaluation_cycle`; `Resume` exige um REST backfill autorizado, validado, contíguo e atual antes de invocá-lo. Avalia-se somente o candle fechado mais recente, nunca todos os sinais omitidos na pausa. Depois disso, WS/REST continuam com a deduplicação existente.
3. **Canal WS:** o produtor de `ClosedKline` nunca espera por capacidade do canal. Quando os 64 lugares estão ocupados, descarta o novo evento com log/contador de overflow; REST continua como recuperação. Em qualquer estado, um candle WS cuja hora de fechamento esteja no futuro pelo relógio local é ignorado antes de entrar no feed ou watermark. Canal fechado encerra apenas a recepção WS, não a TUI nem o REST.

Esses são os seams propostos para obter acordo explícito do usuário antes de escrever testes, conforme `AGENTS.md`. Não há promessa de compatibilidade de uma API pública externa nova; o binário/TUI é a superfície de produto.

## Design

### Estado e transições

Introduzir estado explícito do monitor: `Running`, `Paused`, `Resuming`. O loop principal é a autoridade; ao consumir comando ou completar a reconciliação, publica o estado confirmado por `watch` e atualiza o `Dashboard` antes do próximo Refresh. A TUI lê o estado confirmado do `watch` com prioridade sobre snapshots antigos. Ela envia o comando conforme esse estado; pressionar espaço durante `Resuming` pede `Pause`. `Pause` vindo de `Running` ou `Resuming` entra em `Paused`; solicita cancelamento das operações em voo e nenhum resultado tardio pode ser aplicado. `Resume` vindo de `Paused` entra em `Resuming` e inicia a primeira tentativa REST imediatamente. Um segundo `Resume` não cria tentativa duplicada. `Quit`/Ctrl-C cancelam WS e operações em voo.

REST, avaliação/JEV e publicação de resultado não devem bloquear a recepção de comandos no loop principal. A busca REST e o cálculo/parecer JEV executam como operações assíncronas canceláveis, com uma geração monotônica de controle; tarefas retornam resultados candidatos, e apenas o loop principal verifica a geração e confirma alterações no feed, `last_evaluated_ts`, dashboard e eventos. Em `Pause`, incrementar a geração e cancelar/abortar tanto REST normal como reconciliação e avaliação/JEV em voo. Uma tarefa de geração antiga nunca marca o candle como avaliado nem publica sinal. O loop limita a uma busca REST e uma avaliação candidata em voo, coalescendo candles WS novos no feed até poder avaliar o mais recente. Antes de aplicar uma candidata, compara seu timestamp ao último candle do feed e ao marcador já avaliado: se chegou candle mais novo, descarta integralmente sinal, parecer JEV e atualização de posição/`last_evaluated_ts` da candidata antiga e agenda avaliação do último candle quando a janela estiver pronta. Essa regra vale em `Running` e para a candidata iniciada pela primeira janela REST após reconciliação. Não existe prazo real rígido de interface sob escalonamento ou travamento da TUI, mas a confirmação de `Pause` não espera rede, timeout REST nem JEV; teste com essas chamadas bloqueadas verifica confirmação dentro de um prazo local curto e nenhuma publicação posterior. Antes da confirmação, o rótulo permanece no estado anterior, sem prometer uma pausa que ainda não entrou em vigor.

O resultado de um REST poll normal em `Running` também é apenas candidato. Antes de chamar `ingest_rest_window`, comparar seu último timestamp ao último candle já presente no feed no momento do commit. Se a janela REST termina antes desse candle (por exemplo, WS B chegou enquanto REST buscava até A), descartar toda a janela, inclusive sua persistência, sem mutar feed/marcador nem publicar avaliação, e aguardar próximo poll; não substituir o feed por uma série regressiva. Se termina no mesmo timestamp ou depois, pode substituir a janela após as validações existentes; a avaliação subsequente ainda obedece ao gate de candidata acima. Isso impede que uma resposta REST atrasada desfaça um WS mais recente.

Em `Paused` e `Resuming`, o loop continua consumindo o canal WS. Validações de estrutura/OHLC já ocorreram no produtor; o app descarta os candles recebidos sem tocar no feed, estratégia, JEV ou persistência e guarda apenas o maior timestamp fechado plausível observado desde a pausa (`pause_ws_watermark`). Antes de usar um candle WS em qualquer estado, exigir com relógio UTC injetável que `timestamp_ms + period_ms <= now_ms`, com aritmética verificada. Um timestamp no futuro, inclusive alinhado e com OHLCV válido, é ignorado tanto para feed como para watermark e registrado como anomalia; nunca pode avançar o marcador de avaliação nem manter `Resuming` preso à espera de REST futuro. O tick de poll é consumido em `Paused` sem chamada REST. Em `Resuming`, o tick agenda uma nova tentativa apenas quando não há outra em voo. Em `Running`, o fluxo WS/REST atual permanece salvo a execução assíncrona cancelável descrita acima. O loop não usa uma seleção interna com prioridade permanente de WS sobre REST ou comandos: o tick REST e a conclusão de reconciliação precisam receber oportunidade mesmo sob fluxo WS contínuo.

A tentativa REST é uma operação assíncrona limitada a 10 segundos, executada de forma que o loop continue consumindo comandos e WS. Ela usa a autorização existente `RestUse::HistoricalBackfill`, a conta Spot `dev`, símbolo/timeframe/limite configurados e a validação integral de janela do adaptador. Antes de aceitar o resultado, o loop esvazia o backlog já presente no canal WS sem avaliação, atualizando o watermark. Resultado de tentativa cancelada ou de geração de pausa anterior é ignorado. A implementação deve preservar um único request em voo e descartar/cancelar seu handle na saída.

Uma janela de reconciliação só é aceita se:

- não for vazia e passar validação de candles REST existente;
- seu último candle for pelo menos tão recente quanto `pause_ws_watermark`, quando houver;
- seu último candle não for anterior ao último timestamp já avaliado pelo feed;
- o último timestamp não for futuro e representar um candle fechado recente: com relógio UTC `now_ms` e período configurado `period_ms`, exigir `0 <= now_ms - (latest_ts + period_ms) <= period_ms`, com aritmética verificada. Isso tolera no máximo um período de atraso depois do fechamento. A checagem de tempo fica em função interna pura com relógio injetável para teste determinístico;
- o feed resultante passar `ready_for_evaluation(sma_slow, timeframe)` antes de produzir uma avaliação não `Warmup`.

Validar a continuidade numa cópia candidata do feed; janelas recusadas não alteram o feed nem a persistência. Ao aceitar REST, o app substitui o feed pela cópia candidata, preservando o `last_evaluated_ts`, e só chama `run_evaluation_cycle` se o timestamp retornado for novo. Se a janela estiver atual e contígua, mas já for o último candle avaliado antes da pausa, a retomada conclui sem emitir avaliação duplicada. A ausência de sinal novo não impede `Running`. Uma janela atual ainda sem histórico contíguo mantém `Resuming` e é reconsultada no próximo tick; não transforma o marcador de avaliação em avanço falso. Ao voltar a `Running`, o watermark é limpo e WS volta ao processamento normal. Eventos WS que chegam depois dessa transição seguem o marcador monotônico do feed.

Falha de rede, timeout, autorização negada, janela vazia/inválida, dado atrasado, histórico insuficiente ou divergência com o watermark mantêm `Resuming`: registrar motivo sem segredo, publicar erro/estado visível e tentar no próximo poll configurado. Não usar WS como atalho para sair de `Resuming`. Se WS nunca iniciou ou terminou, watermark ausente; a recência e continuidade REST bastam. Um REST que volta a funcionar recupera o monitor mesmo sem WS. A reconciliação não transforma falha de mercado em falha fatal da TUI.

### Pressão no canal e heartbeat

Trocar o `send().await` de candle fechado por envio não bloqueante (`try_send`). `Full` descarta o evento e incrementa contador/log estruturado com símbolo, timestamp e capacidade, sem payload sensível; `Closed` encerra normalmente a sessão. O loop segue drenando WS durante pausa/retomada, e o produtor continua processando ping/pong, reconnect e cancelamento mesmo se a TUI estiver pausada ou lenta. O canal de 64 continua limitado. Uma perda por overflow não se converte em ordem nem em avaliação extra; o próximo poll REST substitui a janela e o marcador impede replay. Para 1m, `poll_seconds` já é limitado pela configuração ao intervalo do candle.

### Dados, segurança e observabilidade

Durante `Paused`/`Resuming`, WS descartado não é persistido e REST não é consultado na pausa; a primeira janela aceita após `Resume` pode voltar a ser persistida pela regra atual de 1m. Uma pausa longa pode exceder `candle_limit`; não há garantia de arquivo histórico sem lacunas. Logs estruturados registram transições, duração da pausa, tentativas/erros REST, maior timestamp WS observado, atraso da janela aceita e overflow do canal. Não registrar mensagens WS completas, credenciais nem respostas JEV. JEV não é aplicável à decisão de retomada: trata-se de regra determinística de integridade/recência, e o aconselhamento JEV continua apenas após avaliação normal.

## Alternativas consideradas

| Alternativa | Decisão | Motivo |
|---|---|---|
| Congelar o receiver durante pausa | Rejeitada | Bloqueia o produtor ao encher 64 posições; pode prejudicar heartbeat e reproduz candles velhos na retomada. |
| Manter todos os WS pausados no feed e avaliar a fila depois | Rejeitada | Replay de sinais atrasados e estado dependente da duração da pausa; REST é a fonte de recuperação. |
| Apenas esvaziar a fila no `Resume`, sem REST | Rejeitada | Não comprova ausência de lacunas ou atualidade, especialmente se WS caiu ou houve overflow. |
| Encerrar e recriar WS a cada pausa | Rejeitada | Adiciona reconexões e não resolve REST atrasado; desconexão WS não deve bloquear monitor. |
| Substituir canal por `watch` com último candle | Adiada | Pode reduzir backlog, mas muda a semântica do fluxo e amplia o CL; o envio não bloqueante mais REST já resolve o risco delimitado. |

## Entregas, TDD e validação

**C14 — Estado Pause/Resume e reconciliação (Builder Backend; Critic Backend independente).** Depois de G1 e acordo dos seams: teste red do fluxo observável por `UiCommand`/estado confirmado/`AppEvent` e fonte REST falsa com relógio controlado: pausa recebe >64 WS sem avaliação; retomada ignora WS até REST válido; avalia apenas o último candle uma vez; REST inválido/atrasado/falho mantém `RESUMING` e recupera em novo poll; WS ausente/fechado ainda retoma; Pause durante reconciliação invalida resultado. Um WS alinhado com OHLCV válido mas timestamp futuro não eleva watermark, não entra no feed em `Running` e não prende a retomada. Um REST normal e um JEV em `Running` bloqueados por fixtures não atrasam a confirmação de Pause nem publicam avaliação depois dela. Com JEV bloqueado para candle A, entregar candle B antes de liberar A: nenhum sinal/estado/posição de A pode ser publicado, e B é avaliado uma vez; repetir após a primeira janela REST de retomada. Com REST normal bloqueado após capturar janela até A, entregar WS B e então liberar REST: a janela antiga não substitui o feed, não é persistida, não causa replay e B permanece o último candle. O estado confirmado prevalece sobre Refresh antigo. Implementação mínima green; testes existentes de feed/monitor continuam passando. Manter a costura de injeção interna ao monitor, sem publicar API externa adicional.

**C15 — Envio WS sem bloqueio e documentação (Builder Backend; Critic Backend independente).** Teste red do produtor com canal capacidade 1 saturado: enviar segundo candle fechado retorna sem aguardar receiver e mantém capacidade limitada; canal fechado encerra envio; mensagens inválidas seguem ignoradas. Implementação mínima green; atualizar `backend/README.md` e logs do monitor para semântica de pausa/retomada e perda recuperável. O Critic verifica que `try_send` não converte `Full` em encerramento do WS. Cada CL executa `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` e `cargo test`; teste integrado com servidor/clock falso verifica o caminho end-to-end sem Binance real.

O teste do documento é comportamental, não código: o Crítico confronta cada regra acima com `app.rs`, `exchanges/live.rs`, `market_feed.rs` e `ui/mod.rs`, e a implementação só começa após aprovação G1 e dos seams pelo usuário. Nenhum resultado de teste de implementação é alegado aqui.

## Riscos e rollout/rollback

- Um relógio local incorreto pode manter o monitor em `RESUMING`; o erro deve mostrar idade da janela sem expor segredos. O operador corrige o relógio ou a fonte; não se afrouxa silenciosamente o limite.
- Um WS com tempo muito à frente é ignorado para watermark mesmo se o parser aceitar seus OHLCV; esse evento não é avaliado durante a pausa. A reconciliação REST ainda passa pelas regras independentes de recência/continuidade.
- O REST pode não publicar a barra recém-fechada imediatamente; o limite de um período evita avaliação muito velha, e o próximo poll recupera. Métricas/logs de tentativa permitem diagnosticar atraso.
- Em fluxo WS mais rápido que o consumo, `try_send` pode descartar o candle mais recente. O poll REST e o watermark/recência impedem a retomada com janela conhecida como antiga; em `Running`, a avaliação desse candle pode atrasar até o poll.
- Mudança vale somente para o monitor local após build; rollback é reverter C14/C15 juntos. Até novo build/restart, a pausa antiga continua sujeita a backpressure. Não há migração de dados, feature flag ou deploy implícito.

## Questões abertas e gate

- **Acordo do usuário pendente:** os três seams da seção de contratos, inclusive o estado `RESUMING` e a política de descarte por canal cheio, devem ser aceitos antes de escrever testes.
- **G1 técnico aprovado:** `/root/resume_design_critic` confirmou resolução dos três achados em revisão independente; a aprovação não autoriza testes antes do acordo do usuário sobre seams.
- **G2/G3 pendentes:** instâncias Builder/Critic e critérios por CL serão confirmados pelo Orquestrador; nenhuma implementação/teste começou por este SDD.
