-- Initial database schema for Quest Log application
-- Migration: 001_initial_schema

-- Quests table: Tasks that can be completed for EXP
CREATE TABLE quests (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    description TEXT,
    exp_value INTEGER NOT NULL DEFAULT 10,
    day_of_week INTEGER NOT NULL CHECK(day_of_week BETWEEN 0 AND 6), -- 0=Sunday, 6=Saturday
    image_data BLOB, -- Image stored as binary data
    image_content_type TEXT, -- e.g., 'image/png', 'image/jpeg'
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Quest completions table: Tracking when quests are completed
CREATE TABLE quest_completions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    quest_id INTEGER NOT NULL REFERENCES quests(id) ON DELETE CASCADE,
    completed_date DATE NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(quest_id, completed_date) -- Prevent duplicate completions
);

-- Rewards table: Prizes for meeting weekly EXP goals
CREATE TABLE rewards (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    description TEXT,
    required_exp INTEGER NOT NULL, -- Weekly EXP needed to claim
    image_data BLOB, -- Image stored as binary data
    image_content_type TEXT, -- e.g., 'image/png', 'image/jpeg'
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Reward claims table: Tracking claimed rewards
CREATE TABLE reward_claims (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    reward_id INTEGER NOT NULL REFERENCES rewards(id) ON DELETE CASCADE,
    claimed_date DATE NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(reward_id, claimed_date) -- One claim per reward per day
);

-- Settings table: Application configuration (single row)
CREATE TABLE settings (
    id INTEGER PRIMARY KEY CHECK(id = 1), -- Single row table
    weekly_exp_goal INTEGER NOT NULL DEFAULT 100,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Insert default settings
INSERT INTO settings (id, weekly_exp_goal) VALUES (1, 100);

-- Create indexes for better query performance
CREATE INDEX idx_quests_day_of_week ON quests(day_of_week);
CREATE INDEX idx_quests_active ON quests(is_active);
CREATE INDEX idx_quest_completions_date ON quest_completions(completed_date);
CREATE INDEX idx_quest_completions_quest_id ON quest_completions(quest_id);
CREATE INDEX idx_rewards_required_exp ON rewards(required_exp);
CREATE INDEX idx_rewards_active ON rewards(is_active);
CREATE INDEX idx_reward_claims_date ON reward_claims(claimed_date);
CREATE INDEX idx_reward_claims_reward_id ON reward_claims(reward_id);