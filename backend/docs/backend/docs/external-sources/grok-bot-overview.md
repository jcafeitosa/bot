---
title: Grok Bot — overview
description: Visão oficial de Bots persistentes, contexto e trabalho em segundo plano
type: source
source_url: https://docs.x.ai/grok-bot/overview
media_type: text/html
date_fetched: 2026-09-27
preservation: text-extracted
tags:
  - source
  - immutable
  - layer-ingest
  - text
  - grok-bot
---
# Grok Bot

Grok Bot gives you Bots you can keep around: AI teammates with names, jobs, and context that compounds over time. Each Bot works on a persistent cloud computer with a browser, filesystem, and terminal, so tasks finish in your real tools instead of as chat drafts.

You work with a Bot by messaging it. Give it a task, relevant context, and access to tools it needs. It takes on multi-step work across apps and websites, keeps you updated in the conversation (including drafts you approve before they are sent), and returns when something needs your approval.

## What makes Grok Bot different

- Each Bot has a persistent cloud computer with browser, filesystem, and terminal.
- Bots can coordinate in parallel, message each other, share context in group chats, and pass ownership of tasks.
- Bots can learn workflows by demonstration, save them as skills, and rerun them on a schedule.
- A named Bot keeps memory, files, browser sessions, and preferences across sessions.

## Shared computer

All Bots for one account share one cloud computer, including files, browser sessions, and app logins. The computer belongs to the account, not an individual Bot. The documentation says isolation between users is strict.

## FAQ excerpts

Work continues on the cloud computer when the laptop or app is closed. Several Bots can work at the same time; each Bot gets its own screen on the shared computer, and one computer-use task runs at a time on that screen. Memory includes stable preferences, role context, and summaries of prior work. Conversations and learned context stay separate per Bot; shared files, browser sessions, and direct handoffs move context between them. Sites may block automation, expire sessions, or require a human step; the Bot hands those steps to the user.
