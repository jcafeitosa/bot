---
title: SDD — Remoção de arquivos legados de configuração e migração
description: Design da remoção de arquivos legados e migração de configuração
tags:
  - sdd
  - backend
  - configuration
---

# SDD — Remoção de arquivos legados de configuração e migração

- **ID:** T-13
- **Autor:** System Designer Builder (`/root/legacy_cleanup_designer`)
- **Revisor:** Crítico de Arquitetura (`/root/resume_design_critic`)
- **Status:** G1 e G3 aprovados pelo Crítico de Arquitetura (`/root/resume_design_critic`) após revisão independente. Os seams públicos foram preservados sem novos testes.
- **Data:** 2026-09-26

## Contexto e objetivo

Antes da limpeza T-13, três arquivos versionados permaneciam em locais que não eram fontes de leitura do backend Rust. Isso deixava caminhos concorrentes aparentes e, no caso da configuração da Binance, dois schemas incompatíveis. O objetivo era deixar uma única localização documentada por responsabilidade, sem alterar o comportamento de inicialização, seleção de contas, presets ou migrações.

## Evidência de uso e decisão por arquivo

| Arquivo candidato | Evidência no baseline anterior à remoção | Decisão de design |
|---|---|---|
| `migrations/0001_market_data.sql` | SHA-1 idêntico a `src/persistence/migrations/0001_market_data.sql` (`8a68c4abe0dd6e18dab1c4c9cb2257521a752fd9`). `Database::migrate` em `src/persistence/mod.rs` cria `Migrator` apenas a partir de `src/persistence/migrations`. CI executa esse código no teste de integração PostgreSQL. | Remover somente a cópia em `migrations/`; preservar integralmente a migração ativa e seu nome/checksum. |
| `exchanges/config/binance.toml` | `load_registry` em `src/exchanges/bootstrap.rs` enumera apenas `src/config/exchanges`, ancorado em `CARGO_MANIFEST_DIR`. Testes de `account_file.rs` leem a mesma fonte ativa. O arquivo legado usa `[exchange]`, `[spot]` e `[futures]`, enquanto `ExchangeConfigFile` exige `exchange` e `[accounts.spot]`/`[accounts.futures]`. | Remover o arquivo legado; preservar `src/config/exchanges/binance.toml` e o carregamento fail-closed. |
| `src/config/profiles.toml` | Nenhum caminho de produção carrega este TOML. `OperationMode::supported_timeframes` e `strategy::periods_for_mode` definem os presets em Rust; `Config::validate_operation_profile` os aplica. O teste em `config/mod.rs` até exige que o erro de preset não mencione `profiles.toml`. README já afirma que Rust é a fonte dos presets. | Remover o arquivo ilustrativo; preservar enums, períodos e validação atuais. |

A busca textual recursiva no baseline anterior à remoção, em código, testes, `.github/workflows/backend-ci.yml`, `docker-compose.bot.yml`, README e docs, encontrou somente as referências acima e a menção dos três candidatos em `docs/backend-corrections-sdd.md`. O README então afirmava que `migrations/` não existia, mas o diretório ainda estava presente. A análise do repositório não prova que ferramentas externas não versionadas nunca usem esses caminhos; este é o risco residual para operadores.

## Seams públicos e comportamento preservado

1. `Config::load` e o CLI `monitor`/`backtest --config` continuam lendo `src/config/bot.toml` ou o caminho fornecido. Os modos e combinações de timeframe/SMA aceitos ou rejeitados não mudam.
2. `load_registry(Environment)` continua lendo somente `src/config/exchanges/*.toml`; a conta Spot de desenvolvimento e seus endpoints continuam vindos de `src/config/exchanges/binance.toml`.
3. `Database::migrate` continua aplicando `src/persistence/migrations/0001_market_data.sql`; o comando manual SQLx continua exigindo `--source src/persistence/migrations` a partir de `backend/`.
4. Nenhum schema TOML, API Rust, formato JSON ou migração SQL ativa muda. Não criar fallback para caminhos legados.

Antes de escrever **qualquer novo teste** desses seams, obter o acordo explícito do usuário com as interfaces públicas acima, conforme `AGENTS.md`. A implementação proposta é remoção de arquivos e ajuste documental; não requer novos testes que apenas espelhem a ausência física dos arquivos.

## Alternativas

- **Manter os arquivos como exemplos:** perpetua duas fontes aparentes; o TOML legado da Binance tem schema incompatível e pode induzir operadores a editarem o arquivo errado.
- **Mover o loader para os caminhos legados:** alteraria comportamento e contratos já documentados, e faria o SQLx depender novamente de um diretório padrão que a aplicação não usa.
- **Criar sincronização/cópia automática:** introduz manutenção e risco de divergência sem caso de uso demonstrado.

A remoção das três cópias, sem redirecionamento automático, é a menor mudança coerente com os loaders existentes.

## Riscos e controles

- **Uso externo não versionado:** scripts locais podem chamar `sqlx migrate run` sem `--source` ou editar os TOMLs legados. README deve manter instrução explícita do SQLx CLI e indicar o único arquivo de conta ativo. Anunciar na descrição do CL os caminhos removidos e as substituições; não prometer compatibilidade de caminhos não documentados.
- **Migração alterada por engano:** comparar hash/bytes antes da remoção e verificar no diff que `src/persistence/migrations/0001_market_data.sql` permaneceu intacta. Usar banco PostgreSQL efêmero e isolado para o teste de integração; nunca rodar migrações de teste em banco compartilhado ou operacional.
- **Configuração errada após limpeza:** confirmar que `load_registry` ainda usa apenas a configuração ativa e que `Config::validate` mantém os presets; não tocar em credenciais nem endpoints.

Jev: não aplicável — a decisão é determinada por referências locais e comportamento observável dos loaders; uma classificação probabilística não melhora a evidência.

## Validação proporcional e plano de CL

**CL único e pequeno:** excluir os três arquivos legados e atualizar `backend/README.md` (incluindo a instrução SQLx) e `backend/docs/backend-corrections-sdd.md` para registrar a limpeza concluída. Preservar os três arquivos ativos citados e evitar refatorações adjacentes.

1. Antes da edição, registrar `git status --short`, comparação byte a byte das duas migrações e baseline de `cargo test`/comportamentos de configuração e registro. A remoção não introduz comportamento novo capaz de produzir um ciclo red/green útil; para este artefato não-code, a validação observável é comparar os comportamentos públicos antes e depois, além de verificar que a documentação aponta somente às fontes ativas. Se um novo teste de comportamento for necessário por achado do Critic, acordar primeiro o seam com o usuário e executar red/green.
2. Remover os candidatos. Confirmar por `rg` que não restam referências operacionais aos caminhos removidos; permitir apenas registro histórico explícito em SDD/ADR. Confirmar que os arquivos ativos existem e estão inalterados.
3. Executar `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` e comandos CLI apropriados para configuração/backtest. Executar `persist_dataset_round_trip` apenas no banco isolado provisionado em CI ou em ambiente de teste explicitamente descartável; registrar se não houver tal banco local. `git diff --check` e revisão independente encerram o CL.

## Rollback e gate

Rollback é restaurar apenas os três arquivos removidos e a documentação do mesmo CL; não reverter ou duplicar a migração ativa. G1 requer aprovação do Critic deste SDD. G2 registra Builder/Critic do CL e acordo do usuário com seams caso novos testes sejam escritos. G3 exige evidências acima e LGTM independente; nenhuma exclusão é considerada aceita apenas por este documento.

Na árvore após a implementação T-13, os três candidatos foram removidos. `src/persistence/migrations/0001_market_data.sql`, `src/config/exchanges/binance.toml` e os presets em Rust continuam como fontes ativas. O Crítico de Arquitetura (`/root/resume_design_critic`) emitiu LGTM para a entrega após correção de dois achados documentais; 72 testes passaram, o teste PostgreSQL permaneceu ignorado, e `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo run --locked -- backtest --config src/config/bot.toml` e `git diff --check` passaram.
