use crate::core::agent::errors::AgentDomainError;
use crate::core::agent::repositories::RepositoryResult;
use crate::core::agent::services::{
    ExecutionStep, FileAction, FileChange, SessionDetails, TerminalLine,
};
use crate::core::agent::session::{AgentSession, SessionState};
use crate::core::agent::task::StepAction;

use super::store_error_to_domain;
use super::AgentServiceImpl;

impl AgentServiceImpl {
    pub(super) fn get_conversation_history_impl(
        &self,
        conversation_id: &str,
    ) -> RepositoryResult<Vec<AgentSession>> {
        Ok(self
            .session_repo
            .get_sessions_for_conversation(conversation_id)?
            .into_iter()
            .filter(|session| {
                matches!(
                    session.state,
                    SessionState::Finished { .. }
                        | SessionState::Error { .. }
                        | SessionState::Cancelled { .. }
                        | SessionState::Paused { .. }
                )
            })
            .collect())
    }

    pub(super) fn get_session_details_impl(
        &self,
        session_id: &str,
    ) -> RepositoryResult<SessionDetails> {
        let sid = crate::core::agent::session::SessionId(session_id.to_string());
        let session = self
            .telemetry_repo
            .get_session(&sid)
            .map_err(store_error_to_domain)?
            .ok_or_else(|| AgentDomainError::SessionNotFound {
                id: session_id.to_string(),
            })?;

        let files_touched = self
            .telemetry_repo
            .get_session_files(&sid)
            .map(|files| {
                files
                    .into_iter()
                    .map(|file| FileChange {
                        path: file.file_path,
                        action: match file.action.as_str() {
                            "read" => FileAction::Read,
                            "write" => FileAction::Write,
                            "delete" => FileAction::Delete,
                            _ => FileAction::Read,
                        },
                        diff: file.diff,
                    })
                    .collect()
            })
            .unwrap_or_default();

        let terminal_output = self
            .telemetry_repo
            .get_session_terminal_output(&sid)
            .map(|lines| {
                lines
                    .into_iter()
                    .filter_map(|line| {
                        let timestamp = chrono::DateTime::parse_from_rfc3339(&line.timestamp)
                            .ok()
                            .map(|dt| dt.with_timezone(&chrono::Utc));

                        timestamp.map(|timestamp| TerminalLine {
                            id: line.id,
                            line_type: line.line_type,
                            content: line.content,
                            timestamp,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let execution_steps = self
            .telemetry_repo
            .get_session_execution_steps(&sid)
            .map(|steps| {
                steps
                    .into_iter()
                    .map(|step| {
                        let (step_type, description) = match &step.action {
                            StepAction::LlmRequest { model } => (
                                "llm_request".to_string(),
                                format!("LLM request using model: {model}"),
                            ),
                            StepAction::ToolCall { tool, arguments } => {
                                let arg_count = if arguments.is_object() {
                                    arguments.as_object().map_or(0, serde_json::Map::len)
                                } else if arguments.is_array() {
                                    arguments.as_array().map_or(0, std::vec::Vec::len)
                                } else {
                                    1
                                };

                                (
                                    "tool_call".to_string(),
                                    format!("Tool call: {tool} with {arg_count} arguments"),
                                )
                            }
                        };

                        ExecutionStep {
                            id: format!("step-{}", step.step_number),
                            step_type,
                            description,
                            status: if step.result.is_some() {
                                "completed".to_string()
                            } else {
                                "pending".to_string()
                            },
                            timestamp: step.timestamp,
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(SessionDetails {
            session,
            files_touched,
            terminal_output,
            execution_steps,
        })
    }
}
