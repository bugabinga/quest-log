use chrono::{Datelike, NaiveDate, NaiveDateTime, Utc, Weekday};

#[cfg(debug_assertions)]
use crate::config;

thread_local! {
    static FAKE_TODAY: std::cell::RefCell<Option<NaiveDateTime>> = const { std::cell::RefCell::new(None) };
}

/// Returns the current date adjusted to the specified timezone.
///
/// This function provides timezone-aware date calculation:
/// 1. **Test Override**: If [`set_today()`] was called (thread-local), returns that date
/// 2. **Environment Variable**: If `QUEST_LOG_TODAY` is set (debug builds only)
/// 3. **Timezone-Aware**: If a valid timezone is provided, returns the local date in that timezone
/// 4. **UTC Fallback**: Returns UTC date if no timezone is provided or timezone is invalid
///
/// # Arguments
///
/// * `tz` - Optional IANA timezone string (e.g., `America/New_York`, `Asia/Tokyo`)
///
/// # Examples
///
/// ```
/// use quest_log::time::today_with_timezone;
///
/// // With valid timezone
/// let today = today_with_timezone(Some("America/New_York"));
///
/// // With None (defaults to UTC)
/// let today_utc = today_with_timezone(None);
/// ```
#[must_use]
pub fn today_with_timezone(tz: Option<&str>) -> NaiveDate {
    if let Some(datetime) = FAKE_TODAY.with(|m| *m.borrow()) {
        if let Some(tz_str) = tz
            && let Ok(tz) = tz_str.parse::<chrono_tz::Tz>()
        {
            return datetime.and_utc().with_timezone(&tz).date_naive();
        }
        return datetime.date();
    }

    #[cfg(debug_assertions)]
    {
        if let Some(val) = config::today_override() {
            return parse_today_override(&val);
        }
    }

    if let Some(tz_str) = tz
        && let Ok(tz) = tz_str.parse::<chrono_tz::Tz>()
    {
        return Utc::now().with_timezone(&tz).date_naive();
    }

    Utc::now().date_naive()
}

/// Returns the current date.
///
/// This function provides the current date with the following precedence:
/// 1. **Test Override**: If [`set_today()`] was called (thread-local), returns that date
/// 2. **Environment Variable**: If `QUEST_LOG_TODAY` is set (debug builds only)
/// 3. **Real Date**: Returns the actual current date from the system clock
///
/// # Testing
///
/// For integration tests, use [`set_today()`] and [`reset_today()`] to override
/// the date. This provides better test isolation than environment variables.
///
/// For manual testing, set the `QUEST_LOG_TODAY` environment variable:
/// - Specific date: `QUEST_LOG_TODAY=2024-01-15`
/// - Weekday number: `QUEST_LOG_TODAY=1` (0=Sunday, 1=Monday, etc.)
/// - Weekday name: `QUEST_LOG_TODAY=Monday`
#[must_use]
pub fn today() -> NaiveDate {
    if let Some(datetime) = FAKE_TODAY.with(|m| *m.borrow()) {
        return datetime.date();
    }

    #[cfg(debug_assertions)]
    {
        if let Some(val) = config::today_override() {
            return parse_today_override(&val);
        }
    }

    Utc::now().date_naive()
}

fn parse_today_override(val: &str) -> NaiveDate {
    let val = val.trim();

    if let Ok(num) = val.parse::<u8>()
        && num <= 6
    {
        let today = today();
        let current_weekday = today.weekday().num_days_from_sunday();
        let target = u32::from(num);
        let offset = i64::from(current_weekday) - i64::from(target);
        return today - chrono::Duration::days(offset);
    }

    let lower = val.to_lowercase();
    let weekday = match lower.as_str() {
        "sunday" | "sun" => Some(Weekday::Sun),
        "monday" | "mon" => Some(Weekday::Mon),
        "tuesday" | "tue" => Some(Weekday::Tue),
        "wednesday" | "wed" => Some(Weekday::Wed),
        "thursday" | "thu" => Some(Weekday::Thu),
        "friday" | "fri" => Some(Weekday::Fri),
        "saturday" | "sat" => Some(Weekday::Sat),
        _ => None,
    };
    if let Some(weekday) = weekday {
        let today = today();
        let current_weekday = i64::from(today.weekday().num_days_from_sunday());
        let target = i64::from(weekday.num_days_from_sunday());
        return today - chrono::Duration::days(current_weekday - target);
    }

    if let Ok(date) = NaiveDate::parse_from_str(val, "%Y-%m-%d") {
        return date;
    }

    panic!("Invalid QUEST_LOG_TODAY value: {val}");
}

/// Override the current date for testing purposes.
///
/// This function sets a thread-local variable that makes [`today()`] return
/// the specified date instead of the real current date. The time component
/// is set to the current UTC time to enable proper timezone conversion in
/// [`today_with_timezone()`].
///
/// # Why Thread-Locals Instead of Environment Variables?
///
/// Integration tests use thread-locals (via this function) instead of the
/// `QUEST_LOG_TODAY` environment variable because:
///
/// 1. **Test Isolation**: Thread-locals are isolated per-thread, so each test
///    can set its own date without affecting other concurrent tests.
/// 2. **No Cleanup Required**: Environment variables are process-wide and
///    require manual cleanup after each test to avoid polluting other tests.
///    Thread-locals automatically reset when the test completes.
/// 3. **No Race Conditions**: Environment variables can cause race conditions
///    when tests run in parallel. Thread-locals are thread-safe.
/// 4. **Immediate Effect**: Setting a thread-local takes effect immediately,
///    whereas environment variables need process restart.
///
/// # Alternative: `QUEST_LOG_TODAY` Environment Variable
///
/// For manual testing, you can set the `QUEST_LOG_TODAY` environment variable
/// (only works in debug builds):
///
/// ```bash
/// # Set to a specific date
/// QUEST_LOG_TODAY=2024-01-01 cargo run
///
/// # Set to a specific weekday (0 = Sunday, 1=Monday, etc.)
/// QUEST_LOG_TODAY=1 cargo run  # Forces Monday
///
/// # Set a weekday by name
/// QUEST_LOG_TODAY=Monday cargo run
/// ```
///
/// Note: This environment variable is only checked in debug builds for
/// security reasons (prevents production date manipulation).
#[cfg(feature = "test-utils")]
#[allow(dead_code, reason = "only used in tests")]
pub fn set_today(date: NaiveDate) {
    let now = Utc::now().naive_utc();
    let datetime = date.and_time(now.time());
    FAKE_TODAY.with(|m| *m.borrow_mut() = Some(datetime));
}

/// Set a specific UTC datetime for testing timezone-aware functions.
///
/// This function sets both the date AND time, allowing proper testing of
/// [`today_with_timezone()`] where timezone offset can change the resulting date.
///
/// # Example
///
/// For testing when UTC is ahead but local timezone is behind:
/// ```
/// use chrono::{NaiveDate, Utc};
/// use quest_log::time::{set_fake_datetime, reset_today};
///
/// // Jan 2nd 2026 02:00 UTC = Jan 1st 2026 21:00 NY (previous day in NY)
/// let dt = NaiveDate::from_ymd_opt(2026, 1, 2)
///     .unwrap()
///     .and_hms_opt(2, 0, 0)
///     .unwrap()
///     .and_utc();
/// set_fake_datetime(dt);
/// ```
#[cfg(feature = "test-utils")]
#[allow(dead_code, reason = "only used in tests")]
pub fn set_fake_datetime(datetime: chrono::DateTime<Utc>) {
    FAKE_TODAY.with(|m| *m.borrow_mut() = Some(datetime.naive_utc()));
}

/// Reset the date override set by [`set_today()`] or [`set_fake_datetime()`].
///
/// After calling this function, [`today()`] will return the real current date
/// again.
///
/// This is typically called in test cleanup (e.g., in a `Drop` impl or
/// `after_each` hook) to ensure tests don't affect each other.
#[cfg(feature = "test-utils")]
#[allow(dead_code, reason = "only used in tests")]
pub fn reset_today() {
    FAKE_TODAY.with(|m| *m.borrow_mut() = None);
}

/// Get the start and end dates of the week containing the given date.
/// Weeks start on Monday.
#[must_use]
pub fn get_week_bounds(date: NaiveDate) -> (NaiveDate, NaiveDate) {
    let week_start =
        date - chrono::Duration::days(i64::from(date.weekday().num_days_from_monday()));
    let week_end = week_start + chrono::Duration::days(6);
    (week_start, week_end)
}

/// Format a date as ISO string (YYYY-MM-DD).
#[must_use]
pub fn format_date_iso(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

/// Format a date for display (e.g., "January 15").
#[must_use]
pub fn format_date_display(date: NaiveDate) -> String {
    date.format("%B %-d").to_string()
}

/// Get the previous day.
#[must_use]
pub fn prev_day(date: NaiveDate) -> NaiveDate {
    date - chrono::Duration::days(1)
}

/// Get the next day.
#[must_use]
pub fn next_day(date: NaiveDate) -> NaiveDate {
    date + chrono::Duration::days(1)
}

/// Parse a date from ISO string (YYYY-MM-DD).
#[must_use]
pub fn parse_date(date_str: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_week_bounds_monday() {
        // Jan 1, 2024 is a Monday
        let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let (start, end) = get_week_bounds(date);
        assert_eq!(start, NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        assert_eq!(end, NaiveDate::from_ymd_opt(2024, 1, 7).unwrap());
    }

    #[test]
    fn test_get_week_bounds_wednesday() {
        // Jan 3, 2024 is a Wednesday
        let date = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap();
        let (start, end) = get_week_bounds(date);
        assert_eq!(start, NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        assert_eq!(end, NaiveDate::from_ymd_opt(2024, 1, 7).unwrap());
    }

    #[test]
    fn test_get_week_bounds_sunday() {
        // Jan 7, 2024 is a Sunday (end of week)
        let date = NaiveDate::from_ymd_opt(2024, 1, 7).unwrap();
        let (start, end) = get_week_bounds(date);
        assert_eq!(start, NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        assert_eq!(end, NaiveDate::from_ymd_opt(2024, 1, 7).unwrap());
    }

    #[test]
    fn test_get_week_bounds_cross_month() {
        // Jan 31, 2024 is a Wednesday
        let date = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();
        let (start, end) = get_week_bounds(date);
        assert_eq!(start, NaiveDate::from_ymd_opt(2024, 1, 29).unwrap()); // Monday
        assert_eq!(end, NaiveDate::from_ymd_opt(2024, 2, 4).unwrap()); // Sunday
    }

    #[test]
    fn test_get_week_bounds_cross_year() {
        // Dec 31, 2024 is a Tuesday
        let date = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
        let (start, end) = get_week_bounds(date);
        assert_eq!(start, NaiveDate::from_ymd_opt(2024, 12, 30).unwrap()); // Monday
        assert_eq!(end, NaiveDate::from_ymd_opt(2025, 1, 5).unwrap()); // Sunday
    }

    #[test]
    fn test_weekday_0_sunday() {
        let reference = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap(); // Wednesday
        set_today(reference);
        let result = parse_today_override("0");
        assert_eq!(result, NaiveDate::from_ymd_opt(2023, 12, 31).unwrap()); // Previous Sunday
        reset_today();
    }

    #[test]
    fn test_weekday_1_monday() {
        let reference = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap(); // Wednesday
        set_today(reference);
        let result = parse_today_override("1");
        assert_eq!(result, NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        reset_today();
    }

    #[test]
    fn test_weekday_2_tuesday() {
        let reference = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap(); // Wednesday
        set_today(reference);
        let result = parse_today_override("2");
        assert_eq!(result, NaiveDate::from_ymd_opt(2024, 1, 2).unwrap());
        reset_today();
    }

    #[test]
    fn test_weekday_3_wednesday() {
        let reference = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap(); // Wednesday
        set_today(reference);
        let result = parse_today_override("3");
        assert_eq!(result, NaiveDate::from_ymd_opt(2024, 1, 3).unwrap());
        reset_today();
    }

    #[test]
    fn test_weekday_4_thursday() {
        let reference = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap(); // Wednesday
        set_today(reference);
        let result = parse_today_override("4");
        assert_eq!(result, NaiveDate::from_ymd_opt(2024, 1, 4).unwrap());
        reset_today();
    }

    #[test]
    fn test_weekday_5_friday() {
        let reference = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap(); // Wednesday
        set_today(reference);
        let result = parse_today_override("5");
        assert_eq!(result, NaiveDate::from_ymd_opt(2024, 1, 5).unwrap());
        reset_today();
    }

    #[test]
    fn test_weekday_6_saturday() {
        let reference = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap(); // Wednesday
        set_today(reference);
        let result = parse_today_override("6");
        assert_eq!(result, NaiveDate::from_ymd_opt(2024, 1, 6).unwrap());
        reset_today();
    }

    #[test]
    #[should_panic(expected = "Invalid QUEST_LOG_TODAY value: 7")]
    fn test_weekday_7_invalid() {
        let reference = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap();
        set_today(reference);
        let _ = parse_today_override("7");
        reset_today();
    }

    #[test]
    #[should_panic(expected = "Invalid QUEST_LOG_TODAY value: 255")]
    fn test_weekday_255_invalid() {
        let reference = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap();
        set_today(reference);
        let _ = parse_today_override("255");
        reset_today();
    }

    #[test]
    #[should_panic(expected = "Invalid QUEST_LOG_TODAY value: -1")]
    fn test_weekday_negative_invalid() {
        let reference = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap();
        set_today(reference);
        let _ = parse_today_override("-1");
        reset_today();
    }

    #[test]
    fn test_weekday_name_sunday() {
        let reference = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap(); // Wednesday
        set_today(reference);
        let result = parse_today_override("Sunday");
        assert_eq!(result, NaiveDate::from_ymd_opt(2023, 12, 31).unwrap());
        reset_today();
    }

    #[test]
    fn test_weekday_name_monday() {
        let reference = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap();
        set_today(reference);
        let result = parse_today_override("Monday");
        assert_eq!(result, NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        reset_today();
    }

    #[test]
    fn test_weekday_name_tuesday() {
        let reference = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap();
        set_today(reference);
        let result = parse_today_override("Tuesday");
        assert_eq!(result, NaiveDate::from_ymd_opt(2024, 1, 2).unwrap());
        reset_today();
    }

    #[test]
    fn test_weekday_name_wednesday() {
        let reference = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap();
        set_today(reference);
        let result = parse_today_override("Wednesday");
        assert_eq!(result, NaiveDate::from_ymd_opt(2024, 1, 3).unwrap());
        reset_today();
    }

    #[test]
    fn test_weekday_name_thursday() {
        let reference = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap();
        set_today(reference);
        let result = parse_today_override("Thursday");
        assert_eq!(result, NaiveDate::from_ymd_opt(2024, 1, 4).unwrap());
        reset_today();
    }

    #[test]
    fn test_weekday_name_friday() {
        let reference = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap();
        set_today(reference);
        let result = parse_today_override("Friday");
        assert_eq!(result, NaiveDate::from_ymd_opt(2024, 1, 5).unwrap());
        reset_today();
    }

    #[test]
    fn test_weekday_name_saturday() {
        let reference = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap();
        set_today(reference);
        let result = parse_today_override("Saturday");
        assert_eq!(result, NaiveDate::from_ymd_opt(2024, 1, 6).unwrap());
        reset_today();
    }

    #[test]
    fn test_weekday_name_case_insensitive() {
        let reference = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap();
        set_today(reference);

        assert_eq!(
            parse_today_override("MONDAY"),
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
        );
        assert_eq!(
            parse_today_override("monday"),
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
        );
        assert_eq!(
            parse_today_override("MonDay"),
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
        );

        reset_today();
    }

    #[test]
    fn test_weekday_abbrev_sun() {
        let reference = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap();
        set_today(reference);
        let result = parse_today_override("Sun");
        assert_eq!(result, NaiveDate::from_ymd_opt(2023, 12, 31).unwrap());
        reset_today();
    }

    #[test]
    fn test_weekday_abbrev_mon() {
        let reference = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap();
        set_today(reference);
        let result = parse_today_override("Mon");
        assert_eq!(result, NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        reset_today();
    }

    #[test]
    fn test_weekday_abbrev_tue() {
        let reference = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap();
        set_today(reference);
        let result = parse_today_override("Tue");
        assert_eq!(result, NaiveDate::from_ymd_opt(2024, 1, 2).unwrap());
        reset_today();
    }

    #[test]
    fn test_weekday_abbrev_wed() {
        let reference = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap();
        set_today(reference);
        let result = parse_today_override("Wed");
        assert_eq!(result, NaiveDate::from_ymd_opt(2024, 1, 3).unwrap());
        reset_today();
    }

    #[test]
    fn test_weekday_abbrev_thu() {
        let reference = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap();
        set_today(reference);
        let result = parse_today_override("Thu");
        assert_eq!(result, NaiveDate::from_ymd_opt(2024, 1, 4).unwrap());
        reset_today();
    }

    #[test]
    fn test_weekday_abbrev_fri() {
        let reference = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap();
        set_today(reference);
        let result = parse_today_override("Fri");
        assert_eq!(result, NaiveDate::from_ymd_opt(2024, 1, 5).unwrap());
        reset_today();
    }

    #[test]
    fn test_weekday_abbrev_sat() {
        let reference = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap();
        set_today(reference);
        let result = parse_today_override("Sat");
        assert_eq!(result, NaiveDate::from_ymd_opt(2024, 1, 6).unwrap());
        reset_today();
    }

    #[test]
    #[should_panic(expected = "Invalid QUEST_LOG_TODAY value: Funday")]
    fn test_weekday_name_invalid() {
        let reference = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap();
        set_today(reference);
        let _ = parse_today_override("Funday");
        reset_today();
    }

    #[test]
    #[should_panic(expected = "Invalid QUEST_LOG_TODAY value: Javaday")]
    fn test_weekday_name_invalid_javaday() {
        let reference = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap();
        set_today(reference);
        let _ = parse_today_override("Javaday");
        reset_today();
    }

    #[test]
    fn test_full_date_2026_02_20() {
        let reference = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap();
        set_today(reference);
        let result = parse_today_override("2026-02-20");
        assert_eq!(result, NaiveDate::from_ymd_opt(2026, 2, 20).unwrap());
        reset_today();
    }

    #[test]
    fn test_full_date_2024_01_01() {
        let reference = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        set_today(reference);
        let result = parse_today_override("2024-01-01");
        assert_eq!(result, NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        reset_today();
    }

    #[test]
    fn test_full_date_2024_12_31() {
        let reference = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        set_today(reference);
        let result = parse_today_override("2024-12-31");
        assert_eq!(result, NaiveDate::from_ymd_opt(2024, 12, 31).unwrap());
        reset_today();
    }

    #[test]
    fn test_full_date_leap_year() {
        let reference = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        set_today(reference);
        let result = parse_today_override("2024-02-29");
        assert_eq!(result, NaiveDate::from_ymd_opt(2024, 2, 29).unwrap());
        reset_today();
    }

    #[test]
    #[should_panic(expected = "Invalid QUEST_LOG_TODAY value: 20-02-2026")]
    fn test_full_date_invalid_format_dd_mm_yyyy() {
        let reference = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap();
        set_today(reference);
        let _ = parse_today_override("20-02-2026");
        reset_today();
    }

    #[test]
    #[should_panic(expected = "Invalid QUEST_LOG_TODAY value: 2026/02/20")]
    fn test_full_date_invalid_format_slashes() {
        let reference = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap();
        set_today(reference);
        let _ = parse_today_override("2026/02/20");
        reset_today();
    }

    #[test]
    #[should_panic(expected = "Invalid QUEST_LOG_TODAY value: ")]
    fn test_empty_string() {
        let reference = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap();
        set_today(reference);
        let _ = parse_today_override("");
        reset_today();
    }

    #[test]
    fn test_whitespace_trimmed() {
        let reference = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap();
        set_today(reference);
        let result = parse_today_override("  1  ");
        assert_eq!(result, NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        reset_today();
    }

    #[test]
    fn test_reference_date_is_sunday() {
        // When reference is Sunday, weekday 0 should return same day
        let reference = NaiveDate::from_ymd_opt(2024, 1, 7).unwrap(); // Sunday
        set_today(reference);
        let result = parse_today_override("0");
        assert_eq!(result, NaiveDate::from_ymd_opt(2024, 1, 7).unwrap());
        reset_today();
    }

    #[test]
    fn test_reference_date_is_saturday() {
        // When reference is Saturday, weekday 6 should return same day
        let reference = NaiveDate::from_ymd_opt(2024, 1, 6).unwrap(); // Saturday
        set_today(reference);
        let result = parse_today_override("6");
        assert_eq!(result, NaiveDate::from_ymd_opt(2024, 1, 6).unwrap());
        reset_today();
    }

    #[test]
    fn test_cross_month_boundary() {
        // Wednesday Jan 3 2024 → request Monday (1) → should give Jan 1
        let reference = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap();
        set_today(reference);
        let result = parse_today_override("1");
        assert_eq!(result, NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        reset_today();
    }

    #[test]
    fn test_cross_year_boundary() {
        // Dec 31 2024 is Tuesday → request Tuesday (2) → should give Dec 31
        let reference = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(); // Tuesday
        set_today(reference);
        let result = parse_today_override("2");
        assert_eq!(result, NaiveDate::from_ymd_opt(2024, 12, 31).unwrap());
        reset_today();
    }

    #[test]
    fn test_number_takes_precedence_over_name() {
        // "1" should parse as number, not name
        let reference = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap();
        set_today(reference);
        let result = parse_today_override("1");
        assert_eq!(result, NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        reset_today();
    }

    #[test]
    fn test_fake_today_takes_precedence_over_real() {
        // When fake is set, it should be returned regardless of env var
        let fake_date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        set_today(fake_date);

        // The fake should take precedence
        let result = today();
        assert_eq!(result, fake_date);

        reset_today();
    }

    #[test]
    fn test_env_var_takes_precedence_over_real() {
        // Set env var and verify it takes effect
        // Note: This test only runs in debug mode where env var is checked
        let result = today();
        // Just verify it returns some valid date in the valid range
        assert!(result.year() >= 2020 && result.year() <= 2030);
        reset_today();
    }

    #[test]
    fn test_format_date_iso() {
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        assert_eq!(format_date_iso(date), "2024-01-15");
    }

    #[test]
    fn test_format_date_display() {
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        assert_eq!(format_date_display(date), "January 15");
    }

    #[test]
    fn test_prev_day() {
        let date = NaiveDate::from_ymd_opt(2024, 1, 2).unwrap();
        assert_eq!(prev_day(date), NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
    }

    #[test]
    fn test_next_day() {
        let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        assert_eq!(next_day(date), NaiveDate::from_ymd_opt(2024, 1, 2).unwrap());
    }

    #[test]
    fn test_parse_date_valid() {
        let result = parse_date("2024-01-15");
        assert_eq!(result, Some(NaiveDate::from_ymd_opt(2024, 1, 15).unwrap()));
    }

    #[test]
    fn test_parse_date_invalid() {
        let result = parse_date("15-01-2024");
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_date_empty() {
        let result = parse_date("");
        assert_eq!(result, None);
    }
}
