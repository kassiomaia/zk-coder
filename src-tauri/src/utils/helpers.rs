//! Helper utility functions.

use uuid::Uuid;

/// Generate a new unique identifier.
pub fn generate_id() -> String {
    Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_id_returns_valid_uuid() {
        let id = generate_id();
        assert!(Uuid::parse_str(&id).is_ok());
    }

    #[test]
    fn test_generate_id_returns_unique_ids() {
        let id1 = generate_id();
        let id2 = generate_id();
        assert_ne!(id1, id2);
    }
}
