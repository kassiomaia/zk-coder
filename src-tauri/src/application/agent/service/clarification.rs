//! Reply delivery + restart rehydration for pending clarifications
//! (agent-clarification-resume plan, Phases 5–6).
//!
//! Implements [`AgentServiceImpl::reply_to_clarification`] — the
//! single application-level entry for replying to a paused session —
//! and [`AgentServiceImpl::rehydrate_paused_clarifications`], called
//! at startup to surface paused sessions in the UI.

use chrono::{Duration as ChronoDuration, Utc};
use log::{info, warn};
use tauri::{AppHandle, Emitter};

use crate::application::agent::clarification::{
    clear_pending_clarification_persistent, configured_timeout_minutes, deliver_reply,
    ClarificationError, ReplyDelivery,
};
use crate::core::agent::clarification::PendingClarificationRecord;
use crate::core::agent::errors::AgentDomainError;
use crate::core::agent::repositories::RepositoryResult;
use crate::core::agent::session::{PauseReason, SessionId, SessionState};
use crate::core::chat::entities::ChatMessage as ChatTranscriptMessage;
use crate::infrastructure::agent::events::{AgentEvent, AGENT_EVENT_NAME};

use super::AgentServiceImpl;

impl AgentServiceImpl {
    /// Delivers a user reply to a pending clarification.
    ///
    /// Persists the reply to the chat transcript, attempts the live
    /// delivery, and emits a `ClarificationReplied` event. On the
    /// post-restart path (Phase 6) the method rebuilds the coordinator
    /// from the checkpoint; until Phase 6 lands, it repairs the
    /// session to `Error { recoverable: true }` so the user is not
    /// stuck on a silent failure.
    pub async fn reply_to_clarification(
        &self,
        session_id: &str,
        content: &str,
        app_handle: AppHandle,
    ) -> RepositoryResult<()> {
        // Persist the reply into the chat transcript so the user sees
        // it inline with prior messages, regardless of which delivery
        // path fires below.
        let session = self
            .session_repo
            .get_session(&SessionId(session_id.to_string()))?;
        let transcript_message = ChatTranscriptMessage::user(&session.conversation_id, content)
            .with_thread(&session.thread_id);
        if let Err(e) = self.runtime.chat_service().add_message(&transcript_message) {
            warn!(
                "[AgentService] reply_to_clarification: failed to persist transcript message: {e}"
            );
        }

        let outcome = deliver_reply(session_id, content).map_err(map_clarification_error);

        match outcome {
            Ok(ReplyDelivery::Live) => {
                info!(
                    "[AgentService] reply_to_clarification: delivered live for session {session_id}"
                );
                emit_clarification_replied(&app_handle, session_id, content, None);
                Ok(())
            }
            Ok(ReplyDelivery::PostRestart {
                record,
                checkpoint_present: _,
            }) => {
                // `deliver_reply` in Phase 2 cannot populate
                // `checkpoint_present` (it has no access to the
                // checkpoint repo). We re-read the flag here.
                let checkpoint_present = self
                    .runtime
                    .execution_checkpoint_repo()
                    .checkpoint_present(session_id)
                    .unwrap_or(false);
                emit_clarification_replied(
                    &app_handle,
                    session_id,
                    content,
                    record.subtask_id.as_deref(),
                );
                if checkpoint_present {
                    self.resume_from_checkpoint(session_id, content, &app_handle)
                } else {
                    // Orphan pending clarification with no checkpoint —
                    // the session's pause state is corrupt. Clean up
                    // and degrade to a recoverable error.
                    let _ = clear_pending_clarification_persistent(session_id);
                    self.repair_session_to_recoverable_error(
                        session_id,
                        "Clarification state corrupted; please retry",
                    )?;
                    Err(AgentDomainError::InvalidInput {
                        reason: "Clarification state corrupted; please retry".to_string(),
                    })
                }
            }
            Err(err) => Err(err),
        }
    }

    fn repair_session_to_recoverable_error(
        &self,
        session_id: &str,
        message: &str,
    ) -> RepositoryResult<()> {
        let session_id = SessionId(session_id.to_string());
        let state = SessionState::Error {
            message: message.to_string(),
            recoverable: true,
        };
        self.session_repo.update_state(&session_id, &state)
    }

    /// Surfaces persisted pending clarifications on startup
    /// (agent-clarification-resume plan, Phase 6).
    ///
    /// For each row in `agent_pending_clarifications`, this method:
    ///
    /// 1. Loads the matching `ExecutionCheckpoint`. If missing, the
    ///    pause state is orphaned — we repair the session to
    ///    `Error { recoverable: true }`, clear the clarification row,
    ///    and continue to the next record.
    /// 2. If the clarification is already past its timeout, fires the
    ///    timeout path (fail active subtask, transition `Paused`,
    ///    clear both rows, emit `ClarificationTimedOut`).
    /// 3. Otherwise emits `ClarificationRestored` so the frontend can
    ///    render the question with its reply affordance. The
    ///    coordinator is **not** rebuilt here — we wait for a reply
    ///    via [`AgentServiceImpl::reply_to_clarification`] and let
    ///    [`AgentServiceImpl::resume_from_checkpoint`] spawn the
    ///    runtime at that point.
    pub fn rehydrate_paused_clarifications(&self, app_handle: &AppHandle) -> RepositoryResult<()> {
        let pending = match crate::application::agent::clarification::list_pending_clarifications()
        {
            Ok(list) => list,
            Err(err) => {
                warn!(
                    "[AgentService] rehydrate_paused_clarifications: failed to list pending: {err}"
                );
                return Ok(());
            }
        };

        let timeout_minutes = configured_timeout_minutes();
        for record in pending {
            if let Err(err) = self.rehydrate_one(&record, timeout_minutes, app_handle) {
                warn!(
                    "[AgentService] rehydrate_paused_clarifications: failed for session {}: {err}",
                    record.session_id
                );
            }
        }
        Ok(())
    }

    fn rehydrate_one(
        &self,
        record: &PendingClarificationRecord,
        timeout_minutes: i64,
        app_handle: &AppHandle,
    ) -> RepositoryResult<()> {
        let checkpoint_repo = self.runtime.execution_checkpoint_repo();
        let checkpoint = checkpoint_repo
            .load_checkpoint(&record.session_id)
            .unwrap_or_else(|err| {
                warn!(
                    "[AgentService] rehydrate_one: failed to load checkpoint for session {}: {err}",
                    record.session_id
                );
                None
            });

        if checkpoint.is_none() {
            // Orphan clarification — repair the session.
            let _ =
                crate::application::agent::clarification::clear_pending_clarification_persistent(
                    &record.session_id,
                );
            self.repair_session_to_recoverable_error(
                &record.session_id,
                "Pause state incomplete; please retry",
            )?;
            return Ok(());
        }

        if is_timed_out(record, timeout_minutes) {
            // Fire the timeout path on load.
            let _ = clear_pending_clarification_persistent(&record.session_id);
            let _ = checkpoint_repo.clear_checkpoint(&record.session_id);
            // Transition to Paused so the UI surfaces a retry rather
            // than leaving the session stuck in AwaitingClarification.
            let _ = self.session_repo.update_state(
                &SessionId(record.session_id.clone()),
                &SessionState::Paused {
                    paused_at: Utc::now(),
                    pause_reason: PauseReason::ClarificationTimeout,
                    message: "Clarification timed out".to_string(),
                    current_subtask_id: record.subtask_id.clone(),
                },
            );
            if let Some(subtask_id) = &record.subtask_id {
                let _ = self.task_repo.update_subtask_status(
                    subtask_id,
                    crate::core::agent::task::TaskStatus::Failed,
                );
            }
            let _ = app_handle.emit(
                AGENT_EVENT_NAME,
                AgentEvent::ClarificationTimedOut {
                    session_id: record.session_id.clone(),
                    subtask_id: record.subtask_id.clone(),
                    prompt: record.prompt.clone(),
                },
            );
            return Ok(());
        }

        // Fresh pending — surface to the UI.
        let _ = app_handle.emit(
            AGENT_EVENT_NAME,
            AgentEvent::ClarificationRestored {
                session_id: record.session_id.clone(),
                prompt: record.prompt.clone(),
                asked_at: record.asked_at,
                subtask_id: record.subtask_id.clone(),
            },
        );
        info!(
            "[AgentService] Restored pending clarification for session {}",
            record.session_id
        );
        Ok(())
    }

    /// Post-restart resume path for a clarification reply
    /// (agent-clarification-resume plan, Phase 6).
    ///
    /// Loads the persisted `ExecutionCheckpoint`, rebuilds the minimal
    /// session state, appends the reply to the transcript, and
    /// transitions the session back to `Running`. Spawning a fresh
    /// coordinator seeded from `ExecutionContext::from_checkpoint`
    /// would require re-threading the full `AgentRuntime::execute`
    /// bootstrap around a prebuilt context — that coordinator-entry
    /// refactor is tracked as a follow-up; until it lands, this
    /// method clears the pause and reports the session as needing a
    /// manual retry so the user is never silently stuck.
    pub fn resume_from_checkpoint(
        &self,
        session_id: &str,
        reply_content: &str,
        app_handle: &AppHandle,
    ) -> RepositoryResult<()> {
        let checkpoint = self
            .runtime
            .execution_checkpoint_repo()
            .load_checkpoint(session_id)?;
        if checkpoint.is_none() {
            self.repair_session_to_recoverable_error(
                session_id,
                "Clarification state corrupted; please retry",
            )?;
            return Err(AgentDomainError::InvalidInput {
                reason: "No checkpoint available for resume".to_string(),
            });
        }

        let session = self
            .session_repo
            .get_session(&SessionId(session_id.to_string()))?;
        let transcript_message =
            ChatTranscriptMessage::user(&session.conversation_id, reply_content)
                .with_thread(&session.thread_id);
        if let Err(e) = self.runtime.chat_service().add_message(&transcript_message) {
            warn!("[AgentService] resume_from_checkpoint: failed to persist transcript: {e}");
        }

        let _ = clear_pending_clarification_persistent(session_id);
        let _ = self
            .runtime
            .execution_checkpoint_repo()
            .clear_checkpoint(session_id);
        self.repair_session_to_recoverable_error(
            session_id,
            "Session reply recorded; use Retry to continue the task",
        )?;

        let _ = app_handle.emit(
            AGENT_EVENT_NAME,
            AgentEvent::ClarificationReplied {
                session_id: session_id.to_string(),
                content: reply_content.to_string(),
                subtask_id: None,
            },
        );
        Ok(())
    }
}

fn is_timed_out(record: &PendingClarificationRecord, timeout_minutes: i64) -> bool {
    if timeout_minutes <= 0 {
        return false;
    }
    let elapsed = Utc::now() - record.asked_at;
    elapsed >= ChronoDuration::minutes(timeout_minutes)
}

fn emit_clarification_replied(
    app_handle: &AppHandle,
    session_id: &str,
    content: &str,
    subtask_id: Option<&str>,
) {
    let _ = app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::ClarificationReplied {
            session_id: session_id.to_string(),
            content: content.to_string(),
            subtask_id: subtask_id.map(str::to_string),
        },
    );
}

fn map_clarification_error(err: ClarificationError) -> AgentDomainError {
    match err {
        ClarificationError::NotPending(id) => AgentDomainError::InvalidInput {
            reason: format!("No pending clarification for session {id}"),
        },
        ClarificationError::ReplyAlreadyDelivered(id) => AgentDomainError::InvalidInput {
            reason: format!("Clarification reply already delivered for session {id}"),
        },
        ClarificationError::StoreUninitialized => AgentDomainError::InvalidInput {
            reason: "Clarification store not initialized".to_string(),
        },
        ClarificationError::Repository(msg) => AgentDomainError::InvalidInput {
            reason: format!("Clarification repository error: {msg}"),
        },
    }
}
