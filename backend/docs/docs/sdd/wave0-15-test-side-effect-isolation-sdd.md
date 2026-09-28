
## Emenda proposta — bounded self-test DNS e supervisão do host

**Status desta emenda: PROPOSTA; G1 independente pendente e decisão do owner necessária antes de implementação/TDD.** Ela acrescenta requisitos ao self-test T-W0-06 sem editar nem reinterpretar a matriz de validação já auditada. Os critérios abaixo não declaram implementação, aprovação ou execução.

### Probe DNS e resultado observável

Substituir a consulta via libc resolver por um datagrama UDP DNS mínimo enviado diretamente a um endereço reservado TEST-NET, `192.0.2.53:53`, sem chamar `getaddrinfo`, `gethostbyname` ou outro resolver. O probe deve correlacionar, pelo mesmo run-id, exatamente uma tentativa `sendto` destinada ao endpoint TEST-NET/53 e o resultado observado. O perfil de plataforma deve declarar previamente qual errno de bloqueio espera: `ENETUNREACH` ou `EPERM`; aceitar ambos genericamente sem essa escolha explícita não prova a política. A tentativa só passa se a syscall for observada e retornar `-1` com o errno esperado para o perfil. Syscall aceita, resultado ambíguo/inesperado, timeout, NXDOMAIN ou qualquer resposta DNS, ausência de trace, trace truncado ou tentativa não correlacionada falham fechado. NXDOMAIN e timeout nunca contam como prova de bloqueio de egress. O audit deve validar completude do trace e emitir somente o resultado normalizado da tentativa, sem imprimir o trace bruto.

### Supervisor do host, deadline e cleanup

Um supervisor no host com a mesma semântica em macOS e Linux mede prazo monotônico total de no máximo **30 segundos** para o self-test, incluindo preflight do container e probe. O supervisor atribui run-id aleatório, aplica label dedicado contendo esse run-id e registra o container ID por `--cidfile` dentro do diretório privado da execução. Em timeout ou falha, interrompe apenas o processo Docker que iniciou; só encerra/remove um container quando o ID completo recuperado pelo cidfile foi inspecionado e sua label corresponde exatamente ao run-id desta execução. ID ausente, incompleto, divergente ou pertencente a outro run impede cleanup por nome/filtro amplo e produz falha. Nenhum comando de cleanup pode selecionar containers alheios por imagem, prefixo ou label parcial.

### Evidência de falha e privacidade

Antes do primeiro artefato, o host cria diretório de evidência exclusivo com modo `0700` e arquivos com modo `0600`. Timeout/falha preserva nesse diretório um relatório sanitizado contendo run-id, container ID quando conhecido, fase atingida, duração, exit/timeout, completude do trace e resumo normalizado syscall/destino reservado/errno. O relatório não inclui payload DNS, trace bruto, ambiente, URL, credencial ou conteúdo de logs. Não remover evidência de timeout/falha pelo trap normal; o cleanup de container continua limitado ao ID próprio. Nesses caminhos o terminal informa somente a falha e o caminho privado da evidência; nunca imprime trace bruto nem mensagem de sucesso. Só um resultado integralmente válido pode produzir resumo de sucesso sanitizado.

### Fixtures comportamentais e riscos

Fixtures locais com supervisor, Docker e processo de probe fakes devem verificar: errno esperado por perfil passa; syscall aceita, errno diferente, resposta/NXDOMAIN, tentativa ausente ou não correlacionada, trace ausente/truncado falham; um comando/container fake que excede deadline retorna dentro do limite, preserva relatório sanitizado e limpa somente o container cujo ID e run-id coincidem. Um segundo container fake com ID/label diferente deve permanecer intacto. Nenhuma fixture usa rede real, Docker real, Cargo, banco ou exchange.

Riscos restantes incluem diferenças de errno entre kernels e impossibilidade de limpar um container quando o daemon não retorna ID verificável; ambos devem permanecer fail-closed e não podem prolongar o prazo total. A aceitação de errno fica vinculada ao perfil de plataforma fixado, sem fallback. Rollback remove esta emenda proposta e mantém self-test indisponível até novo desenho aprovado; não restaura uma resolução DNS sem deadline nem autoriza ampliar egress. G1 independente e decisão do owner são gates prévios a qualquer código ou TDD.