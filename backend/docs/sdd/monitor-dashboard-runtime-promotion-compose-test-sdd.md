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

- **RED:** executar o filtro exato do teste pelo runner de testes isolado aprovado. Antes da anotação `#[test]`, o filtro não encontra teste correspondente: zero testes executados. O Cargo pode retornar sucesso quando nenhum teste combina; por isso, a evidência RED é a contagem zero, e o gate deste item trata qualquer contagem diferente de um como falha.
- **GREEN:** adicionar somente `#[test]` à função existente e repetir o mesmo filtro, com o mesmo runner e snapshot de código. Deve executar exatamente um teste e passar, preservando os asserts existentes.
- **Verificação posterior:** quando G3 permitir a execução, rodar `cargo check --locked --offline --all-targets` e Clippy pelo mesmo caminho isolado, confirmando que o aviso de dead code desapareceu sem novos warnings desta função.
- Os resultados RED/GREEN e da verificação posterior devem registrar comando/filtro, contagem observada e resultado, sem conteúdo de `.env` ou segredos.

## Fases e dependências

A implementação aguarda aprovação independente G1 deste SDD e aprovação G3 do runner isolado de testes. Não usar Cargo direto do host para produzir evidência de aceite. A fatia não depende de PG, Neo4j, rede, credenciais Binance, Testnet, containers ou da promoção via HTTP.

## Alternativas e riscos

- **Manter a função sem atributo:** não adiciona cobertura e mantém o diagnóstico de código morto; rejeitado.
- **Criar um teste duplicado/refatorar o runtime:** amplia escopo sem necessidade; rejeitado nesta fatia.
- **Risco de filtro falso-verde:** Cargo pode terminar com status zero ao encontrar zero testes. A contagem exata (RED=0, GREEN=1) é obrigatória, em vez de usar somente o código de saída.

## Rollback

Se os asserts atuais falharem após ativar o teste, remover somente a anotação `#[test]` e registrar a causa fora do escopo. Não alterar ou remover os asserts para forçar GREEN. Se o runner isolado não estiver aprovado/disponível, não executar o teste e manter a entrega pendente.
