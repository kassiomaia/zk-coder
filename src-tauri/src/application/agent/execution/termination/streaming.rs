//! Streaming-message chunking and emission helpers used by the termination
//! paths.
//!
//! Extracted from `termination.rs` so the pure event builders can be unit-tested
//! independently of the orchestration code in the parent module.

use super::super::deps::ExecutionDeps;
use super::super::message_ids::{streaming_message_id, StreamingMessageIdInput, StreamingPhase};
use crate::infrastructure::agent::events::{AgentEvent, AGENT_EVENT_NAME};
use tauri::Emitter;

/// Minimum byte count before `chunk_response` splits at the next
/// whitespace boundary.
///
/// Picked together with the lack of an inter-chunk delay: the frontend
/// has its own 16 ms streaming throttle (`STREAM_THROTTLE_MS` in
/// `src-ui/src/lib/stores/agent/streaming/streamingThrottle.ts`) which
/// paces visible updates. Emitting fewer, larger events here cuts the
/// per-step Tauri/IPC + Svelte-reactivity cost roughly in proportion to
/// the chunk size increase without hurting the typing feel.
const STREAMING_CHUNK_BYTES: usize = 384;

fn chunk_response(content: &str) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();

    for ch in content.chars() {
        current.push(ch);
        if current.len() >= STREAMING_CHUNK_BYTES && ch.is_whitespace() {
            chunks.push(std::mem::take(&mut current));
        }
    }

    if !current.is_empty() {
        chunks.push(current);
    }

    if chunks.is_empty() {
        chunks.push(String::new());
    }

    chunks
}

fn build_streaming_events(content: &str, finalize_stream: bool) -> Vec<(String, bool)> {
    let chunks = chunk_response(content);
    let last_index = chunks.len().saturating_sub(1);

    chunks
        .into_iter()
        .enumerate()
        .map(|(index, chunk)| {
            let is_final = finalize_stream && index == last_index;
            (chunk, is_final)
        })
        .collect()
}

/// Pure builder for the list of `AgentEvent::StreamingContent` events produced
/// by one streamed assistant emission.
///
/// Extracted from [`emit_streaming_message`] so the payload shape —
/// specifically whether `persist_message` is set — can be unit-tested without
/// a live Tauri `AppHandle`. The `emit_*` helpers become thin loops over the
/// output of this function.
pub(super) fn build_streaming_content_events(
    session_id: &str,
    conversation_id: &str,
    thread_id: &str,
    content: &str,
    finalize_stream: bool,
    message_id: String,
    persist_message: bool,
) -> Vec<AgentEvent> {
    build_streaming_events(content, finalize_stream)
        .into_iter()
        .map(|(chunk, is_final)| AgentEvent::StreamingContent {
            session_id: session_id.to_string(),
            conversation_id: conversation_id.to_string(),
            thread_id: thread_id.to_string(),
            chunk,
            is_final,
            message_id: Some(message_id.clone()),
            persist_message: persist_message.then_some(true),
        })
        .collect()
}

/// Builds the streaming events emitted when a subtask completes while more
/// work remains on the task. Mirrors the call made by `complete_execution` so
/// the persistence contract can be exercised in unit tests without a live
/// `AppHandle`.
///
/// The events always request persistence: subtask summaries are user-visible
/// messages that must survive a reload — otherwise the chat history would show
/// only the final answer, with the intermediate per-subtask narration lost.
pub(super) fn subtask_result_streaming_events(
    session_id: &str,
    task_id: &str,
    subtask_id: Option<&str>,
    step_number: u32,
    conversation_id: &str,
    thread_id: &str,
    content: &str,
) -> Vec<AgentEvent> {
    let message_id = streaming_message_id(&StreamingMessageIdInput {
        phase: StreamingPhase::SubtaskResult,
        session_id,
        task_id,
        subtask_id,
        step_number,
    });
    build_streaming_content_events(
        session_id,
        conversation_id,
        thread_id,
        content,
        true,
        message_id,
        true,
    )
}

/// Builds the streaming events emitted when the whole task completes and the
/// agent's final answer is streamed to the user. Always requests persistence
/// so the final summary is available after a reload.
pub(super) fn final_result_streaming_events(
    session_id: &str,
    task_id: &str,
    step_number: u32,
    conversation_id: &str,
    thread_id: &str,
    content: &str,
) -> Vec<AgentEvent> {
    let message_id = streaming_message_id(&StreamingMessageIdInput {
        phase: StreamingPhase::FinalResult,
        session_id,
        task_id,
        subtask_id: None,
        step_number,
    });
    build_streaming_content_events(
        session_id,
        conversation_id,
        thread_id,
        content,
        true,
        message_id,
        true,
    )
}

pub(crate) async fn emit_streaming_message(
    deps: &ExecutionDeps<'_>,
    session_id: &str,
    conversation_id: &str,
    thread_id: &str,
    content: &str,
    finalize_stream: bool,
    message_id: String,
    persist_message: bool,
) {
    let events = build_streaming_content_events(
        session_id,
        conversation_id,
        thread_id,
        content,
        finalize_stream,
        message_id,
        persist_message,
    );
    emit_prebuilt_streaming_events(deps, events).await;
}

/// Emits an already-built list of `StreamingContent` events on the app handle.
///
/// Separated from [`build_streaming_content_events`] so the payload
/// construction remains a pure, unit-testable function. Emits events
/// back-to-back: the frontend streaming throttle
/// (`STREAM_THROTTLE_MS = 16`) already paces visible updates, and an
/// additional backend delay per chunk was measurably slowing every
/// agent response (~24 ms × chunk count) without improving the typing
/// animation beyond what the frontend throttle already provides.
pub(super) async fn emit_prebuilt_streaming_events(
    deps: &ExecutionDeps<'_>,
    events: Vec<AgentEvent>,
) {
    for event in events {
        let _ = deps.app_handle.emit(AGENT_EVENT_NAME, event);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_streaming_content_events, build_streaming_events, final_result_streaming_events,
        subtask_result_streaming_events,
    };
    use crate::application::agent::execution::message_ids::{
        streaming_message_id, StreamingMessageIdInput, StreamingPhase,
    };
    use crate::infrastructure::agent::events::AgentEvent;

    fn streaming_persist_flags(events: &[AgentEvent]) -> Vec<Option<bool>> {
        events
            .iter()
            .map(|event| match event {
                AgentEvent::StreamingContent {
                    persist_message, ..
                } => *persist_message,
                other => panic!("expected StreamingContent event, got {other:?}"),
            })
            .collect()
    }

    fn streaming_final_flags(events: &[AgentEvent]) -> Vec<bool> {
        events
            .iter()
            .map(|event| match event {
                AgentEvent::StreamingContent { is_final, .. } => *is_final,
                other => panic!("expected StreamingContent event, got {other:?}"),
            })
            .collect()
    }

    /// Regression test for the "messages replaced on the frontend" bug.
    ///
    /// When two subtasks complete in the same session, each must produce a distinct
    /// streaming message id so the frontend chat store does not overwrite the first
    /// message with the second. This asserts the id-building contract used by
    /// `complete_execution` at each of its three streaming emission points.
    #[test]
    fn subtask_result_ids_differ_between_two_subtasks_in_the_same_session() {
        let first = streaming_message_id(&StreamingMessageIdInput {
            phase: StreamingPhase::SubtaskResult,
            session_id: "sess-xyz",
            task_id: "task-root",
            subtask_id: Some("subtask-investigate"),
            step_number: 3,
        });
        let second = streaming_message_id(&StreamingMessageIdInput {
            phase: StreamingPhase::SubtaskResult,
            session_id: "sess-xyz",
            task_id: "task-root",
            subtask_id: Some("subtask-fix"),
            step_number: 9,
        });
        assert_ne!(
            first, second,
            "Two subtasks in the same session must not share a streaming message id"
        );
    }

    /// The final completion emission uses a different id than any prior subtask-result
    /// emission, so the final summary cannot overwrite the last subtask's result.
    #[test]
    fn final_result_id_differs_from_subtask_result_id() {
        let subtask_result = streaming_message_id(&StreamingMessageIdInput {
            phase: StreamingPhase::SubtaskResult,
            session_id: "sess-1",
            task_id: "task-1",
            subtask_id: Some("subtask-last"),
            step_number: 12,
        });
        let final_result = streaming_message_id(&StreamingMessageIdInput {
            phase: StreamingPhase::FinalResult,
            session_id: "sess-1",
            task_id: "task-1",
            subtask_id: None,
            step_number: 12,
        });
        assert_ne!(subtask_result, final_result);
    }

    /// Tool-preamble emissions disambiguate by subtask as well as step, so two
    /// subtasks restarting the step counter cannot collide.
    #[test]
    fn tool_preamble_ids_differ_across_subtasks_when_step_resets() {
        let first_subtask_step_1 = streaming_message_id(&StreamingMessageIdInput {
            phase: StreamingPhase::ToolPreamble,
            session_id: "sess-1",
            task_id: "task-1",
            subtask_id: Some("sub-A"),
            step_number: 1,
        });
        let second_subtask_step_1 = streaming_message_id(&StreamingMessageIdInput {
            phase: StreamingPhase::ToolPreamble,
            session_id: "sess-1",
            task_id: "task-1",
            subtask_id: Some("sub-B"),
            step_number: 1,
        });
        assert_ne!(first_subtask_step_1, second_subtask_step_1);
    }

    #[test]
    fn test_build_streaming_events_marks_last_chunk_final_when_requested() {
        let events = build_streaming_events("hello world", true);

        assert!(!events.is_empty());
        assert!(events
            .iter()
            .take(events.len().saturating_sub(1))
            .all(|(_, is_final)| !is_final));
        assert_eq!(events.last().map(|(_, is_final)| *is_final), Some(true));
    }

    /// Concatenating every chunk's text must equal the input — the
    /// frontend relies on this to reconcile streamed content back to
    /// the final assistant message. Guards against chunker regressions
    /// (e.g. a future change that drops whitespace at boundaries).
    #[test]
    fn test_chunk_response_round_trips_content_of_various_sizes() {
        for content in [
            "",
            "short",
            "exactly one line with whitespace inside it and nothing fancy",
            &"x ".repeat(400),
            &"lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. ".repeat(20),
        ] {
            let events = build_streaming_events(content, true);
            let joined: String = events.iter().map(|(c, _)| c.as_str()).collect();
            assert_eq!(joined, content, "round-trip failed for content of len {}", content.len());
        }
    }

    /// Perf guard for Workstream B.1: a >=2 KB response must emit
    /// significantly fewer events than the old 96-byte chunking did.
    /// If this ever regresses, the per-step streaming cost spikes back
    /// to where it was before the chunk-size bump.
    #[test]
    fn test_chunk_response_emits_fewer_events_than_legacy_chunking() {
        let content = "a ".repeat(1024); // 2 KiB
        let events = build_streaming_events(&content, true);
        // Legacy (96-byte) chunking would have produced ~21 events.
        // With the new 384-byte cap we expect 6 or fewer.
        assert!(
            events.len() <= 8,
            "expected <= 8 events for a 2 KiB input, got {}",
            events.len()
        );
    }

    #[test]
    fn test_build_streaming_events_keeps_tool_call_content_non_final() {
        let events = build_streaming_events("Let me inspect the project.", false);

        assert!(!events.is_empty());
        assert!(events.iter().all(|(_, is_final)| !is_final));
    }

    /// Regression for the "subtask summary vanishes on reload" bug.
    ///
    /// `complete_execution` streams a per-subtask summary via
    /// [`subtask_result_streaming_events`] whenever a subtask completes while
    /// more work remains on the task. Every emitted `StreamingContent` event
    /// must carry `persist_message: Some(true)` so the frontend write-through
    /// callback persists the summary to `coder_chat.db`; otherwise the text is
    /// dropped when the session ends and is gone on the next reload.
    #[test]
    fn subtask_result_events_request_persistence() {
        let events = subtask_result_streaming_events(
            "sess-xyz",
            "task-root",
            Some("subtask-investigate"),
            3,
            "conv-1",
            "thread-1",
            "Investigated the repo and found the offending module.",
        );

        assert!(!events.is_empty());
        assert!(
            streaming_persist_flags(&events)
                .into_iter()
                .all(|flag| flag == Some(true)),
            "every subtask-result chunk must carry persist_message: Some(true)"
        );

        let final_flags = streaming_final_flags(&events);
        assert_eq!(
            final_flags.last().copied(),
            Some(true),
            "the last chunk of a subtask-result emission must be is_final"
        );
        assert!(
            final_flags
                .iter()
                .take(final_flags.len().saturating_sub(1))
                .all(|flag| !flag),
            "only the last chunk may be is_final"
        );
    }

    /// Regression for the "final answer vanishes on reload" bug.
    ///
    /// `complete_execution` streams the final answer via
    /// [`final_result_streaming_events`]. This is the summary the user sees at
    /// the end of an agent run; it must be persisted to the chat database.
    #[test]
    fn final_result_events_request_persistence() {
        let events = final_result_streaming_events(
            "sess-xyz",
            "task-root",
            12,
            "conv-1",
            "thread-1",
            "Done. Summary of everything I did...",
        );

        assert!(!events.is_empty());
        assert!(
            streaming_persist_flags(&events)
                .into_iter()
                .all(|flag| flag == Some(true)),
            "every final-result chunk must carry persist_message: Some(true)"
        );
    }

    /// The "Continuing with next subtask..." narration-loop filler is UI-only
    /// and must NOT be persisted — otherwise reloads would be polluted with
    /// transient filler text.
    #[test]
    fn continuation_filler_events_do_not_request_persistence() {
        let continuation_id = streaming_message_id(&StreamingMessageIdInput {
            phase: StreamingPhase::SubtaskContinuation,
            session_id: "sess-xyz",
            task_id: "task-root",
            subtask_id: Some("subtask-advance"),
            step_number: 7,
        });

        let events = build_streaming_content_events(
            "sess-xyz",
            "conv-1",
            "thread-1",
            "\n\nContinuing with next subtask...\n",
            false,
            continuation_id,
            false,
        );

        assert!(!events.is_empty());
        assert!(
            streaming_persist_flags(&events)
                .into_iter()
                .all(|flag| flag.is_none()),
            "continuation filler must not carry a persist_message flag"
        );
    }
}
