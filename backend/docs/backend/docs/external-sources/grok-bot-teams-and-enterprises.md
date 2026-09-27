---
title: Grok Bot — teams and enterprises
description: Controles oficiais de identidade, isolamento, política, aprovação e auditoria
type: source
source_url: https://docs.x.ai/grok-bot/teams-and-enterprises
media_type: text/html
date_fetched: 2026-09-27
preservation: text-extracted
tags:
  - source
  - immutable
  - layer-ingest
  - text
  - grok-bot
  - enterprise
---
# Grok Bot for teams and enterprises

Grok Bot gives team members standing Bots for everyday work. The documentation describes one persistent Firecracker microVM per user. All Bots for that user share the computer; Bot personas/workspaces do not isolate compute. Team rules, connectors, approvals, and administrator controls govern use.

## Security model

- Per-user isolation: each user's work runs in a dedicated Firecracker microVM.
- No access by default: a Bot can use only accounts and plugins the user or team grants.
- Human approval gates: sensitive actions require user approval, evaluated by Auto Review.
- Administrative control: admins configure team rules, delegation, template sharing, and execution limits.

The page distinguishes team and enterprise features. Enterprise controls include organization enablement, network controls, enforced Auto Review, action recording, audit logs, and OpenTelemetry export. It describes SCIM provisioning/deprovisioning through the identity provider. Disabling access blocks members without deleting computers.

The documentation says Team Rules guide Bots, while Auto Review rules govern approval behavior. It recommends least privilege for connectors, keeping external email/payments/legal terms behind approval, and disabling local execution unless needed.
