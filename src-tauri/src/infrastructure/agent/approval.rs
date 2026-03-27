//! Approval queue for pending agent actions.
//!
//! Manages actions that require user approval before execution.
//! Actions auto-deny after a configurable timeout (default: 5 minutes).

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, RwLock};

use crate::core::agent::session::PendingAction;

/// Timeout in minutes before a pending action is auto-denied.
const APPROVAL_TIMEOUT_MINUTES: i64 = 5;

/// Events emitted by the approval queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ApprovalEvent {
    ActionQueued { action: PendingAction },
    ActionApproved { action_id: String },
    ActionDenied { action_id: String },
    ActionTimeout { action_id: String },
}

/// Manages pending actions awaiting user approval.
///
/// Thread-safe and supports multiple subscribers for approval events.
pub struct ApprovalQueue {
    pending: Arc<RwLock<HashMap<String, PendingAction>>>,
    tx: broadcast::Sender<ApprovalEvent>,
}

impl ApprovalQueue {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel::<ApprovalEvent>(64);
        Self {
            pending: Arc::new(RwLock::new(HashMap::<String, PendingAction>::new())),
            tx,
        }
    }

    /// Add an action to the approval queue.
    pub async fn queue(&self, action: PendingAction) {
        let mut pending = self.pending.write().await;
        pending.insert(action.id.clone(), action.clone());
        let _ = self.tx.send(ApprovalEvent::ActionQueued { action });
    }

    /// Approve a pending action and return it for execution.
    pub async fn approve(&self, action_id: &str) -> Option<PendingAction> {
        let mut pending = self.pending.write().await;
        if let Some(action) = pending.remove(action_id) {
            let _ = self.tx.send(ApprovalEvent::ActionApproved {
                action_id: action_id.to_string(),
            });
            Some(action)
        } else {
            None
        }
    }

    /// Deny a pending action.
    pub async fn deny(&self, action_id: &str) -> bool {
        let mut pending = self.pending.write().await;
        let removed = pending.remove(action_id).is_some();
        if removed {
            let _ = self.tx.send(ApprovalEvent::ActionDenied {
                action_id: action_id.to_string(),
            });
        }
        removed
    }

    /// Check for timed-out actions and auto-deny them.
    ///
    /// Returns the IDs of actions that were auto-denied.
    /// Should be called periodically by a background task.
    pub async fn check_timeouts(&self) -> Vec<String> {
        let mut pending = self.pending.write().await;
        let now = Utc::now();
        let timeout = chrono::Duration::minutes(APPROVAL_TIMEOUT_MINUTES);

        let timed_out: Vec<String> = pending
            .iter()
            .filter(|(_, action)| now.signed_duration_since(action.created_at) > timeout)
            .map(|(id, _)| id.clone())
            .collect();

        for id in &timed_out {
            pending.remove(id);
            let _ = self.tx.send(ApprovalEvent::ActionTimeout {
                action_id: id.clone(),
            });
        }

        timed_out
    }

    /// Subscribe to approval events.
    pub fn subscribe(&self) -> broadcast::Receiver<ApprovalEvent> {
        self.tx.subscribe()
    }

    /// Get all currently pending actions.
    pub async fn get_pending(&self) -> Vec<PendingAction> {
        let pending = self.pending.read().await;
        pending.values().cloned().collect()
    }

    /// Get pending actions for a specific session.
    pub async fn get_pending_for_session(&self, session_id: &str) -> Vec<PendingAction> {
        let pending = self.pending.read().await;
        pending
            .values()
            .filter(|a| a.session_id.0 == session_id)
            .cloned()
            .collect()
    }

    /// Remove all pending actions for a session (e.g., on cancellation).
    pub async fn clear_session(&self, session_id: &str) {
        let mut pending = self.pending.write().await;
        let to_remove: Vec<String> = pending
            .iter()
            .filter(|(_, a)| a.session_id.0 == session_id)
            .map(|(id, _)| id.clone())
            .collect();

        for id in &to_remove {
            pending.remove(id);
            let _ = self.tx.send(ApprovalEvent::ActionDenied {
                action_id: id.clone(),
            });
        }
    }
}

impl Default for ApprovalQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::agent::session::SessionId;
    use crate::core::agent::types::ActionType;

    fn make_action(id: &str, session_id: &str) -> PendingAction {
        PendingAction {
            id: id.to_string(),
            session_id: SessionId(session_id.to_string()),
            tool_name: "fs_write_file".to_string(),
            action_type: ActionType::WriteFile {
                path: "/tmp/test".to_string(),
                size_bytes: 100,
            },
            description: "Write test file".to_string(),
            preview: None,
            arguments: serde_json::json!({}),
            created_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_queue_and_approve() {
        let queue = ApprovalQueue::new();
        let action = make_action("a1", "s1");

        queue.queue(action).await;
        assert_eq!(queue.get_pending().await.len(), 1);

        let approved = queue.approve("a1").await;
        assert!(approved.is_some());
        assert_eq!(queue.get_pending().await.len(), 0);
    }

    #[tokio::test]
    async fn test_queue_and_deny() {
        let queue = ApprovalQueue::new();
        let action = make_action("a1", "s1");

        queue.queue(action).await;
        let denied = queue.deny("a1").await;
        assert!(denied);
        assert_eq!(queue.get_pending().await.len(), 0);
    }

    #[tokio::test]
    async fn test_deny_nonexistent() {
        let queue = ApprovalQueue::new();
        let denied = queue.deny("nonexistent").await;
        assert!(!denied);
    }

    #[tokio::test]
    async fn test_clear_session() {
        let queue = ApprovalQueue::new();
        queue.queue(make_action("a1", "s1")).await;
        queue.queue(make_action("a2", "s1")).await;
        queue.queue(make_action("a3", "s2")).await;

        queue.clear_session("s1").await;

        let remaining = queue.get_pending().await;
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, "a3");
    }

    #[tokio::test]
    async fn test_get_pending_for_session() {
        let queue = ApprovalQueue::new();
        queue.queue(make_action("a1", "s1")).await;
        queue.queue(make_action("a2", "s2")).await;

        let s1_actions = queue.get_pending_for_session("s1").await;
        assert_eq!(s1_actions.len(), 1);
        assert_eq!(s1_actions[0].id, "a1");
    }
}
