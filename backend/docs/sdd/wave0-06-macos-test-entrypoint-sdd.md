---
title: SDD W0-06 — ponto de entrada seguro para testes no macOS
description: Proposta de ponto único de entrada para testes unitários locais no macOS.
tags:
  - sdd
  - backend
  - testing
  - security
  - wave0
status: proposed
---
# SDD W0-06 — ponto de entrada seguro para testes no macOS

**Status:** PROPOSTO; aprovação do seam público pelo owner pendente. Este addendum trata somente da entrada e evidência de teste unitário em host macOS, como exceção material de plataforma ao contrato de mesmo-runner descrito no SDD [T-W0-06](./wave0-15-test-side-effect-isolation-sdd.md). Para Linux, o target runner compartilhado aprovado no SDD canônico permanece sem alterações. Para macOS, propõe-se que o wrapper seja a única rota aceita de evidência; esta exceção não está aprovada pelo owner até que o seam desta proposta seja aceito. O addendum não reabre nem altera o G1 APROVADO COM FOLLOW-UP do T-W0-06.

## Contexto

O host local inspecionado é macOS ARM64 (`aarch64-apple-darwin`). A fatia de isolamento existente executa Cargo e os binários de teste dentro de uma imagem Linux pinada. Um executável Mach-O produzido por Cargo no macOS não pode ser executado diretamente dentro do container Linux. Nesta máquina não há rustup, target Rust Linux AArch64 instalado, linker Linux AArch64 ou strace. `sandbox-exec` existe, mas a política de negação de rede e sua herança por processos descendentes não foram provadas. A tentativa local de listar probes DTrace falhou por falta de privilégios; é apenas observação diagnóstica deste host, não evidência de comportamento sistêmico nem fundamento para rejeitar a alternativa nativa macOS.

O wrapper `backend/scripts/verify-test-isolation.sh` já é capaz de criar um snapshot allowlisted de arquivos backend rastreados, validar imagem e arquitetura antes de iniciar Cargo, montar a registry offline em modo somente leitura e executar `cargo test --locked --offline` dentro do container com `--network=none`, seccomp e `strace -f` mais auditoria. O container não recebe o checkout completo, `.env`, Docker socket ou configuração Cargo local. Opt-ins de integração são zerados no launcher interno.

## Seam proposto para aprovação do owner

A única rota aceita para executar e reportar a suíte unitária localmente será, a partir da raiz do repositório:

```sh
./backend/scripts/verify-test-isolation.sh
```

Filtros pontuais devem continuar usando o modo `--filter <nome-exato>` desse mesmo wrapper. Esse entrypoint constrói e executa os binários dentro da imagem Linux pinada; assim, o executável permanece no mesmo ambiente Linux em que foi compilado. Cargo offline, registry readonly, deny-egress e auditoria de descendentes se aplicam ao caminho executado. Falha de arquitetura, imagem/digest, versão de strace, registry ou isolamento aborta antes de Cargo/testes; não há fallback para host.

A chamada direta de `cargo test` no host macOS fica classificada como **não aprovada e fora da evidência de aceite**. Isso é uma exceção material ao contrato de target runner compartilhado do T-W0-06 para Linux; no macOS, somente a saída do wrapper satisfaz o aceite proposto. O projeto não afirma que a chamada direta esteja bloqueada: um desenvolvedor ainda pode invocá-la ou iniciar diretamente um binário. Resultados obtidos por essas rotas não contam como verificação segura/aceite; somente a saída e os artefatos do wrapper satisfazem o gate desta suíte. Uma verificação de CI/integração deve exigir a evidência emitida pelo wrapper.

Esta proposta reduz a escolha pública a um seam verificável: um comando suportado que compila e executa sob a mesma barreira. Não promete controle técnico sobre ferramentas arbitrárias no host.

## Opções e trade-offs

1. **Wrapper como único entrypoint aceito — recomendado.** Usa a imagem Linux e o isolamento já especificados, evita incompatibilidade Mach-O/Linux e evita depender de uma política nativa macOS ainda não demonstrada. Tem custo de preparar imagem/cache/registry e não impede que alguém rode Cargo fora do wrapper; esse uso fica fora da aceitação e deve ser identificado como bypass não confiável.
2. **Runner nativo macOS para `cargo test` — adiado, sem rejeição permanente.** Cargo permite `target.<triple>.runner` e o executa para binários de `cargo test`; esse runner envolve o executável-alvo, não todo o processo Cargo nem seus build scripts. Portanto, sozinho, não impõe a barreira durante a compilação nem durante build scripts. O Cargo também não permite usar aliases para redefinir comandos built-in, então um alias `test` não força o wrapper. Consulte [Cargo configuration: target runner](https://doc.rust-lang.org/cargo/reference/config.html#targettriple-runner) e [Cargo aliases](https://doc.rust-lang.org/cargo/reference/config.html#alias). A opção pode ser reavaliada com um desenho que cubra fase de build, binário de teste e descendentes, e com evidência de deny-network, isolamento de arquivos/ambiente e observabilidade equivalente. A observação DTrace local acima não decide essa alternativa.
3. **Cross-compile de Darwin para Linux ou emulador — adiado.** A inspeção desta máquina não encontrou target, linker nem ferramenta de cross-build instalados. Adicionar toolchain/emulação aumenta superfície e tempo de build e não é necessário para a rota recomendada, que compila dentro do container.

## Escopo, riscos e validação

Este addendum cobre somente a entrada da suíte unitária local e seus critérios de evidência. Não autoriza nem descreve runners PG, Neo4j, Binance, testes de produção ou qualquer acesso a serviços externos. Mantêm-se os gates independentes e as restrições do SDD T-W0-06.

O snapshot inclui arquivos backend rastreados no Git; arquivos novos não rastreados ficam fora dele até serem incorporados ao conjunto rastreado. O wrapper valida se os artefatos mínimos estão presentes e falha se o snapshot estiver incompleto. Ele não representa isolamento de comandos arbitrários executados pelo desenvolvedor no host; esse é o risco residual mais importante da proposta.

Após aprovação explícita do seam, a validação G3 deve primeiro provar por teste mockado que pré-requisitos ausentes/divergentes (digest, arquitetura, versão do auditor/strace e registry offline) abortam antes de iniciar Cargo; que wrapper e fluxo composto chegam ao mesmo launcher deny-egress; que `.env`/checkout e opt-ins não entram no container; e que auditoria acompanha descendentes. A verificação comportamental autorizada da suíte unitária usa apenas o comando acima, com sentinelas não secretas, sem serviços, banco ou exchange. G4 registra a saída do wrapper e confirma que falha de pré-requisito não executa testes.

## Rollout e rollback

Depois do aceite do seam, documentar o wrapper como único comando de teste aceito no README e nos gates de CI; manter falha fechada antes da compilação quando os requisitos locais não estiverem disponíveis. Se imagem, cache ou isolamento ficar indisponível, pausar a verificação e corrigir esse ambiente; não trocar silenciosamente para Cargo no host. Rollback consiste em suspender temporariamente a execução aceita até o wrapper recuperar seus pré-requisitos. A classificação de Cargo direto como fora de aceite permanece explícita durante todo o rollback.