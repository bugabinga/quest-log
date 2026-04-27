#![allow(unused_imports, clippy::allow_attributes_without_reason)]

use chrono::{Datelike, NaiveDate, TimeDelta, Utc};
use sqlx::{Row, SqlitePool};
use std::env;
use std::path::Path;
use tracing::instrument;

use crate::models::{
    ClaimState, CreateQuestRequest, CreateRewardRequest, Quest, QuestCompletion, Reward, Settings,
    ToggleResult, UpdateQuestRequest, UpdateRewardRequest, UpdateSettingsRequest, WeeklyChampion,
    WeeklyRewardDisplay,
};

use crate::time;

/// Represents a value that can either be explicitly set or left unchanged.
#[derive(Clone, Debug, Default)]
pub enum SetOrRemove<T> {
    #[default]
    /// No change to the existing value.
    Unchanged,
    /// Explicitly set to a new value.
    Set(T),
}

impl<T> SetOrRemove<T> {
    /// Creates a new `SetOrRemove` with the given value.
    #[must_use]
    pub fn set(value: T) -> Self {
        Self::Set(value)
    }

    /// Returns the contained value if set, or `None` if unchanged.
    pub fn as_option(&self) -> Option<&T> {
        match self {
            Self::Set(value) => Some(value),
            Self::Unchanged => None,
        }
    }

    /// Returns `true` if the value is unchanged.
    pub fn is_unchanged(&self) -> bool {
        matches!(self, Self::Unchanged)
    }
}

/// Database connection wrapper for `SQLite` operations.
#[derive(Clone, Debug)]
pub struct Database {
    pool: SqlitePool,
}

mod queries;
mod queries_core;
mod queries_weekly;
#[cfg(test)]
mod tests;
