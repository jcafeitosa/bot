# SDD T-W0-02b recast — proteção do alvo PostgreSQL e comportamento do gate

**Status: PROPOSED; G1 independente pendente.** Este é um SDD substituto para fechar o comportamento do gate PostgreSQL e supersede explicitamente o item escalado T-W0-02b do [SDD W0-02 anterior](./wave0-02-ci-pg-fail-loud-sdd.md), que permanece como histórico `draft` reprovado. A substituição não aprova o desenho, não autoriza implementação e não transforma vereditos de outros SDDs em autorização.

## Contexto e objetivo

O desenho W0-02b acumulou três ciclos e ainda não fechou como a identidade runtime independente do marker permanece vinculada ao endpoint/database exatos consumidos por SQLx durante o uso do pool. Esta proposta reúne os requisitos já acordados para marker fail-closed, configuração e identidade num único seam, e acrescenta lease compartilhado/exclusivo para fechar TOCTOU entre validação, conexão, uso do pool e lifecycle local.

Sucesso observável: qualquer prova ausente, inválida ou divergente bloqueia connector SQLx; target validado só conecta pela URL exata que foi comparada ao manifest e estado live; nenhuma operação de lifecycle substitui recursos enquanto um `AppDatabase`/pool mantém lease compartilhado; nenhum estado inválido pós-opt-in é convertido em skip.

## Escopo, decisões e limites

- Runtime seleciona um único `Environment`: dev usa somente `BOT_DATABASE_URL_DEV`; prod selecionaria somente `BOT_DATABASE_URL_PROD`, mas prod continua inativo/bloqueado e este SDD não habilita comando/efeito prod. Não criar `--database-environment`.
- Integração PG usa somente `BOT_PG_TEST_DATABASE_URL` e manifest/identity do runner efêmero. Presença de `DATABASE_URL` herdada, mesmo vazia ou junto da URL dedicada, é erro antes de SQLx; o snapshot carrega somente o bit de presença, nunca o valor.
- Neo4j READ e WRITE têm opt-ins independentes. Binance é exclusivamente Spot Testnet. Ausência/`0` é opt-out; opt-in literal `1` seguido de configuração/credenciais/identity/permissões/conexão inválidas resulta em erro não-zero, nunca skip.
- Marker presente vazio, inválido, múltiplo ou schema/query incompatível é fail-closed. Relação ausente só permite runtime após identidade independente validada antes do connector.
- T-DB-ENV G1 `be7c3d47e10bd0e32ddde84e582066e7dad18ea2` aprovou com follow-up a capability e o lease; o follow-up G3 offline de retenção do lease continua requisito, não evidência de implementação. Este SDD integra esse contrato; não invoca o veredito para autorizar trabalho de implementação.
- W0-06 é pré-requisito de integração: conforme o estado informado pelo Orquestrador, G1 foi aprovado com follow-up, mas a validação G3 da topologia de egress ainda precisa ser implementada/revisada antes de habilitar o perfil de integração correspondente. Nenhuma etapa G4, acesso a DB/Docker/CI/Testnet ou produção faz parte deste desenho.

Não inclui migrações em banco persistente, conexão remota, ativação de prod/live, alteração de workflows, execução de testes ou inicialização de containers/serviços.

## Autoridade e identidade do alvo

URL, nome do database, hostname/porta, `.env`, `DATABASE_URL` e marker não são autoridade de identidade. A prova independente combina:

1. `Environment` efetivo entregue pela configuração central e URL candidata exclusiva (`BOT_DATABASE_URL_DEV` ou `BOT_DATABASE_URL_PROD`);
2. uma única entrada do manifest runtime `backend/.local/db-targets/<environment>.json`, gerada pelo lifecycle local; manifest sozinho não autoriza;
3. Compose versionado do perfil selecionado, que fixa project/service/labels e recursos esperados;
4. inspeção ao vivo do Docker daemon local via context/socket Unix local validado, comparando daemon/context, environment-id, project/service/labels, container ID e estado, image digest/ID, volume ID/mount, binding host-port→5432, database e role;
5. parse local da URL candidata e comparação de scheme/opções permitidas, host loopback, porta numérica, path/database e username/role com manifest e estado live. Credenciais nunca aparecem em erros/logs.

Overrides `DOCKER_HOST`, `DOCKER_CONTEXT`, context SSH/TCP/HTTP, daemon desconhecido/remoto, manifest ausente/duplicado/corrompido ou qualquer mismatch encerram antes de connector. Não se abre PostgreSQL para descobrir identidade.

O verifier só emite `VerifiedLocalPostgresTarget` após todas as comparações. A capability é opaca, fields/constructor privados, sem `Default`, `Deserialize`, `FromStr`, builder público, serialização/cache ou accessor de URL substituível. Ela mantém privadamente `Environment`, identidade e a própria URL candidata verificada. O connector aceita somente essa capability e entrega ao SQLx exatamente os bytes da URL guardada; não aceita Environment/URL paralelos, não relê configuração e não reconstrói o destino. Mismatch/proof ausente produz zero tentativas SQLx.

Para runner de integração, a mesma forma de capability/lease liga o manifest do run ao endpoint privado e database do container efêmero. A URL é somente `BOT_PG_TEST_DATABASE_URL`; sua presença não corrige divergência de identidade.

## Seam de lease compartilhado/exclusivo

Usar um interprocess read/write lease com chave estável que possa ser determinada sem ler o manifest: runtime usa `backend/.local/locks/postgres-<environment>.lock` selecionado pelo `Environment` validado pela configuração central (por exemplo, `postgres-dev.lock`); runner PG efêmero usa `postgres-pg-test-<run-id>.lock`, onde o `run-id` é criado pelo processo supervisor antes de escrever/ler o manifest. Todos os callers runtime e lifecycle suportado usam o mesmo provider e a mesma chave; não há lock baseado em URL, nome de database ou campo ainda não verificado do manifest.

- Verifier adquire lease **compartilhado antes** de ler manifest, config de target ou inspecionar daemon/container. Falha/timeout/corrupção de lock resulta em erro e zero connector.
- Mantendo lease, verifica autoridade/manifest/live state e URL; revalida manifest, IDs, digest, volume/mount, porta, database e role imediatamente antes de emitir capability/conectar.
- Capability mantém o lease durante a chamada do connector. Se a conexão/pool falhar, capability é descartada e lease libera.
- Em sucesso, `AppDatabase` assume o lease junto com o pool e o conserva até fechar e descartar o pool e todas as referências/clones de `AppDatabase`. O wrapper não expõe um pool que possa sobreviver sem o lease. A capability pode ser movida, não clonada para liberar prematuramente; se o app clona handles, todos compartilham a mesma posse refcounted do lease.
- Toda ação lifecycle que possa criar, parar, substituir ou remover container/volume, alterar manifest, environment-id, host-port ou identidade (incluindo up/start/stop/restart/recreate/migrate/cleanup) adquire lease **exclusivo antes** de observar ou mutar recursos. Com leitores ativos, aguarda sem mutar ou falha de forma explícita; nunca substitui recursos enquanto houver lease compartilhado.
- Mudança de identidade entre primeira observação e connector deve ser impossível para lifecycle conforme o lock; se um observer fake ou estado externo revelar mismatch/revalidação divergente antes da conexão, aborta com zero SQLx. Mutação manual do daemon que ignora o lock é operação fora de suporte: callers devem ser encerrados e a identidade revalidada antes de reconectar.

A semântica de lock (aquisição, posse até drop/close, compartilhamento entre processos, exclusão contra lifecycle, liberação após crash/process exit e chaves por target/run) é contrato observável; escolher o primitive de implementação em G3 não pode enfraquecê-la.

## Semântica do gate e marker

| Estado | Resultado |
|---|---|
| Verifier de identidade não produz capability | erro estável antes de SQLx, pool, migration ou startup |
| Tabela marker ausente + identity capability válida | `UNMARKED_RUNTIME`; runtime pode criar pool somente com URL vinculada à capability e lease ativo |
| Tabela presente, vazia | `MARKER_LOOKUP_FAILED`; não cria pool de aplicação nem migra |
| Uma linha marker válida | `TEST_DATABASE_MARKER_PRESENT`; runtime recusa antes do pool/migration |
| Linha inválida, múltiplas linhas, schema incompatível, timeout, permissão negada, conexão/query falha ou estado ambíguo | `MARKER_LOOKUP_FAILED`; fail-closed |

Classifier/query SQL é a mesma função usada pelo preflight runtime e exercitada pelo helper test-only seguro. Marker nunca atesta locality/identity. Identity inválida é rejeitada antes de connector; para alvo validado, lookup SQL pode usar conexão transitória, mas só ausencia inequívoca libera a criação do pool. Tabela vazia não é ausência.

Configuração central `PgIntegrationSettings` é o único reader de ambiente. Inclui `database_url_present` sem reter valor e gates PG, `BOT_RUN_NEO4J_READ_INTEGRATION`, `BOT_RUN_NEO4J_WRITE_INTEGRATION`, `BOT_RUN_BINANCE_TESTNET_ORDER`, credenciais/config necessárias e sinais de override Docker. Não há env read direto/indireto em `core/persistence/pg_integration.rs`; `verify-backend-gates.sh` não ganha allowlist.

| Perfil | Opt-out | Literal `1` + estado inválido |
|---|---|---|
| PG (`BOT_RUN_PG_INTEGRATION`) | ausência/`0`: ignored; sem URL/connector | URL dedicada/manifest/identity ausente ou `DATABASE_URL` presente: erro não-zero antes de SQLx |
| Neo4j READ | ausência/`0`: skip sem resolver/driver/conexão | target/config/credencial/conexão/read-only proof inválidos ou inconclusivos: erro não-zero antes de query/assertion |
| Neo4j WRITE | ausência/`0`: skip sem resolver/driver/conexão | target/config/credencial/conexão/permissão de escrita inválidos: erro não-zero; READ não habilita WRITE |
| Binance Spot Testnet | ausência/`0`: zero cred lookup/client/transport/submit | chave/secret Testnet ausente/parcial/inválida, target ou conexão inválidos: erro não-zero; não há endpoint/chave Live |

Valores de gate diferentes de `0` ou `1` são erro de configuração não-zero. READ exige prova de escrita negada server-side com o mesmo principal/driver/session do teste, conforme W0-06. Opt-in nunca encaminha credenciais `BINANCE_PROD_*`.

## Testes comportamentais planejados (TDD)

Todos os testes abaixo são requisitos de desenho, não executados nesta tarefa.

| Comportamento público | Fixture/fake RED→GREEN | Critério observável |
|---|---|---|
| alvo não marcado + identity válida | verifier fake confirma manifest/live/URL; marker fake ausente | emite unmarked e pool fake recebe exatamente URL capability; lease segue vivo |
| marker vazio | lookup fake devolve relação presente sem linhas | fail-closed; zero pool factory/migration/startup |
| marker válido/presente ou lookup inválido/erro | lookup fake retorna estado tipado | marker presente/erro impede pool; contadores zero |
| identity mismatch | variar environment, project/service, daemon, container, digest, volume/mount, port, database, role ou URL | verifier falha; connector SQLx fake = 0 |
| candidate URL substituída após verify | fixture tenta passar outra URL ou construir capability em módulo externo | API não oferece argumento/construtor; compile-fail; se o estado fake divergir, zero connector |
| inherited `DATABASE_URL` | snapshot só marca presença, incluindo valor vazio; URL dedicada válida permanece configurada | erro estável e zero connector |
| opt-out vs requested-invalid | absent/`0` Neo4j READ/WRITE/Binance e depois literal `1` com config/credenciais/permissões/conexão ruins | opt-out: zero resolver/client/driver/submit; solicitado inválido: erro não-zero, nunca skip |
| lease lifecycle | AppDatabase fake mantém shared lease; lifecycle fake solicita exclusive | lifecycle não substitui target enquanto pool/clone existe; após close/drop adquire exclusive |
| mutation/mismatch pre-connect | sob shared lease, fake lifecycle tenta trocar manifest/port/container ou verifier observa divergence entre checks | operação lifecycle bloqueada/falha; caso de mismatch retorna erro e connector attempts = 0 |
| release on connector failure | connector fake falha depois de capability válida | capability/drop libera shared lease; sem pool sobrevivente |

**Sete cenários SQL do marker (G3 congela, G4 executa somente após autorização):**

| Estado transacional | Resultado esperado do lookup/classificador real |
|---|---|
| Relação ausente com identidade independente validada | `UNMARKED_RUNTIME` |
| Relação presente vazia | `MARKER_LOOKUP_FAILED` |
| Uma linha completa válida | `TEST_DATABASE_MARKER_PRESENT` |
| Uma linha inválida | `MARKER_LOOKUP_FAILED` |
| Linhas múltiplas | `MARKER_LOOKUP_FAILED` |
| Schema/colunas incompatíveis | `MARKER_LOOKUP_FAILED` |
| Falha SQL controlada | `MARKER_LOOKUP_FAILED` |

Cada cenário fixture usa apenas conexão de `database_for_integration_test()` e rollback na mesma transação. Identidade ausente/inválida não é cenário SQL: fake verifier deve comprovar zero connector antes de lookup. Em G3 o teste SQL é escrito/congelado, sem serviço; Critic revê o teste. G4 autorizado observa RED pré-correção e retorna a G3 para fix/revisão; somente depois G4 reexecuta o mesmo teste para GREEN. Setup/identity failure não conta como RED, e as evidências offline G3 e SQL G4 ficam separadas.

Public API seam para callers é `Environment + central config -> verifier -> VerifiedLocalPostgresTarget(lease, exact URL) -> connector(target only) -> AppDatabase(pool + retained lease)`. Teste estrutural prova que callers não constroem SQLx connector nem descartam lease. Unit tests de gates/resolver usam snapshot tipado e spies/fakes; nenhuma variável global ou serviço é lido.

## Fases, dependências e evidências

- **G1:** Critic independente aprova este SDD de substituição antes de implementação.
- **G3:** RED/GREEN offline para verifier, URL binding, capability API, marker-policy fakes, snapshot/gates, lease shared/exclusive, lifecycle interlock, ownership até pool drop e zero connector em mismatch. Critic revisa implementação e fixtures.
- **Dependência W0-06:** o status informado é G1 aprovado com follow-up; topologia G3 precisa estar implementada, observada e revisada antes de habilitar qualquer integração que dependa daquele perfil de egress. O gate de integração permanece disabled até esse pré-requisito G3.
- **G4:** nenhum SQL real até G1/G3 pertinentes aprovados e autorização de execução específica. Se autorizados, somente runner PG efêmero isolado e validado; registrar sete cenários SQL marker separados de evidências offline. Nada neste SDD autoriza DB persistente, Docker, CI, Neo4j, Binance Testnet ou ambiente prod.

## Alternativas, riscos e mitigação

- **Confiar só no marker ou URL/nome/localhost:** rejeitado; não prova target nem vincula o destino que SQLx consumirá.
- **Verificar uma vez e soltar lock antes do pool:** rejeitado; lifecycle poderia substituir target entre verificação/conexão ou enquanto o pool é usado.
- **Lease compartilhado só durante connector:** rejeitado; ainda permite substituição com conexões/pool ativos.
- **Lifecycle muta apesar de lease ativo:** rejeitado; viola o contrato de exclusão. Deve esperar ou falhar sem mutação.
- **Invalidar opt-in solicitado como skip:** rejeitado; esconde credenciais, permissão, connectivity e setup quebrados.
- **Riscos residuais:** advisory locks coordenam apenas processos que usam o lifecycle suportado; edição/manual mutation de Docker que ignora o lock é fora de suporte. Handles clonados precisam compartilhar posse de lease. Crash do processo deve liberar o lock pelo sistema operacional; manifest stale ainda exige verificação live e mismatch fail-closed. Ambientes sem daemon/primitive suportado não conectam.

## Rollout e rollback

Após G1, implementar primeiro o seam offline de verifier/capability/lease com alvos dev fake; verificar RED/GREEN e ownership do pool antes de runners. Depois conectar os gates de configuração sem ativar perfis externos. Só habilitar integração após W0-06 G3 e approvals G4 específicos. Prod permanece disabled até nova proposta e aprovação explícitas; nenhuma credencial presente altera essa política.

Rollback reverte coordinator, capability/lease wiring e lifecycle interlock como unidade, mantém erros fail-closed e gates externos disabled. Fechar `AppDatabase`/pool primeiro para liberar leases; rollback não muda manifest/container/volume, não migra nem apaga banco e não restaura `DATABASE_URL` como fallback. Recursos efêmeros só podem ser limpos por run-id depois de obter lease exclusivo e validar identidade.

## Critério de conclusão

Critic independente aprova esta revisão G1; G3 prova os seams e ownership sem egress/serviços; G4 somente após aprovações necessárias registra qualquer SQL real autorizado. Até esses eventos, o status continua PROPOSED, o T-W0-02b anterior segue superseded/histórico draft reprovado e nenhuma implementação ou operação externa é autorizada.