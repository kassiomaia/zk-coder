//! Durable harness state authoring for long-running agent sessions.
//!
//! Session-state transitions alone cannot tell the UI everything about a
//! long-running run: a resumed session may have already verified its
//! environment, a paused session may carry a real handoff summary, and a
//! finished session may have recorded an explicit clean-exit marker. This
//! module owns that session-facing harness metadata so execution lifecycle
//! code can record it once and every consumer (IPC, live events) sees the
//! same authoritative values.
//!
//! `derive_harness_state` still produces conservative defaults from the
//! current session and task snapshot. The store sits in front of it: any
//! override recorded here replaces the corresponding derived field. When
//! nothing has been recorded, the derived defaults stand unchanged.
//!
//! The first implementation is in-memory and lost on restart. A persistent
//! backing can be added later without changing the consumer contract.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::application::agent::read_models::{
    HarnessMode, SessionHandoffSummary, StartupVerificationSummary,
};

/// Field-level harness-state overrides merged onto the derived defaults.
///
/// Each `Some` field replaces the derived value for that field. `None`
/// leaves the derived value untouched. The store never produces partial
/// `StartupVerificationSummary` or `SessionHandoffSummary` values — when
/// one is present, it is authoritative for the whole sub-structure.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HarnessStateOverrides {
    /// Authoritative harness mode for the current session window.
    pub mode: Option<HarnessMode>,
    /// Authoritative startup verification summary, if recorded.
    pub startup_verification: Option<StartupVerificationSummary>,
    /// Authoritative handoff summary, if recorded.
    pub handoff: Option<SessionHandoffSummary>,
    /// Authoritative clean-exit marker, if recorded.
    pub clean_exit_recorded: Option<bool>,
}

impl HarnessStateOverrides {
    /// Returns `true` if every override slot is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.mode.is_none()
            && self.startup_verification.is_none()
            && self.handoff.is_none()
            && self.clean_exit_recorded.is_none()
    }
}

/// Application-layer write and read surface for durable harness state.
///
/// Execution lifecycle code records authoritative harness artifacts here.
/// Query and event code reads them to override the conservative defaults
/// produced by `derive_harness_state`.
pub trait HarnessStateStore: Send + Sync {
    /// Returns the current overrides for `session_id`, or an empty override
    /// set when nothing has been recorded.
    fn get(&self, session_id: &str) -> HarnessStateOverrides;

    /// Replaces the recorded harness mode for `session_id`.
    fn record_mode(&self, session_id: &str, mode: HarnessMode);

    /// Replaces the recorded startup verification summary for `session_id`.
    fn record_startup_verification(
        &self,
        session_id: &str,
        summary: StartupVerificationSummary,
    );

    /// Replaces the recorded handoff summary for `session_id`.
    fn record_handoff(&self, session_id: &str, summary: SessionHandoffSummary);

    /// Replaces the clean-exit marker for `session_id`.
    fn record_clean_exit(&self, session_id: &str, recorded: bool);

    /// Drops every override recorded for `session_id`.
    fn clear(&self, session_id: &str);
}

/// In-memory `HarnessStateStore` used as the default backing.
///
/// State is lost on restart. Persistence can be layered in later without
/// changing the trait.
pub struct InMemoryHarnessStateStore {
    entries: Mutex<HashMap<String, HarnessStateOverrides>>,
}

impl InMemoryHarnessStateStore {
    /// Creates an empty in-memory store.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
        }
    }

    /// Returns a shared `Arc` wrapping a fresh in-memory store.
    #[must_use]
    pub fn shared() -> Arc<dyn HarnessStateStore> {
        Arc::new(Self::new())
    }

    fn with_entry<F>(&self, session_id: &str, mutate: F)
    where
        F: FnOnce(&mut HarnessStateOverrides),
    {
        let mut entries = self
            .entries
            .lock()
            .expect("harness state store mutex poisoned");
        let entry = entries.entry(session_id.to_string()).or_default();
        mutate(entry);
    }
}

impl Default for InMemoryHarnessStateStore {
    fn default() -> Self {
        Self::new()
    }
}

impl HarnessStateStore for InMemoryHarnessStateStore {
    fn get(&self, session_id: &str) -> HarnessStateOverrides {
        self.entries
            .lock()
            .expect("harness state store mutex poisoned")
            .get(session_id)
            .cloned()
            .unwrap_or_default()
    }

    fn record_mode(&self, session_id: &str, mode: HarnessMode) {
        self.with_entry(session_id, |entry| {
            entry.mode = Some(mode);
        });
    }

    fn record_startup_verification(
        &self,
        session_id: &str,
        summary: StartupVerificationSummary,
    ) {
        self.with_entry(session_id, |entry| {
            entry.startup_verification = Some(summary);
        });
    }

    fn record_handoff(&self, session_id: &str, summary: SessionHandoffSummary) {
        self.with_entry(session_id, |entry| {
            entry.handoff = Some(summary);
        });
    }

    fn record_clean_exit(&self, session_id: &str, recorded: bool) {
        self.with_entry(session_id, |entry| {
            entry.clean_exit_recorded = Some(recorded);
        });
    }

    fn clear(&self, session_id: &str) {
        self.entries
            .lock()
            .expect("harness state store mutex poisoned")
            .remove(session_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::agent::read_models::{
        EnvironmentHealth, StartupVerificationState,
    };

    fn sample_verification() -> StartupVerificationSummary {
        StartupVerificationSummary {
            state: StartupVerificationState::Passed,
            headline: "Workspace check passed".to_string(),
            details: Some(vec!["git status clean".to_string()]),
            checked_at: Some("2026-04-17T10:00:00Z".to_string()),
        }
    }

    fn sample_handoff() -> SessionHandoffSummary {
        SessionHandoffSummary {
            last_completed_increment: "Wired harness state store".to_string(),
            current_objective: "Expose handoff in session summary".to_string(),
            next_recommended_action: "Verify panel rendering".to_string(),
            open_issues: vec!["Audit logs before ship".to_string()],
            environment_health: EnvironmentHealth::Healthy,
            recorded_at: Some("2026-04-17T10:05:00Z".to_string()),
        }
    }

    #[test]
    fn in_memory_store_returns_empty_overrides_for_unknown_session() {
        let store = InMemoryHarnessStateStore::new();
        let overrides = store.get("missing-session");
        assert!(overrides.is_empty());
    }

    #[test]
    fn in_memory_store_records_and_reads_clean_exit() {
        let store = InMemoryHarnessStateStore::new();
        store.record_clean_exit("session-a", true);
        let overrides = store.get("session-a");
        assert_eq!(overrides.clean_exit_recorded, Some(true));
    }

    #[test]
    fn in_memory_store_records_startup_verification_and_handoff_independently() {
        let store = InMemoryHarnessStateStore::new();
        store.record_startup_verification("session-a", sample_verification());
        store.record_handoff("session-a", sample_handoff());

        let overrides = store.get("session-a");
        assert_eq!(overrides.startup_verification, Some(sample_verification()));
        assert_eq!(overrides.handoff, Some(sample_handoff()));
        // Untouched fields stay empty.
        assert!(overrides.clean_exit_recorded.is_none());
        assert!(overrides.mode.is_none());
    }

    #[test]
    fn in_memory_store_later_writes_replace_earlier_writes_on_same_field() {
        let store = InMemoryHarnessStateStore::new();
        store.record_mode("session-a", HarnessMode::Initializer);
        store.record_mode("session-a", HarnessMode::Worker);
        assert_eq!(store.get("session-a").mode, Some(HarnessMode::Worker));
    }

    #[test]
    fn in_memory_store_keeps_sessions_isolated() {
        let store = InMemoryHarnessStateStore::new();
        store.record_clean_exit("session-a", true);
        store.record_clean_exit("session-b", false);
        assert_eq!(store.get("session-a").clean_exit_recorded, Some(true));
        assert_eq!(store.get("session-b").clean_exit_recorded, Some(false));
    }

    #[test]
    fn in_memory_store_clear_drops_all_fields_for_session() {
        let store = InMemoryHarnessStateStore::new();
        store.record_clean_exit("session-a", true);
        store.record_handoff("session-a", sample_handoff());
        store.clear("session-a");
        assert!(store.get("session-a").is_empty());
    }
}
