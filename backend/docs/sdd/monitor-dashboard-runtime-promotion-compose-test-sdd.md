---
title: SDD — Ativar teste de composição monitor→runtime de bots
description: Especifica ativação isolada do teste de composição dashboard, MonitorSnapshot e runtime promovido.
tags:
  - sdd
  - backend
  - monitor
  - bots
  - testing
status: proposed
---

# SDD — Ativar teste de composição monitor→runtime de bots

**Estado:** proposta G1 pendente de revisão independente. Nenhuma alteração de código ou execução de teste está autorizada por este documento.

## Contexto e objetivo

A auditoria reportou `monitor_snapshot_from_dashboard_then_runtime_apply_enriches_promotion` como `dead_code`: a função já está dentro de `#[cfg(test)] mod mapping_tests`, contém o cenário e assertions de composição pretendidos, mas não tem `#[test]` nem chamadores. Isso indica cobertura ausente; não demonstra defeito no caminho de produção.

Este item limita-se a marcar a função existente como teste executável. Não altera assinatura, produção, semântica da API, setup do dashboard, dados do runtime ou assertions atuais.

Referências: [contrato de apresentação do monitor](./monitor-presentation-contract-sdd.md) e [SDD do runtime de bots](./bots-runtime-live-gate2-sdd.md).

## Seam e comportamento observável

Usar o seam público já existente `monitor_snapshot_from_dashboard(&dashboard, revision)` seguido por `apply_bot_runtime_to_monitor_snapshot(&mut snapshot, &runtime)`. O teste permanece na unidade `mapping_tests` do mapper e usa `InMemoryBotRuntime`; não precisa de HTTP, store persistente, banco ou exchange.

O cenário preserva os asserts atuais: snapshot inicial com runtime desabilitado; após promoção válida pelo runtime em memória, snapshot enriquecido com `bot_runtime_enabled == true` e `promoted_bot_id` igual ao ID promovido. Quando uma promoção válida foi aplicada, ausência, valor antigo ou divergência desses campos reprova o teste. Este critério verifica composição e não autoriza execução de ordens.

## TDD e critério de aceite

- **Contrato necessário do wrapper:** o caminho isolado `verify-test-isolation.sh --filter <filtro>` deve interpretar o summary do Cargo e falhar fechado se a contagem de testes selecionados/executados não for exatamente um. Status de processo zero não basta. Este requisito depende do gate T-W0-06a e deve ser revisado/implementado antes de usar o wrapper como evidência para este item.
- **Fixture do wrapper:** cobrir summaries fakes de 0, 1 e 2 testes: zero e múltiplos terminam não-zero sem reportar PASS; exatamente um permite validar o código de saída e o resultado daquele teste. O filtro continua usando `--exact`; a fixture não inicia Cargo, Docker ou serviços.
- **RED:** antes da anotação `#[test]`, o filtro exato não encontra caso correspondente e o summary informa 0; a nova fixture deve provar que o wrapper falha fechado nesse caso, ainda que Cargo retorne status zero.
- **GREEN:** adicionar somente `#[test]` à função existente; o mesmo filtro deve produzir summary com exatamente 1 teste executado e passar, preservando os asserts existentes.
- **Verificação posterior:** após a fixture e a implementação do contrato no wrapper, e quando G3 permitir execução do caso, rodar `cargo check --locked --offline --all-targets` e Clippy pelo caminho isolado; confirmar que o aviso de dead code desapareceu sem novos warnings desta função.
- Os resultados RED/GREEN e da verificação posterior devem registrar comando/filtro, contagem observada e resultado, sem conteúdo de `.env` ou segredos.

## Fases e dependências

A implementação aguarda aprovação independente G1 deste SDD e aprovação G3 do runner isolado de testes. Não usar Cargo direto do host para produzir evidência de aceite. A fatia não depende de PG, Neo4j, rede, credenciais Binance, Testnet, containers ou da promoção via HTTP.

## Alternativas e riscos

- **Manter a função sem atributo:** não adiciona cobertura e mantém o diagnóstico de código morto; rejeitado.
- **Criar um teste duplicado/refatorar o runtime:** amplia escopo sem necessidade; rejeitado nesta fatia.
- **Risco de filtro falso-verde:** Cargo pode terminar com status zero ao encontrar zero testes. A contagem exata (RED=0, GREEN=1) é obrigatória, em vez de usar somente o código de saída.

## Rollback

Se os asserts atuais falharem após ativar o teste, remover somente a anotação `#[test]` e registrar a causa fora do escopo. Não alterar ou remover os asserts para forçar GREEN. Se o runner isolado não estiver aprovado/disponível, não executar o teste e manter a entrega pendente.
