# ADR-004: Bounty Board Feature - Weekly Rewards Page with Celebration Animations

## Status

Accepted

## Context

Users needed a dedicated place to view and claim their weekly rewards, separate
from the main daily quests page. The existing weekly rewards were displayed in a
collapsible section on the main page, but we wanted to create a more prominent,
themed experience that:

1. Provides a clear dedicated page for weekly rewards
2. Uses game-like theming ("Bounty Board") fitting the quest theme
3. Celebrates when users complete all weekly rewards ("Weekly Champion")
4. Prevents celebration replay on page refresh

## Decision

We implemented the Bounty Board feature with the following components:

### 1. New Route and Page

- **Route**: `GET /bounty` - New dedicated page for weekly rewards
- **Navigation**: Link added between Quests and Highscore in the navigation bar
- **Template**: `src/ui/bounty.rs` - Maud template using the base template

### 2. Database Changes

Created new table `weekly_champions` to track when users complete all weekly
rewards:

```sql
CREATE TABLE weekly_champions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    week_start DATE NOT NULL UNIQUE,
    earned_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

Database methods added:

- `get_weekly_champion(week_start)` - Check if user earned Weekly Champion
- `create_weekly_champion(week_start)` - Record Weekly Champion achievement

### 3. Celebration Animations (The "Cool Gimmick")

When all weekly rewards are claimed, the following animations play:

1. **Achievement Banner**: "🏆 Achievement Unlocked: Weekly Champion!" appears
   above the rewards
2. **Golden Pulse Animation**: All reward cards pulse with a golden glow effect
3. **Weekly Champion Badge**: A badge displays next to the rewards summary
4. **Confetti Burst**: Extra confetti particles spawn (3 bursts at 0ms, 300ms,
   600ms)

### 4. localStorage Prevention

```javascript
const storageKey = `celebrated_for_week_${weekStartStr}`;
if (localStorage.getItem(storageKey)) {
  return; // Skip celebration
}
localStorage.setItem(storageKey, "true");
```

This ensures celebration only plays once per week, not on every page refresh.

## Consequences

### Good

- Dedicated page provides clear UX for weekly rewards
- "Bounty Board" name fits the gamified quest theme
- Celebration animations provide satisfying feedback for completing all rewards
- localStorage prevents annoying replay on refresh
- Server-side rendering (HTML) for the page content
- Client-side JavaScript handles celebration animations

### Bad

- Additional route to maintain
- More CSS for celebration animations (~100 lines)
- localStorage is browser-specific (but that's acceptable for this feature)

## Considered Alternatives

### Option A: Keep Rewards on Main Page Only

- **Pros**: Simpler, no new route needed
- **Cons**: Less prominent, doesn't match the "dedicated page" requirement

### Option B: Modal Instead of Page

- **Pros**: Faster to implement, stays on same page
- **Cons**: Less memorable, harder to animate celebration

### Option C: Use localStorage for Rewards State

- **Rejected**: All state should be server-side per architecture invariants

## Implementation Details

### Files Changed

Current implementation lives under `src/`, `migrations/`, and `static/`.
Use `ARCHITECTURE.md` for current source-of-truth pointers.

### CSS Animations

- `celebration-banner-appear` - Banner slides in from top
- `celebration-shine` - Shimmer effect on banner
- `celebration-bounce` - Icon bounces
- `celebration-pulse` - Golden glow pulse on reward cards

### Navigation Position

The Bounty Board link appears in the navigation between Quests (/) and
Highscore, following the natural flow from daily activities to weekly
achievements.
