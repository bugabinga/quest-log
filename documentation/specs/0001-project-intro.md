---
author: Oliver Jan Krylow <oliver@bugabinga.net>
date: 2026-03-19
status: done
tags: [intro]
---

# Quest Log — Project Introduction

## Overview

Quest Log is a gamified task tracker for children. Parents assign daily "quests"
(tasks), and children earn EXP (experience points) by completing them. A weekly
EXP goal unlocks rewards, creating a light RPG-style motivation loop.

The application serves a single household in single-user mode. Child view has no
authentication. Parent/editor settings at `/editor` are protected by a password.

## Target Audience

- **Primary**: Children aged ~6–12 who benefit from game-like task motivation
- **Secondary**: Parents who configure quests, set weekly goals, and define
  rewards

## Core Loop

1. Parent configures quests per weekday (title, description, image, EXP value)
2. Child views today's quests
3. Child marks quests complete → earns EXP
4. Weekly EXP goal tracked toward reward threshold
5. Rewards claimed when weekly goal met
6. History of completed quests visible

## Technology Choices

| Layer               | Technology      | Rationale                                |
| ------------------- | --------------- | ---------------------------------------- |
| Language            | Rust (2024 ed.) | Memory safety, single-binary output      |
| Web server          | Axum            | Async, minimal overhead, Tower ecosystem |
| Database            | SQLite          | Zero-config, embedded, single-file       |
| SQL toolkit         | sqlx            | Compile-time query verification          |
| HTML templates      | Maud            | Compile-time HTML validation             |
| Frontend reactivity | Datastar        | Server-driven UI via SSE, minimal JS     |
| Asset serving       | Embedded        | Single-binary deployment                 |

## Design Language

**Theme**: Dark fantasy RPG / medieval tavern bulletin board.

**Colors**: Three base hues define the entire palette. Every color in the UI is
derived from these three via oklch. Never hardcode colors directly.

**Typography**: One cohesive typeface throughout. Distinct weight for headings
vs body. Readable for young readers.

**Motion**: Responsive feedback that celebrates progress. Rewarding without
being distracting.

**Iconography**: Simple geometric SVGs for navigation. Emoji for personality and
content accents.

**Aesthetic intent**: Fun, slightly dark, occasionally sarcastic. The app speaks
to the child like a Dungeon Master narrating their adventure — playful roasts
for missed quests, genuine celebration for completions.

## Open Source

- License: MIT
- Repository: <https://github.com/bugabinga/quest-log>
- Contributions welcome via PRs

## Goals

- Single-binary deployment: one executable, no external runtime dependencies
- Provide clear, satisfying progress feedback through XP accumulation and reward
  claims
- Make task completion feel like an achievement, not a chore

## Non-Goals

- No child accounts or data collection
- No ads or monetization
- No complex client-side state

## Specs

This project uses specs to encode human intent persistently. Start with
[[0000-spec-zero]] to understand the spec system.
