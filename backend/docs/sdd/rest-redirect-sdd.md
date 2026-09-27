---
title: SDD — Restringir redirects REST do monitor Binance
description: Especificação da política de redirects REST same-origin no cliente Binance.
tags:
  - sdd
  - backend
  - seguranca
  - binance
  - rest
  - redirect
---

# SDD — Restringir redirects REST do monitor Binance

- **ID:** T-05
- **Autor:** System Designer (Builder)
- **Revisor:** Crítico de Arquitetura independente, designado pelo Orquestrador
- **Estado:** G1 tecnicamente aprovado por Crítico independente; usuário aprovou os seams em 2026-09-26; C9 G3 aprovado com follow-up após parecer complementar de `/root/c16_critic`; C10/G4 pendentes
- **Data:** 2026-09-26

## Contexto e objetivo

O monitor usa `BinanceMarketData::candles` para obter OHLCV público da conta Spot `dev`, cujo endpoint inicial é fixado em `https://testnet.binance.vision/api/v3`. O SDD anterior documentou que `ccxt-core` 0.1.5 constrói `reqwest::Client` sem política de redirect; portanto `fetch_ohlcv_v2` pode seguir uma resposta HTTP 3xx para outra origem. Esse método chama `load_markets(false)` antes de buscar candles, de modo que o carregamento de mercados também faz parte do caminho REST efetivo a proteger.

**Objetivo:** nenhuma requisição REST disparada pelo caminho OHLCV deve seguir um redirect para origem diferente da URL inicial de sua respectiva requisição. Redirects HTTPS dentro da mesma origem testnet podem continuar, sujeitos ao limite atual de dez saltos. Um redirect rejeitado vira erro de poll; o monitor continua em execução e mantém o fallback WS/REST já existente. Sucesso exige teste HTTP observável de tentativa entre origens bloqueada e de redirect na mesma origem aceito, além de confirmação de que o adaptador usa o cliente corrigido.

**Não objetivos:** habilitar ordens, modo de produção, HFT, contas privadas/futures, novas origens REST, proxy configurável, ou alterar estratégia, risco, persistência e política de fallback. Credenciais continuam fora de logs e das chamadas públicas de OHLCV.

## Design e seams propostos para acordo do usuário

O seam público do produto permanece `BinanceMarketData::candles(symbol, timeframe, limit) -> BotResult<Vec<Candle>>`, incluindo erro de mercado quando a requisição REST é recusada. A construção de `BinanceMarketData::new` continua exigindo a conta Spot `dev` e a origem exata testnet. Não acrescentar opção de configuração que permita relaxar a origem.

Aplicar `[patch.crates-io]` para uma cópia local, versionada e fixa de `ccxt-core` 0.1.5. Alterar somente a construção do cliente HTTP em `HttpClient::new`: instalar `reqwest::redirect::Policy::custom` que obtém a **primeira URL da cadeia** com `Attempt::previous().first()`. Se a cadeia vier vazia, a política falha fechada com `Attempt::error`, sem indexação direta nem acesso inseguro. Comparar a URL de destino com essa primeira URL por `scheme`, `host`, `port_or_known_default()` e ausência de userinfo em ambas; recusar mudança de origem, downgrade, userinfo e mais de dez redirects com `Attempt::error`, não com `stop`, para que o poll receba um erro explícito. Para conservar o limite padrão do reqwest, recusar quando `Attempt::previous().len() > 10`. Na biblioteca genérica, a política aceita uma origem inicial HTTP ou HTTPS, mas nunca troca entre elas; a restrição HTTPS *testnet* permanece no adaptador. A comparação deve usar URLs parseadas, sem prefixos textuais. A política se aplica ao cliente usado por `load_markets` e `fetch_ohlcv_v2`, e não apenas a uma verificação posterior do URL da resposta.

O seam HTTP testável da dependência é `ccxt_core::http_client::HttpClient::get` (ou o método público equivalente já usado pelo adaptador): um servidor local A retorna 302 para a mesma origem ou para um servidor B. A prova dinâmica exige que B não receba conexão no caso entre origens e que A entregue o corpo esperado após redirect na própria origem; verificar ambos os efeitos, além do erro ou sucesso retornado ao cliente. Para evitar DNS/TLS real, a lógica de comparação de origens será extraída para função pura interna que aceita URLs HTTP e HTTPS. Um teste unitário dessa função cobre explicitamente HTTPS→HTTP, pois servidores HTTP locais não demonstram um downgrade TLS; outros casos incluem porta efetiva, userinfo e cadeia vazia na política. O teste local HTTP verifica que a política instalada no cliente realmente impede uma conexão à segunda origem. O adaptador do produto continua exclusivamente HTTPS testnet. O consumidor externo observa o erro pelo seam `candles`; em testes do projeto, o adaptador deve confirmar a origem inicial e a resolução do patch no `Cargo.lock`, sem ampliar a allowlist em produção.

**Acordo requerido antes de escrever testes:** manter `BinanceMarketData::candles` e `HttpClient::get` como limites comportamentais; permitir redirects apenas dentro da mesma origem inicial (sem userinfo e com máximo de dez saltos); recusar redirects externos antes de conectar ao destino. O Crítico deve aprovar este SDD antes de implementação.

## Preflight T-17 (somente leitura)

A inspeção do pacote local `ccxt-core` 0.1.5 confirmou que `src/http_client/builder.rs` constrói o `reqwest::Client` em `HttpClient::new`, ponto de inserção proposto, e que o manifesto do pacote declara `license = "MIT"`. Não foi encontrado arquivo de licença na extração local consultada; os metadados isolados não bastam para afirmar que a atribuição necessária foi preservada. C9 deve verificar o texto e a proveniência da licença em fonte verificável antes de copiar o pacote. Este preflight não escreveu testes nem código, não altera o seam público e não fecha o acordo do usuário nem G3.

## Alternativas e decisão

| Alternativa | Consequência | Decisão |
|---|---|---|
| Patch local de `ccxt-core` | Mudança pequena de comportamento em um cliente já usado por `load_markets` e OHLCV; cópia local aumenta manutenção e exige acompanhamento de atualizações. | Proposta. Preserva parsing, rate limiting, timeout e API `ccxt` atual. |
| Adaptador HTTP próprio para `exchangeInfo` e `klines` | Evita fork, mas exige substituir descoberta de mercado, parsing, limites, retries e controle de taxa; área de regressão maior para o monitor. | Adiar; reconsiderar ao atualizar/remover `ccxt`. |
| Negar todos os redirects | Regra simples e segura, porém pode rejeitar redirects legítimos da mesma origem e reduzir disponibilidade. | Não preferida. |
| Validar apenas `response.url()` após `ccxt` | Detecta desvio depois de conectar ao destino externo; não fecha o risco. | Rejeitada. |

## Entregas, TDD e verificação

1. **C9 — política na dependência.** Após acordo dos seams e aprovação G1, o Builder verifica antes de copiar a dependência a proveniência do pacote `ccxt-core` 0.1.5 e a licença MIT: versão e checksum do registro/Cargo.lock, metadados de licença e texto de licença/atribuição aplicável em fonte verificável. Preserva a licença e os avisos de atribuição na cópia; se não conseguir estabelecer esses dados, interrompe o vendor e devolve a decisão ao Orquestrador. Então escreve primeiro testes red na cópia local: 302 de uma origem A para B não atinge B; 302 dentro de A retorna o corpo esperado; cadeias com userinfo e mais de dez saltos falham; o teste unitário puro cobre HTTPS→HTTP e a cadeia vazia falha fechada. Implementa o mínimo no construtor `HttpClient::new` e confirma green. Os testes não fazem chamadas externas. Verificar com `cargo tree -i ccxt-core` e o `Cargo.lock` que o patch local é o único `ccxt-core` resolvido e que `ccxt-exchanges` o consome; o teste HTTP deve exercitar a política instalada no `HttpClient`, além de qualquer função pura auxiliar.
2. **C10 — contrato do produto e docs.** Um teste do adaptador confirma conta/endpoint testnet e que erro REST preserva o estado de monitor/fallback existente; usar teste local de transporte apenas se puder injetar o cliente sem abrir URL de produção. Revisar README e SDD anterior para trocar o risco residual por garantia e limite efetivamente testados. Executar `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --locked` e `git diff --check`. Registar qualquer integração dependente de PostgreSQL como não executada, sem tratá-la como aprovação.

Cada artefato de C9/C10 requer Builder e Crítico independentes. Achados bloqueantes retornam ao Builder. O Orquestrador audita diff e evidências, sem substituir o Crítico.

## Riscos, operação e reversão

- **Dependência vendorizada:** a cópia aumenta o custo de atualização e deve manter versão/licença MIT, atribuição aplicável, proveniência verificável e um diff mínimo identificado. A alteração em `HttpClient::new` afeta todos os consumidores de `ccxt-core` dentro do binário, inclusive outros endpoints e exchanges que venham a ser usados; testar os caminhos REST atualmente ativos e reavaliar a política antes de ativar outros. Atualizações futuras precisam reaplicar e testar a política.
- **Redirect legítimo externo:** será recusado e produzirá falha de poll observável. Isso é intencional; não aceitar destino externo automaticamente para recuperar disponibilidade. O monitor segue operando, com WS se disponível e novas tentativas REST no intervalo configurado.
- **Proxy e DNS:** política de redirect restringe URLs, não garante o IP final nem protege contra proxy ou DNS comprometidos. O projeto não configura proxy nesta via. Não alegar contenção de rede além do redirect HTTP.
- **Rollback:** reverter C9/C10 por Git restaura o comportamento anterior, inclusive o risco de redirect; não há migração de dados. Não há deploy autorizado neste trabalho; qualquer lançamento exige G5 explícito.

**Jev:** não aplicável; a regra de segurança é determinística e deve ser verificada por testes HTTP e revisão humana.

## Evidência de C10 — 2026-09-27

- O teste existente `exchanges::binance::tests::public_rest_endpoint_uses_registered_spot_testnet_origin` verifica que a conta Spot `dev` selecionada configura o endpoint público inicial `https://testnet.binance.vision/api/v3`; outro teste recusa origem externa mesmo com credenciais presentes.
- O novo teste `app::tests::rest_poll_error_preserves_ws_evaluation_and_monitor_state` injeta uma fonte de mercado simulada no mesmo contrato `MarketDataSource` usado por `BinanceMarketData::candles`: após uma janela REST válida, ela retorna `BotError::MarketData` representando redirect recusado. O loop mantém `Running`, mostra o erro e aceita um candle WS posterior que atualiza o mercado e limpa o erro. Esse teste passou na primeira execução; é uma prova de regressão do comportamento já existente, **sem alegação de ciclo red→green ou alteração de produção**.
- O teste do loop não chama a Binance nem demonstra a política TLS no adaptador real. A conexão HTTP dinâmica e a política instalada são cobertas pelos testes C9 em `redirect_policy_test.rs`/`redirect_origin_test.rs`. DNS, proxies e uma sessão TUI com rede real continuam fora da evidência. C10 permanece sob revisão independente; G4 e lançamento não são declarados aprovados.
- Nesta rodada, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --locked`, `cargo tree --locked -i ccxt-core` e `git diff --check` passaram. A suíte registrou 93 testes unitários aprovados e um teste PostgreSQL ignorado; os testes de integração somaram mais 10 aprovados, incluindo 2 de redirect HTTP. A integração PostgreSQL não foi executada.

## Evidência de C9 — 2026-09-27

- O pacote local `vendor/ccxt-core-0.1.5` preserva a versão 0.1.5 e a licença MIT obtida do commit indicado nos metadados VCS do pacote. A origem, o checksum anterior do registro e o diff local estão registrados em `vendor/ccxt-core-0.1.5/PROVENANCE.md`.
- O consumidor fixa o patch em `[patch.crates-io]`; `cargo tree --locked -i ccxt-core` mostrou uma única instância local usada diretamente e por `ccxt-exchanges`.
- Ciclo TDD da regra pura: `cargo test --locked --test redirect_origin_test` falhou 3/3 com implementação permissiva e passou 3/3 após a política de origem. Os casos verificam porta efetiva, outro host, downgrade, userinfo, histórico vazio e mais de dez saltos.
- O teste de transporte em `tests/redirect_policy_test.rs` passou fora do sandbox com loopback permitido: a origem B não recebeu conexão no redirect entre origens e o redirect na própria origem retornou o corpo esperado. O teste mantém limite global de quatro segundos por caso.
- O Crítico independente identificou um falso positivo possível no primeiro teste HTTP: o listener B expirava após 300 ms, antes de `get` necessariamente terminar. O Builder corrigiu o teste para manter B ativo até o retorno de `get`, com prioridade para conexões pendentes; o Crítico aceitou essa correção.
- Parecer complementar independente de `/root/c16_critic` para C9 G3: **APROVADO COM FOLLOW-UP**, após reexecução fora do sandbox de **2/2** testes HTTP de transporte e **3/3** testes puros de origem. A política pura e o cliente instalado passaram; a manutenção futura deve repetir a prova após atualizações do vendor. A restrição cobre destinos de redirect por URL, sem demonstrar contenção contra DNS ou proxy comprometidos. C10 (contrato do produto e documentação final) e G4 permanecem pendentes.
