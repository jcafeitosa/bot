---
title: SDD — Contrato de apresentação do monitor e tolerância zero
description: Contrato completo entre a TUI e o monitor para snapshot, comandos, eventos e encerramento
tags:
  - sdd
  - backend
  - monitor
  - presentation
  - architecture
status: draft
---
# SDD — Contrato de apresentação do monitor e tolerância zero

- **Estado:** draft, aguardando revisão independente.
- **Escopo:** contrato entre módulo monitor e TUI para snapshot, comandos, eventos e encerramento, preservando a apresentação e pausa/retomada existentes.
- **Jev:** não aplicável; contrato determinístico validado por testes executáveis.

## Contexto, objetivos e limites

A apresentação atual reúne configuração, risco, estado, mercado/estratégia, persistência, paper positions, avisos, erros e até dez logs. O seam precisa transportar todos esses dados em DTOs independentes dos tipos internos de domínio, tornar estado atual recuperável após perda de eventos e separar estado durável na sessão de notices transitórias.

Não se altera layout, estratégia, risco, mercado, política de persistência nem schema SQL. Não se adiciona framework de UI nem dependência de domínio aos DTOs. Não se persiste notice transitória; erros, advisory e logs continuam como estado visível do snapshot.

## Decisão de ownership e tipos públicos

Local único: `crate::modules::monitor::presentation_contract`, exportado pelo monitor. Este módulo contém somente DTOs, enums, traits/handles de apresentação e validação de comprimento; não importa os tipos internos da estratégia, risco, persistência ou UI. O produtor e as conversões ficam no módulo `monitor`; `ui` é consumidor.

`MonitorCommand`: `Pause`, `Resume`, `Refresh`, `Shutdown`.

Enums sem strings livres para estados: `Environment = Dev | Prod`; `RunMode = Observe | Paper`; `MonitorRunState = Running | Paused | Resuming`; `OperationLabel` e `RiskProfileLabel` são labels textuais validados conforme limite de 128 bytes; `MonitorSignal = Warmup | Buy | Sell | Hold` (Buy/Sell/Hold correspondem aos sinais reais; ausência de mercado/sinal é representada como `None`); `PersistenceStatus` enum fechado com `Healthy | Degraded | Unavailable`.

`MonitorSnapshot` tem exatamente os seguintes campos: `revision: u64`; `environment: Environment`; `run_mode: RunMode`; `symbol: String`; `operation_label: String`; `risk_profile_label: String`; `run_state: MonitorRunState`; `sma_fast_period: u32`; `sma_slow_period: u32`; `max_order_quote: Decimal`; `persistence_status: PersistenceStatus`; `market: Option<MonitorMarketSnapshot>`; `paper_positions: Option<u64>`; `advisory: Option<String>`; `last_error: Option<String>`; `logs: Vec<MonitorLogEntry>`. `MonitorMarketSnapshot` tem exatamente `close: Decimal`, `sma_fast: Option<Decimal>`, `sma_slow: Option<Decimal>`, `signal: Option<MonitorSignal>`. `MonitorLogEntry` tem `message: String`. `MonitorNotice` tem `severity: NoticeSeverity` (`Info | Warning | Error`) e `message: String`. `MonitorEvent` é `StateChanged { revision: u64 } | Notice(MonitorNotice) | Stopped`. O payload do estado não é duplicado no broadcast: a fotografia completa e autoritativa está no watch.

### Decimal canônico

`Decimal` é `{ coefficient: i128, scale: u32 }`, interpretado exatamente como `coefficient × 10^-scale`. `scale` permitido é 0..=18; coeficiente cobre todo intervalo i128 assinado. Não se normaliza escala nem se converte via ponto flutuante; serialização preserva coefficient e scale, inclusive zeros finais. Valores não representáveis nesse intervalo/escala são rejeitados e geram estado de erro, nunca saturados. Formatação de mercado e SMA mostra exatamente quatro casas; `max_order_quote` exatamente duas. Formatação que demande redução de escala usa arredondamento decimal half-even explicitamente somente para texto, sem alterar o DTO; valores com precisão acima da escala de apresentação são exibidos com half-even e o teste verifica o caso de empate positivo e negativo. A conversão interna->DTO deve preservar o valor original exato. Testes verificam encode/decode round-trip do par coefficient/scale e valores extremos válidos.

### Strings e truncamento

Limites são bytes UTF-8: labels e símbolo 128 cada; notice 1.024; error e advisory 4.096 cada; cada log 4.096. No máximo 10 entradas de log, as mais recentes em ordem cronológica crescente. Ao exceder o limite, truncar somente no último limite de caractere UTF-8 que caiba e acrescentar `…` dentro do limite; não cortar code point, não normalizar Unicode e não modificar enum, número, identidade ou semântica dos campos. Comprimento inválido de símbolo/label rejeita a atualização do snapshot e publica erro de validação; textos informativos truncam conforme regra. Teste usa texto multibyte exatamente no limite e um caractere acima.

`paper_positions: None` significa não aplicável/indisponível; `Some(0)` é contagem zero. `market: None` significa sem fotografia de mercado; em mercado presente, close é obrigatório e médias/sinal são opcionalmente indisponíveis. Sinal ausente renderiza warmup. Close ausente só ocorre com `market: None` e renderiza `—`; média ausente renderiza `warming up`. Error atual precede advisory. Paper exibe contagem quando disponível.

## Publicação, revisão e notices

O servidor mantém `watch<MonitorSnapshot>` como fonte de verdade. `revision` começa em 0, aumenta estritamente uma unidade para cada mutação visível comprometida e nunca diminui nem reinicia durante a vida do monitor. Snapshot é publicado atomicamente antes do wake-up `StateChanged { revision }` no broadcast. Esse evento é somente wake-up: consumidor sempre relê watch; ignora revisão menor ou igual à já desenhada. O TUI relê snapshot após wake-up, lag ou reconexão, compara revision e nunca regride. `Lagged(n)` não encerra a subscrição; dispara re-read completo do watch e segue recebendo eventos. Broadcast fechado antes de estado terminal é falha do produtor.

Avisos transitórios são somente `Notice`, não alteram snapshot nem revision e não substituem advisory/error/log. A TUI exibe notice como mensagem temporária até expirar em 5 segundos ou chegar notice posterior. StateChanged não pode ser usado como notice.

Coalescência: alterações ordinárias visíveis são consolidadas em no máximo uma publicação por janela de 250 ms; ao fim da janela publica-se o último snapshot integral. São críticas e imediatas, uma por revision: transições Running/Paused/Resuming, terminal, mudança de persistence para Degraded/Unavailable e aparecimento/alteração de last_error. `Refresh` força publicação imediata do snapshot atual se houve alteração desde a última revision publicada; se não mudou, responde com a revisão atual sem incrementar. Testes determinísticos com relógio controlado verificam coalescência, immediate-critical e que toda publicação contém a revisão corrente.

## Comandos e resultado de envio

`MonitorHandle::send(command) -> Result<(), MonitorSendError>` é não bloqueante (`try_send`); `MonitorSendError = Full | Closed`. `subscribe() -> broadcast::Receiver<MonitorEvent>` e `latest_snapshot() -> watch::Receiver<MonitorSnapshot>`. `Full` significa comando não aceito; UI preserva estado e apresenta aviso. `Closed` significa produtor não aceita comandos; UI lê o último snapshot e informa falha, exceto se já observou terminal. Comandos Pause/Resume repetidos seguem idempotência definida no SDD de pausa/retomada. Nenhum resultado de envio/recepção é descartado.

## Shutdown terminal monotônico

Estado terminal é latch do servidor, não inferido de broadcast. A primeira solicitação Shutdown atomically marca `ShuttingDown` e rejeita toda nova tarefa e todo comando não-Shutdown; comandos já aceitos não iniciam trabalho novo. Nenhuma transição de snapshot não-terminal é permitida depois do latch. O snapshot recebe revisão final e estado terminal `Stopped` (adicionar variante `MonitorRunState::Stopped`), que é monotônico e permanece no watch. Shutdown duplicado é aceito como idempotente. A fila fechada impede novos comandos.

Sequência obrigatória: (1) latch terminal e rejeição de trabalho; (2) cancelar tokens REST/WS/Jev e tarefas monitoradas, sinalizando cancelamento à escrita de persistência; (3) aguardar tarefas em `JoinSet` até deadline absoluto de 2 segundos desde o latch; (4) abortar tarefas restantes e aguardar término/confirmar cancelamento, sem afirmar commit durable de escrita interrompida; (5) consolidar resultado de persistência em log/error do snapshot final quando possível, atualizar watch para `Stopped` com revision incrementada; (6) tentar emitir `StateChanged { revision }` final; (7) enviar exatamente um `Stopped` broadcast; (8) fechar canais de comando e eventos. Receber `Stopped` não é pré-requisito para encerrar: TUI observa `Stopped` ou fechamento, e em ambos relê watch para decisão terminal. Watch terminal continua consultável após broadcast fechado. Deadline total do TUI também é 2 segundos desde envio, com espera em timeout único e envio sem retry: `Full` indica falha explícita pois não aceitou Shutdown; `Closed` verifica watch terminal, terminando normalmente somente se `Stopped`, caso contrário erro. Se deadline expira antes do evento, TUI relê watch: `Stopped` é encerramento confirmado; não terminal é timeout e falha reportada. Eventos nunca podem tornar a TUI regredir a estado anterior.

Tasks REST/WS/Jev e demais tarefas iniciadas pelo servidor são server-owned, registradas e joined/aborted pelo supervisor; UI não guarda nem cancela JoinHandle de tarefas do servidor. Escrita de persistência já em voo recebe cancelamento e participa do mesmo deadline. `Stopped` garante tasks encerradas/joined ou abortadas; não garante que escrita concorrente foi commitada. O registro terminal explicita `Persisted`, `NotStarted`, `CancelledBeforeCommit` ou `CommitOutcomeUnknown`; resultado desconhecido é erro visível, sem retry automático que duplique escrita.

Testes com barreiras determinísticas cobrem: Full no envio Shutdown; Closed com watch terminal e sem broadcast recebido; Closed não terminal; produtor não lê Stopped mas deadline observa snapshot final; Shutdown repetido; rejeição de comando/tarefa após latch; nenhum snapshot revision menor; tarefas canceladas/joined; escrita confirmada antes do deadline; escrita bloqueada abortada aos 2s e outcome unknown. Timeout do consumer não depende de dormir arbitrário.

## Migração concreta da TUI

Manter `Dashboard::draw` como renderer. Criar adaptador puro de `MonitorSnapshot` para `Dashboard`, traduzindo labels, enums e decimals; nenhuma lógica de negócio fica na TUI. Retirar o canal `watch` do dashboard de origem privada após substituir seu produtor por `latest_snapshot`; retirar o watch interno de estado quando `run_state` estiver no snapshot; substituir `mpsc` de `AppEvent` para dados/estado/notices por seleção sobre `broadcast` de wake-ups/notices mais watch de snapshot. Eventos de teclado convertem-se em `MonitorCommand` via send não bloqueante; Refresh solicita snapshot atual; erros Full/Closed viram aviso local de UI e não mutação fictícia do monitor. O loop da UI redesenha somente após revision maior, notice, input ou tick visual, nunca aplicando StateChanged como delta.

Ordem/dependências: (1) congelar DTO e testes de serialização/render map; (2) implementar publisher watch + wake broadcast no loop principal monitor sem remover canais antigos; (3) executar dual-publish e comparar Dashboard derivado em teste; (4) adaptar UI para read watch e comandos do handle; (5) remover watch dashboard antigo, watch state antigo e mpsc AppEvent após todos os consumidores mudarem; (6) exercitar saturação/lag/fechamento e shutdown; (7) remover compatibilidade somente com testes completos verdes. Migração não muda `Pause`/`Resume` nem o comportamento de reconciliação do SDD `monitor-pause-resume-sdd.md`.

## Alternativas

Broadcast de snapshots como única fonte foi rejeitado porque lag perde estado. Manter channels da TUI como API final foi rejeitado por acoplar monitor a presentation privada. `f64` e strings enum livres foram rejeitados por perda de exatidão e estados inválidos. DTO dentro de `ui` foi rejeitado por inverter ownership; `modules::monitor::presentation_contract` é a fronteira pública única.

## Plano TDD e critérios de aceite

Testes de contrato precedem implementação: schema completo/validação; round-trip decimal e formatação; limites UTF-8; revisão monotônica e wake-up/re-read em lag; transições de terminal; outcome por cada combinação send Full/Closed e receive Stopped/lag/closed/timeout; cadência e eventos críticos; equivalência entre Dashboard legado e adaptador. Após cada fatia: teste comportamental red observado antes do código, green mínimo, revisão/refatoração preservando teste. Não se altera layout nem integração de domínio. Implementação requer acordo explícito prévio do usuário sobre esses seams conforme instruções do repositório.

## Gates de CI e aviso inventory

Gate obrigatório, no diretório `backend`, executa exatamente os dois comandos: `cargo test --locked` e o comando exato de testes PostgreSQL declarado em `.github/workflows/backend-ci.yml`. Aceite somente se ambos terminarem exit 0, sem warnings, erros, testes falhos, falhas de infraestrutura ou placeholders. Um teste ignorado por `cargo test --locked` só é aceitável se esta especificação registrar a justificativa concreta e o comando exato da CI que o executa; em qualquer outro caso ignored bloqueia. É proibido silenciar globalmente teste PostgreSQL. Atualmente SDD T-10 registra uma integração PostgreSQL ignorada em teste do binário e informa que execução completa sofreu `EPERM` de bind loopback em dois testes HTTP; esses relatos são contexto histórico, não aprovação nem exceção para este gate.

O comando postgres deve executar com serviço/configuração exigidos pelo workflow, sem falha de conexão. Cada warning tem inventory obrigatório com texto completo, origem/comando, responsável nominal (dono: mantenedor backend da área que o gerou), ação corretiva e gate de remoção; nenhum warning baseline é dispensado ou aceito no resultado final. Saída integral, exit code e configuração são registrados para ambos os comandos; nenhum comando alternativo substitui os definidos no workflow.

## Riscos e rollout/rollback

Broadcast é wake-up e watch é autoridade; violar ordem update-watch-before-wake causa teste de revisão falho. Abort de persistência pode deixar commit outcome desconhecido, exibido no estado/log final, sem alegar durabilidade. Conversão decimal fora do range rejeita snapshot em vez de alterar silenciosamente valor. A migração usa dual-publish temporário e rollback removendo adaptador novo, mantendo canais legados até testes passarem; sem deploy/publicação autorizados por este SDD.

## Referências locais

- [Proposta 0001 — Separação entre core, módulos e MVC no backend](../proposals/0001-backend-core-modules-mvc.md)
- [Catálogo completo de módulos do backend](../architecture/module-catalog.md)
- [Matriz de testes](../reference/test-matrix.md)
- [SDD de pausa e retomada](monitor-pause-resume-sdd.md)
- [SDD de política de persistência do monitor](monitor-persistence-policy-sdd.md)
