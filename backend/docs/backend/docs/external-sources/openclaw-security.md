---
title: OpenClaw — Security
description: Modelo de confiança, padrões seguros e orientação oficial de endurecimento
type: source
source_url: https://docs.openclaw.ai/gateway/security
media_type: text/html
date_fetched: 2026-09-27
preservation: text-extracted
tags:
  - source
  - immutable
  - layer-ingest
  - text
  - openclaw
  - security
---
# Security

OpenClaw documents conservative defaults: a regular host install binds the Gateway to loopback; unknown direct-message senders commonly receive a pairing code; group access is allowlisted, often with a mention gate. Container images are an exception and may expose a bind, which should be paired with authentication.

> One trust boundary per gateway. This guidance assumes one trusted boundary per gateway: a single operator or a mutually trusting team. OpenClaw says it is not a hostile multi-tenant security boundary for mutually adversarial users sharing one agent or gateway. For mixed-trust or adversarial-user operation, it recommends separate gateways and credentials, ideally separate OS users or hosts.

Agents with message-tool access can send across conversations and channel providers by default. The documentation links to restrictions for deployments that need messaging confined to a provider or conversation.

The security guide links to detailed guidance on trust model, security audit, access control, prompt injection, tool and agent permissions, sandboxing, browser risks, network exposure, secrets/storage/logs, incident response, and dependency locking. It also documents `openclaw security audit` as a command for checking security posture.
