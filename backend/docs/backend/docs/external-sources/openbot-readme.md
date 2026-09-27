---
title: OpenBot — README
description: README oficial do template OpenBot, arquitetura, políticas e controles
type: source
source_url: https://github.com/CopilotKit/OpenBot
media_type: text/markdown
date_fetched: 2026-09-27
preservation: text-extracted
tags:
  - source
  - immutable
  - layer-ingest
  - text
  - openbot
---
# OpenBot

OpenBot describes itself as an alpha template to clone and customize, rather than a hosted product or package. Its README says the example `.env.example` sets `OPENBOT_SINGLE_USER=true`, which admits every request as one administrator; sign-in must be configured before other users can access a deployment.

The project describes an agent platform running on customer infrastructure, with PostgreSQL data and administrator-supplied model credentials encrypted at rest. It uses AG-UI endpoints to connect agents. A gateway resolves each tool target, evaluates policy, records an audit row, and then acts or refuses. The README describes CEL policy as fail-closed: deny is evaluated before allow; a missing policy permits nothing; a broken rule refuses.

Features described include a separate container/computer per Bot, a browser and shell behind the gateway, human intervention for login/2FA, governed MCP tools, encrypted credentials, audit views for permitted/refused/failed actions, identity providers, durable threads/memory, and scheduled routines. The `/agents` interface supports coworker configuration, and endpoints are checked against browser-navigation target validation. The README states that the system is alpha and under active development.

This wrapper captures selected factual passages from the project README; see the source URL for the full text and current implementation details.
