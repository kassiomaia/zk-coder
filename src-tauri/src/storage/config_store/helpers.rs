//! Helper functions for the config store.

use chrono::{Datelike, DateTime, Timelike, Utc};

/// Parses a datetime string from the database.
pub fn parse_datetime(s: &str) -> DateTime<Utc> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_datetime() {
        let dt = parse_datetime("2024-01-15T10:30:00Z");
        // Verify the date components are correct
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);
        assert_eq!(dt.hour(), 10);
        assert_eq!(dt.minute(), 30);
    }

    #[test]
    fn test_parse_datetime_invalid() {
        // Invalid input should return current time, not panic
        let dt = parse_datetime("invalid");
        // Should not panic and should return a reasonable datetime
        assert!(dt.timestamp() > 0);
    }
}
