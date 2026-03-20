---
author: Oliver Jan Krylow <oliver@bugabinga.net>
date: 2026-03-19
status: done
tags: [ui, quest, core]
---

# Quest Page — Daily Adventure Interface

## Purpose

The quest page is where children encounter their daily adventures. It transforms
assigned tasks into quests with EXP rewards, making the mundane feel like an
epic journey. Every interaction should feel rewarding — completing a quest is an
achievement, not a chore.

## Goals

### Core Experience

- **Immediate feedback** — completing a quest feels instant and satisfying
- **Progress visible** — children always know how much they've done today and
  where they stand for the week
- **Adventure framing** — tasks are "quests", completion is "victory", not
  checking boxes
- **Auto-navigation** — the page flows naturally with time; at midnight it
  shifts to the new day seamlessly

### Navigation

- **Week-bound navigation** — can only navigate within the current week
  (Mon–Sun); no peeking at future or past weeks
- **Quick return** — always able to navigate back to the current day when
  viewing another date
- **Fantasy day names** — each weekday has a themed name (Day of the Sword, Day
  of the Wand, etc.) reinforcing the RPG framing

### Quest Display

- **Clear status at a glance** — past quests show whether they were completed or
  failed; today's quests are actionable; future quests tease upcoming adventures
- **Non-interactive past/future** — you can't toggle quests on days that aren't
  today (no accidental completions on wrong days)
- **Empty day handling** — if no quests are configured, display a playful
  message (parent forgot to set quests)

## Non-Goals

- **No quest editing** — quest creation and modification happen elsewhere
- **No notifications/reminders** — this page does not push or alert
- **No analytics/reporting** — parents see stats elsewhere
- **No offline support** — requires server connection
- **No multiplayer/sharing** — single-user, single-device experience

## User-Facing Behaviors

### Day Change at Midnight

When midnight passes while a child is viewing the page, the page automatically
navigates to the new day. Quests can only be completed on the current day — if
midnight passes, attempting to complete a quest from the previous day shows an
error.

### Quest Completion States

Past quests show final status (completed or missed). Today's quests are
interactive — tapping marks them complete. Future quests are visible but show
"not yet available" status.

### Stats Displayed

- **Today's EXP** — earned / available, with progress bar
- **Weekly Progress** — earned / weekly goal (from settings), with progress bar
- **Quests Completed** — done / total for today

### Empty State

When a day has no quests configured, display a playful message indicating the
parent needs to set up quests for this day.

## Constraints

- **Today-only toggling** — quests can only be marked complete on the actual
  current day
- **Week boundary** — navigation and quest display are scoped to Monday–Sunday
  of the current week

## Technical Notes

- **Timezone handling** — Server stores all times in UTC internally. The client
  sends its timezone via `X-Timezone` header (IANA timezone, e.g.
  "America/New_York"). Server uses this to determine "today" for quest filtering
  and displays dates in the client's local timezone. This ensures "today"
  matches where the user lives, regardless of server location.
- **Midnight behavior** — quests can only be toggled for the current day. If
  midnight passes while viewing, the page auto-navigates to the new day.
  Attempting to toggle a quest from a different day shows an error.
- **Weekly progress visibility** — shown when a weekly goal is configured (> 0).
  If no goal is set, this section is hidden rather than showing 0/max.
- **Toggle is reversible** — completing a quest can be undone by tapping again
  (toggle behavior)
