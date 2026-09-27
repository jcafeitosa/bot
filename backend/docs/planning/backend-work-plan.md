# Plano de execução das correções pendentes do backend

- **Responsável:** Orquestrador `/root`
- **Data:** 2026-09-26
- **Estado:** plano de G2 revisado e aprovado por `/root/workplan_critic`; usuário aprovou as interfaces dos quatro SDDs em 2026-09-26; C9 bloqueado em G3 por falta de prova HTTP no sandbox
- **Base:** SDDs [T-05](../sdd/rest-redirect-sdd.md), [T-07](../sdd/backtest-trades-and-slippage-sdd.md), [T-10](../sdd/monitor-pause-resume-sdd.md) e [T-15](../sdd/monitor-persistence-policy-sdd.md), todos com G1 técnico aprovado por críticos independentes

## Escopo e gates

O objetivo é corrigir o risco de redirect REST, a fixture e o custo de venda do backtest, a pausa/retomada do monitor e a semântica da persistência opcional. As entregas [T-13](../sdd/legacy-file-cleanup-sdd.md) e [T-16](../sdd/backend-module-map-sdd.md) já passaram por revisão independente; seus arquivos permanecem no worktree e não são refeitos aqui.

O `AGENTS.md` exige acordo do usuário com os seams públicos antes de escrever cada teste. O usuário aprovou explicitamente as interfaces de T-05, T-07, T-10 e T-15 em 2026-09-26. G2 está fechado quanto a esse requisito. C9 foi produzido por `/root/c9_builder` e revisado por `/root/c9_critic`; o veredito é **REPROVADO por evidência pendente** porque o sandbox negou bind em `127.0.0.1` para os dois testes HTTP obrigatórios. A política, a licença e os testes puros foram inspecionados, mas G3 de C9 não está aprovado. C10 aguarda C9. Os demais CLs independentes podem avançar com seus próprios pares. O autor nunca aprova o próprio artefato. Cada CL seguirá red → green → revisão, com documentação no mesmo CL quando aplicável.

| CL | Entrega e evidência mínima | Dependência | Builder / Critic |
|---|---|---|---|
| C9 | Política de redirect no `ccxt-core` local: servidores HTTP locais verificam bloqueio antes de contato com outra origem e redirect na mesma origem; testes puros verificam limite de saltos, origem e downgrade; árvore Cargo prova dependência única. | Interface T-05 aprovada | `/root/c9_builder` / `/root/c9_critic` — **G3 BLOQUEADO**; 2 testes HTTP sem bind |
| C10 | Contrato do adaptador e documentação: endpoint testnet e fallback após erro REST verificados; README e risco residual atualizados conforme testes reais. | C9 aprovado | Backend / Backend, instâncias a ativar |
| C12 | Fixture CLI por timeframe produz ao menos um trade fechado por sinal; teste exercita binário e `run_sma_crossover`, sem banco. | Interface T-07 aprovada | `/root/c12_builder` / `/root/c12_critic` — **G3 APROVADO**; 8 pares e next-open revisados |
| C13 | Sell aplica slippage configurado no próximo open; equação de custo e regressão de stop/take profit verificadas. | C12 aprovado | `/root/c13_builder` / `/root/c13_critic` — em andamento |
| C14 | Pause/Resume responde sem esperar REST/JEV; drena WS, reconcilia REST, descarta resultados obsoletos e confirma estado na TUI, com fontes/relógio falsos. | Acordo T-10 | Backend / Backend, instâncias a ativar |
| C15 | Produtor WS não bloqueia com canal cheio; overflow e canal fechado têm testes; README descreve perda recuperável. | C14 aprovado | Backend / Backend, instâncias a ativar |
| C16 | Opt-in PostgreSQL inválido falha antes do monitor; opt-out não abre banco; `backtest --persist` permanece independente. | Acordo T-15 | Backend / Dados, instâncias a ativar |
| C17 | Estado de persistência e recuperação REST na TUI; falha/commit incerto, lacuna, pausa e overflow exercitados com armazenamento falso. | C14, C15 e C16 aprovados | Backend / Dados, instâncias a ativar |
| V18 | Integração PostgreSQL em database `trading_bot` descartável: migração, commit, rollback após erro e idempotência observados; setup, host, resultado e limpeza registrados. | C16 e C17 aprovados; ambiente isolado acessível | QA/Dados / Crítico de Dados, instâncias a ativar; **BLOQUEADA pelo ambiente** |

C9/C10, C12/C13 e C16 podem avançar em frentes independentes depois dos acordos respectivos e da ativação dos pares. C14/C15 precisam preceder C17 porque estabelecem a autoridade de pausa, o commit de janelas REST e o descarte de eventos WS. Alterações simultâneas no README serão sequenciadas para evitar sobrescrita. Achado bloqueante ou importante volta ao Builder; após até três ciclos sem acordo, o Orquestrador arbitra sem substituir aprovação obrigatória.

## Verificação e limites

Cada CL registra testes relevantes, `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked`, `git diff --check`, diff revisado e veredito do Critic. G4 reúne regressão de monitor/backtest, revisão de segurança do redirect e revisão de dados da persistência. Não há deploy ou habilitação de ordens neste plano.

O teste PostgreSQL ignorado e V18 requerem database `trading_bot` **descartável e isolado**. A tentativa local via Colima falhou por permissão no socket e a escalada foi abortada; não se deve repetir a mesma solicitação sem nova autorização ou mudança de contexto, nem usar o container `bot-agents-postgres` existente como substituto. Até existir ambiente autorizado, V18 permanece **BLOQUEADA** e a integração **não executada**, sem alegar aprovação desse cenário. O job de CI configurado é apenas configuração, não evidência de uma execução atual.

## Próximas ações do Orquestrador

1. Reexecutar os dois testes HTTP de C9 em ambiente com bind loopback disponível e devolver evidência ao Critic; C10 depende desse veredito.
2. Ativar Builder e Critic separados para cada CL restante antes do primeiro teste.
3. Encaminhar cada entrega ao Critic, resolver achados, atualizar README/SDDs e registrar evidências por gate.
4. Executar V18 e reavaliar G4 quando houver banco de teste isolado acessível.
