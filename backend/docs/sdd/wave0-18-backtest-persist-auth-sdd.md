---
title: SDD T-W0-08 — Autenticação condicional para persistência de backtest
description: Contrato de auth para persistência do SMA backtest e 404 antes de auth em rotas desconhecidas.
tags:
  - sdd
  - backend
  - security
  - http
  - backtest
  - wave0
status: proposed
---
# SDD T-W0-08 — Autenticação condicional para persistência de backtest

## Estado

**PROPOSED — aguardando G1 independente.** Este documento especifica o fechamento dos achados G3 HTTP/backtest. Nenhuma implementação ou teste foi executado por esta entrega documental. O Critic independente revisará quando o slot for liberado após W0-02.

## Contexto

O contrato de auth HTTP aprovado mantém seis POSTs de cálculo públicos e protege mutações/leituras sensíveis. O contrato também define token admin com pelo menos 32 bytes aleatórios, sem espaços, respostas 503 quando ausente/fraco e 401 para bearer ausente/incorreto quando há token válido. A rota desconhecida deve retornar 404 sem chamar handler. Ver [HTTP admin bearer seam](./http-admin-auth-seam-sdd.md).

A rota POST `/api/v1/backtest/sma-crossover` pode executar persistência quando `persist=true`. Isso cria uma bifurcação: a chamada de cálculo sem persistência deve permanecer pública; a variante que grava dados é uma mutação administrativa. O handler precisa autorizar essa variante antes de resolver qualquer store, URL ou conexão PostgreSQL. Os cenários de persistência em runtime e de teste efêmero também devem seguir [isolamento de efeitos colaterais em testes](./wave0-15-test-side-effect-isolation-sdd.md). Os cálculos e critérios funcionais do backtest estão descritos em [backtest trades e slippage](./backtest-trades-and-slippage-sdd.md).

## Contratos públicos

### POST /api/v1/backtest/sma-crossover

A rota é classificada conforme o comando recebido:

| Requisição | Resposta sem Bearer | Ordem obrigatória | Efeito no PostgreSQL |
|---|---|---|---|
| `persist=false` | resposta normal do cálculo público | calcular sem resolver store ou URL | zero conexão, migração ou escrita |
| `persist=true`, token admin ausente/vazio/fraco | 503 `admin_auth_not_configured` | autenticar antes de resolver store/URL | zero conexão, migração ou escrita |
| `persist=true`, token utilizável configurado e bearer ausente/incorreto | 401 `unauthorized` | rejeitar antes de resolver store/URL | zero conexão, migração ou escrita |
| `persist=true`, bearer válido | resposta normal após persistência | autenticar; então resolver o store runtime selecionado | persistência permitida no store do serviço |

A autenticação é verificada usando `BOT_HTTP_ADMIN_TOKEN`, conforme o seam existente. O parse limitado do corpo necessário para identificar `persist` não pode resolver configuração, abrir store, consultar URL, conectar, migrar ou persistir. Bearer válido só autoriza a operação; o teste de escrita deve apontar exclusivamente ao PostgreSQL efêmero validado pelo runner previsto em [T-W0-06](./wave0-15-test-side-effect-isolation-sdd.md). Em `serve`, após auth válida, a escrita usa o store runtime configurado para o serviço.

### Rotas desconhecidas

Resolução de rota registrada ocorre antes de qualquer política de auth por prefixo. Método/path sem rota registrada, inclusive POST desconhecido dentro de `/api/v1/admin/*`, retorna 404 `route_not_found` com ou sem token e sem executar handler. Não criar catch-all de prefixo que autentique ou transforme rota inexistente em 401/503.

As outras cinco rotas de cálculo da allowlist aprovada permanecem públicas e sem store. Nenhuma alteração amplia ou reduz a lista aprovada. A rota SMA mantém uma única URL pública, com a bifurcação condicionada ao campo `persist`.

## Seams internos propostos

1. A rota é registrada com resolução explícita de existência; fallback desconhecido retorna 404 antes da auth.
2. O handler decodifica o comando de backtest sem efeitos de I/O e seleciona o ramo.
3. `persist=false` segue o cálculo público e não chama o resolver de persistência.
4. `persist=true` verifica configuração admin e bearer antes de qualquer resolver/store/database.
5. Bearer autorizado segue o resolver já configurado para o serviço; nenhum fallback a `DATABASE_URL` é introduzido por esta fatia.
6. Dependências de store/connector podem ser injetadas por seam de teste para contar resoluções e provar zero acessos nos ramos 503/401.
7. Respostas e logs não ecoam token, Authorization header ou URL de banco.

A bifurcação de auth e a ordem de acesso a DB são contratos comportamentais. Nomes de tipos Rust, assinatura dos helpers e mecanismo de injeção são detalhes de implementação que o Critic pode revisar em G1.

## Validação TDD proposta

Nenhum teste foi escrito ou executado nesta etapa. Após G1, cada comportamento deve ser demonstrado em RED antes da implementação mínima e GREEN depois:

| Caso comportamental | Evidência esperada |
|---|---|
| `persist=false` sem token, payload válido | cálculo bem-sucedido; contador do resolver permanece 0 |
| `persist=true` sem token | 503 + `admin_auth_not_configured`; resolver/store/connector permanecem em 0 |
| `persist=true` com token ausente ou fraco | mesma resposta 503; nenhum acesso ao store |
| `persist=true` com token forte, sem bearer | 401 + `unauthorized`; resolver/store/connector permanecem em 0 |
| `persist=true` com token forte e bearer errado | 401 + `unauthorized`; resolver/store/connector permanecem em 0 |
| `persist=true` com bearer válido | handler resolve o store uma vez e delega persistência; teste de escrita somente no PG efêmero validado |
| POST para rota desconhecida sob prefixo admin sem token, com token errado e com token válido | 404 + `route_not_found` em todos os casos; handler e auth protegida não são chamados |
| Outras cinco rotas públicas de cálculo | resposta pública normal sem token; nenhum store/URL/connector |

Os testes unitários de 503/401 e cálculo público usam connector falso e não carregam `.env`. O caso bearer válido com escrita depende do runner PostgreSQL descartável; não usar o banco runtime nem a ordem Binance para validar este contrato. A infraestrutura completa do runner permanece dependência de W0-02/T-W0-06.

## Alternativas e trade-offs

- **Auth em toda a rota SMA:** simples, mas quebra o contrato de cálculo público mesmo quando `persist=false`; rejeitada.
- **Auth somente depois de resolver o store:** preserva resposta normal, mas permite resolução/acesso a recurso antes da autorização; rejeitada.
- **Auth condicional após parse limitado e sem I/O:** conserva o cálculo público, autoriza gravação antes do store e mantém uma única rota; proposta.
- **Auth por catch-all de prefixo antes da correspondência:** pode converter rota inexistente em 401/503; rejeitada em favor de 404 antes da auth.

## Riscos e rollout/rollback

O principal risco é código de persistência continuar alcançável antes da auth ou um helper abrir conexão ao ser construído. Os contadores de resolver/store/connector devem provar a ordem, além do status HTTP. Outro risco é a auth prefix-catch-all mascarar 404; testes precisam cobrir o caminho HTTP real para rota admin inexistente.

Rollout local após G1: implementar os casos negativos e o cálculo público com seams em memória; revisar independentemente; habilitar persistência somente no teste PG efêmero após W0-02/T-W0-06 e no `serve` autenticado. Não iniciar serviço nem tocar no banco operacional nesta fase documental. Rollback reverte apenas a bifurcação de rota/handler adicionada, mantendo o gate de auth e a resposta 404; nunca reabrir persistência sem auth.

## Decisão e revisão

O owner já aprovou os seams de auth descritos em [HTTP admin bearer seam](./http-admin-auth-seam-sdd.md); esta proposta detalha a aplicação condicional ao backtest e o comportamento de rota desconhecida. A aprovação do contrato-base não substitui o G1 deste documento. Status permanece PROPOSED até revisão técnica independente. Critic atribuído: `/root/test_safety_sdd_critic`, após liberação do slot por W0-02.
