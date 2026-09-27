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
- **Escopo:** definir o contrato público entre o módulo `monitor` e a apresentação terminal, preservando os dados atuais da TUI e a operação de pausa, retomada e shutdown.
- **Jev:** não aplicável; contrato de dados e concorrência determinístico, cuja validação é feita por tipos e testes executáveis.

## Contexto e problema

A proposta aprovada com follow-up define comandos/eventos e `MonitorHandle`, mas o snapshot atual contém apenas `state: String`. A TUI atual apresenta configuração do monitor, limites de risco, estado, dados de mercado/estratégia, persistência, posições paper, avisos, erros e até dez logs. Migrar a apresentação sem levar esses dados pelo seam exigiria manter canais internos paralelos ou perder comportamento observável.

## Objetivos

- A TUI recebe uma fotografia completa e sem tipos internos de domínio que permita reconstruir tudo o que já exibe.
- O produtor publica estado atual recuperável e notificações transitórias com semânticas distintas.
- A TUI traduz teclas em comandos do monitor e trata envio, lag, fechamento e shutdown explicitamente.
- Pausa e retomada preservam suas transições atuais.
- Encerramento publica `Stopped` depois do cancelamento cooperativo e a TUI aguarda o resultado com timeout explícito.
- Cada gate de equipe falha diante de qualquer erro, warning, falha de teste, lint/formatação, violação arquitetural, placeholder ou código incompleto.

## Não objetivos

- Alterar layout/rótulos/valores atualmente apresentados pela TUI.
- Alterar estratégia, risco, dados de mercado, persistência ou schema SQL.
- Persistir notices transitórias para auditoria; logs/erros atuais do snapshot permanecem visíveis como estado.
- Introduzir dependência de framework de UI ou de modelo de domínio dentro dos DTOs.

## Contrato público proposto

DTOs no módulo `monitor` ou num módulo de presentation contracts explicitamente neutro:

- `MonitorCommand`: `Pause`, `Resume`, `Refresh`, `Shutdown`.
- `MonitorRunState`: enum equivalente a `Running`, `Paused`, `Resuming`.
- `MonitorEnvironment`, `MonitorRunMode`, `MonitorSignal`, `PersistenceStatus`: enums do contrato, sem importar os enums internos.
- `MonitorSnapshot`: `revision: u64`; environment, run mode, symbol, operation, risk profile; run state; configuração SMA fast/slow; `max_order_quote`; estado de persistência; contagem de posições paper; advisory e último erro; no máximo 10 logs; `market: Option<MonitorMarketSnapshot>`.
- `MonitorMarketSnapshot`: close, SMA fast/slow opcionais e signal. Valores usam representação decimal canônica sem dependência de tipos internos; escala e arredondamento são propriedades do DTO e mantêm os formatos visíveis atuais (mercado/SMA com 4 casas, limite monetário com 2 casas).
- `MonitorNotice`: classe/severidade tipada e mensagem limitada a tamanho documentado; notices são efêmeras e não substituem estado persistente de erro/advisory/log.
- `MonitorEvent`: `StateChanged(MonitorSnapshot)`, `Notice(MonitorNotice)`, `Stopped`.
- `MonitorSendError`: `Full`, `Closed`; `try_send` continua não bloqueante.
- `MonitorHandle::send(command) -> Result<(), MonitorSendError>`.
- `MonitorHandle::subscribe() -> broadcast::Receiver<MonitorEvent>`.
- `MonitorHandle::latest_snapshot() -> watch::Receiver<MonitorSnapshot>` (ou interface equivalente baseada em watch) fornece recuperação autoritativa do snapshot completo após `broadcast::RecvError::Lagged`.

O monitor mantém o snapshot mais recente em `watch`; publica `StateChanged` por `broadcast` para acordar consumidores e envia `Notice`/`Stopped` por broadcast. Depois de lag, a TUI relê o watch e redesenha a fotografia atual; não reconstrói estado aplicando eventos possivelmente perdidos. Alterações visíveis atualizam snapshot e evento de estado; publicação é limitada/coalescida a no máximo uma atualização visual por intervalo de 250 ms, além de mudanças críticas e resposta imediata a Refresh.

### Shutdown e canal

- `Shutdown` é idempotente. O monitor cancela o `CancellationToken` interno, interrompe tarefas e publica exatamente um `Stopped` depois que os componentes supervisionados encerram.
- A TUI trata `Stopped` como encerramento confirmado e aguarda no máximo dois segundos após enviar `Shutdown`.
- `Full` significa comando não aceito; a TUI reporta falha de shutdown e tenta uma única vez após timeout curto definido no código. `Closed` significa que o monitor já não aceita comandos; a TUI consulta o snapshot mais recente e termina indicando encerramento inesperado.
- `broadcast::RecvError::Lagged` aciona recuperação via watch, sem descartar a conexão de eventos. `Closed` antes de `Stopped` é término inesperado e retorna erro ao chamador; `Stopped` seguido do fechamento é término normal.
- Todos os resultados de envio/recepção são tratados; nenhum `let _ =` descarta erros do seam.

## Preservação da apresentação atual

O snapshot precisa reter os dados lidos por `Dashboard::draw`: ambiente, run mode, símbolo, operação, perfil de risco, monitor state, persistence status, close, SMA fast/slow e seus períodos, signal, limite máximo por ordem, paper positions, advisory, último erro e logs recentes. O sinal ausente continua renderizado como warmup; close vazio mantém `—`, médias ausentes mantêm `warming up`; erro atual prevalece sobre advisory; modo Paper mostra a contagem de posições. Testes comparam os campos e rótulos renderizáveis, sem copiar a implementação privada do desenho.

## Alternativas

### Uma única stream broadcast para tudo

É simples e funciona em happy path, mas um consumidor atrasado perde eventos e não consegue reconstruir o estado. Rejeitada para snapshots autoritativos.

### Manter somente os canais atuais da TUI

Minimiza alteração imediata, mas perpetua acoplamento da apresentação aos channels internos e tipos de domínio. Rejeitada para a estrutura-alvo.

### Snapshot `f64` e strings livres

Reduz custo de conversão, mas permite estados inválidos e arredondamento implícito. Rejeitada: enums tipados e decimal canônico preservam valores/semântica sem expor tipos de domínio.

## Migração e gates observáveis

1. Testar o contrato de DTO e canais com produtor e consumidor reais; teste deve falhar antes da implementação por ausência do snapshot completo/recovery.
2. Adaptar o monitor para atualizar watch e publicar eventos, com invariantes de snapshot completo e lag recovery.
3. Adaptar a TUI para comandos/eventos, validar todos os campos e preservar pause/resume/shutdown.
4. Executar teste end-to-end de canal: Pause, estado Paused, Resume, estado Running, Shutdown, cancel token, Stopped e encerramento normal; caso separado cobre canal fechado antes de Stopped.
5. Cada fatia exige testes relevantes, suíte do backend, format, check, verificação de dependências e code review independente.

### Tolerância zero da equipe

Um gate falha se qualquer etapa produzir erro, warning, teste falho/ignorado sem justificativa aceita, lint/format warning, violação de dependência, placeholder, `TODO`, `FIXME`, código incompleto ou comportamento não verificado. Avisos preexistentes entram no baseline e também bloqueiam o gate até resolução ou registro formal de que estão fora do escopo, aprovado pela autoridade competente. Nenhum resultado parcial é descrito como completo. Construtor e Crítico compartilham o mesmo baseline e registram comando, saída integral, exit code, versão/configuração relevante e decisão. Achado crítico/importante bloqueia; revisão independente é obrigatória para cada artefato.

## Questões abertas

- Qual serialização decimal canônica o backend usa para preço, SMA e limites sem introduzir dependência de domínio no seam? Resolver por comparação de tipos atuais e teste round-trip exato; decide arquitetura e banco.
- A cadência de 250 ms preserva fluidez e custo em snapshots completos? Resolver por teste de carga local determinístico e medição de coalescência; decide SRE/arquitetura.
- Quais tarefas do monitor precisam concluir antes de publicar `Stopped`, e qual limite de shutdown é compatível com os timeouts atuais? Resolver pelo mapa de tarefas/cancelamento e testes de timeout; decide backend/SRE.

## Referências locais

- [Proposta 0001 — Separação entre core, módulos e MVC no backend](../proposals/0001-backend-core-modules-mvc.md)
- [Catálogo completo de módulos do backend](../architecture/module-catalog.md)
- [Matriz de testes](../reference/test-matrix.md)
