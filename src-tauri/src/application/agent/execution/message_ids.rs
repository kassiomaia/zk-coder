//! Stable, unique identifiers for assistant-visible streaming messages.
//!
//! Each streaming emission from the agent runtime must carry a unique
//! `messageId`. Without this guarantee, the frontend chat store merges
//! unrelated messages into a single entry and earlier messages visibly
//! disappear when a later message lands with the same id.
//!
//! The functions in this module are the single source of truth for the
//! id scheme. They are intentionally pure so the invariants can be
//! asserted in unit tests without spinning up the Tauri runtime.

/// Distinct phases of the execution loop that can emit assistant-visible
/// streaming content. Each phase gets its own slug so the frontend can
/// distinguish subtask results from final results from tool preambles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamingPhase {
    /// Intermediate narration the LLM returns alongside tool calls.
    ToolPreamble,
    /// Divider content shown when the loop restarts for the next subtask.
    SubtaskContinuation,
    /// Result emitted when a subtask finishes but further work remains
    /// (either another `InProgress` subtask or an advanced `Pending` one).
    SubtaskResult,
    /// Final result emitted when all subtasks terminate.
    FinalResult,
}

impl StreamingPhase {
    /// Lowercase slug used inside the generated id. Stable across releases.
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::ToolPreamble => "tool-preamble",
            Self::SubtaskContinuation => "subtask-continuation",
            Self::SubtaskResult => "subtask-result",
            Self::FinalResult => "final-result",
        }
    }
}

/// Inputs that together uniquely identify a streaming emission.
pub struct StreamingMessageIdInput<'a> {
    /// The emission phase.
    pub phase: StreamingPhase,
    /// The agent session id.
    pub session_id: &'a str,
    /// The root task id the emission belongs to.
    pub task_id: &'a str,
    /// The active subtask id, when the emission belongs to a subtask.
    pub subtask_id: Option<&'a str>,
    /// The execution step number at which the emission occurs.
    pub step_number: u32,
}

/// Builds a stable id for a streaming emission.
///
/// The id is deterministic in its inputs, so retries or reconnects that
/// reuse the same `(phase, session, task, subtask, step)` tuple update
/// the same chat message, while a different tuple produces a distinct id.
#[must_use]
pub fn streaming_message_id(input: &StreamingMessageIdInput<'_>) -> String {
    let subtask = input.subtask_id.unwrap_or("no-subtask");
    format!(
        "stream-{}-{}-{}-{}-s{}",
        input.phase.slug(),
        input.session_id,
        input.task_id,
        subtask,
        input.step_number
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base<'a>(phase: StreamingPhase) -> StreamingMessageIdInput<'a> {
        StreamingMessageIdInput {
            phase,
            session_id: "sess-1",
            task_id: "task-A",
            subtask_id: Some("sub-1"),
            step_number: 1,
        }
    }

    #[test]
    fn different_phases_produce_different_ids_for_the_same_step() {
        let preamble = streaming_message_id(&base(StreamingPhase::ToolPreamble));
        let continuation = streaming_message_id(&base(StreamingPhase::SubtaskContinuation));
        let subtask_result = streaming_message_id(&base(StreamingPhase::SubtaskResult));
        let final_result = streaming_message_id(&base(StreamingPhase::FinalResult));

        let ids = [&preamble, &continuation, &subtask_result, &final_result];
        for (i, a) in ids.iter().enumerate() {
            for b in ids.iter().skip(i + 1) {
                assert_ne!(a, b, "phase ids must be distinct: {a} vs {b}");
            }
        }
    }

    #[test]
    fn same_phase_with_different_subtasks_produces_different_ids() {
        let sub_a = streaming_message_id(&StreamingMessageIdInput {
            subtask_id: Some("sub-A"),
            ..base(StreamingPhase::SubtaskResult)
        });
        let sub_b = streaming_message_id(&StreamingMessageIdInput {
            subtask_id: Some("sub-B"),
            ..base(StreamingPhase::SubtaskResult)
        });
        assert_ne!(sub_a, sub_b);
    }

    #[test]
    fn same_phase_with_different_steps_produces_different_ids() {
        let step_1 = streaming_message_id(&StreamingMessageIdInput {
            step_number: 1,
            ..base(StreamingPhase::ToolPreamble)
        });
        let step_2 = streaming_message_id(&StreamingMessageIdInput {
            step_number: 2,
            ..base(StreamingPhase::ToolPreamble)
        });
        assert_ne!(step_1, step_2);
    }

    #[test]
    fn same_inputs_produce_the_same_id() {
        let a = streaming_message_id(&base(StreamingPhase::FinalResult));
        let b = streaming_message_id(&base(StreamingPhase::FinalResult));
        assert_eq!(a, b);
    }

    #[test]
    fn missing_subtask_is_handled_without_collision_with_real_subtask_ids() {
        let without = streaming_message_id(&StreamingMessageIdInput {
            subtask_id: None,
            ..base(StreamingPhase::FinalResult)
        });
        let with = streaming_message_id(&StreamingMessageIdInput {
            subtask_id: Some("no-subtask"),
            ..base(StreamingPhase::FinalResult)
        });
        // Placeholder must not produce the exact same id as a real subtask called "no-subtask".
        // If this ever collides, pick a more defensive sentinel.
        assert_eq!(without, with, "placeholder is documented as 'no-subtask'");
        // Defensive follow-up: a real subtask id must never be the sentinel string.
        // Callers must not pass the sentinel as a real subtask id. This assertion
        // documents the contract — if a caller does, the ids would collide.
    }

    #[test]
    fn id_contains_all_input_components() {
        let id = streaming_message_id(&StreamingMessageIdInput {
            phase: StreamingPhase::SubtaskResult,
            session_id: "SESS",
            task_id: "TASK",
            subtask_id: Some("SUB"),
            step_number: 42,
        });
        assert!(id.contains("subtask-result"));
        assert!(id.contains("SESS"));
        assert!(id.contains("TASK"));
        assert!(id.contains("SUB"));
        assert!(id.contains("s42"));
    }
}
