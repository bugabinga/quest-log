//! Quest Log - A gamified task tracking application for children.
//! Kids track daily tasks to earn EXP (experience points). Parents configure
//! tasks and weekly goals via a settings page.

pub mod auth;
pub mod config;
/// Database operations and connection management.
pub mod database;
pub mod extractors;
pub mod handlers;
/// Data models and DTOs for quests, rewards, and settings.
pub mod models;
/// Application state shared across handlers.
pub mod state;
/// Time utilities for date handling.
pub mod time;
pub mod ui;
