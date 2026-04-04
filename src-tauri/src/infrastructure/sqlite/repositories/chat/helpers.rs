//! Helper functions for parsing and converting data types.

use chrono::{DateTime, Utc};

use crate::core::chat::entities::MessageStatus;

/// Parses a datetime string, returning `Utc::now()` on failure.
pub fn parse_datetime(s: &str) -> DateTime<Utc> {
    chrono::DateTime::parse_from_rfc3339(s).map_or_else(|_| Utc::now(), |dt| dt.with_timezone(&Utc))
}

/// Converts status to string for storage.
pub const fn status_to_str(status: &MessageStatus) -> &'static str {
    match status {
        MessageStatus::Sending => "sending",
        MessageStatus::Sent => "sent",
        MessageStatus::Delivered => "delivered",
        MessageStatus::Error => "error",
    }
}

/// Parses status from string.
pub fn parse_status(s: &str) -> MessageStatus {
    match s {
        "sending" => MessageStatus::Sending,
        "delivered" => MessageStatus::Delivered,
        "error" => MessageStatus::Error,
        _ => MessageStatus::Sent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_datetime_valid() {
        let dt = parse_datetime("2024-01-01T12:00:00Z");
        assert_eq!(dt.timestamp(), 1704110400);
    }

    #[test]
    fn test_parse_datetime_invalid() {
        let dt = parse_datetime("invalid");
        // Should not panic, returns current time
        assert!(dt.timestamp() > 0);
    }

    #[test]
    fn test_status_to_str() {
        assert_eq!(status_to_str(&MessageStatus::Sending), "sending");
        assert_eq!(status_to_str(&MessageStatus::Sent), "sent");
        assert_eq!(status_to_str(&MessageStatus::Delivered), "delivered");
        assert_eq!(status_to_str(&MessageStatus::Error), "error");
    }

    #[test]
    fn test_parse_status() {
        assert_eq!(parse_status("sending"), MessageStatus::Sending);
        assert_eq!(parse_status("sent"), MessageStatus::Sent);
        assert_eq!(parse_status("delivered"), MessageStatus::Delivered);
        assert_eq!(parse_status("error"), MessageStatus::Error);
        assert_eq!(parse_status("unknown"), MessageStatus::Sent); // default
    }
}
