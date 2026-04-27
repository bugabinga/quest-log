# Quest Log Screenshots - Manual Test Prompt

Start the Quest Log server and capture high-quality screenshots of key UI
moments. These will be used in a presenterm presentation pitching Quest Log.

## Prerequisites

1. Start the server: `cargo x serve`
2. Open browser to http://localhost:3000

## Screenshots Needed

Capture these specific moments (full browser window, not just element):

### 1. Main Quest Log (empty state or with quests)

- URL: http://localhost:3000
- Show the daily quest interface with quest cards
- Ideally shows some XP progress

### 2. Quest Check Animation

- Show a quest being checked off (or just checked state)
- The green glow/completion state is important

### 3. Bounty Board

- URL: http://localhost:3000/bounty
- Show the weekly rewards interface
- The brass/gold styling is key here

### 4. XP Progress / Stats

- Any view showing XP numbers, progress bars, or weekly goals
- Could be on main page or highscore

### 5. Highscore Page (if interesting)

- URL: http://localhost:3000/highscore
- Historical achievements view

### 6. Editor/Parent UI (optional)

- URL: http://localhost:3000/editor (may need auth)
- Show how parents configure quests

## Screenshot Guidelines

- Use browser in a reasonable window size (not fullscreen, not tiny)
- Capture the full UI, not cropped
- Dark mode (default)
- PNG format preferred
- Save to: `documentation/presentation/screenshots/`

## After Capture

List all screenshots taken with their filenames and a brief description of what
each shows.

If the server fails to start or pages don't load, report the error and stop.
