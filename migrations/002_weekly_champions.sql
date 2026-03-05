-- Weekly Champions tracking table
-- Migration: 002_weekly_champions

CREATE TABLE weekly_champions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    week_start DATE NOT NULL UNIQUE,
    earned_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_weekly_champions_week_start ON weekly_champions(week_start);
