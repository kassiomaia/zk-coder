//! Helper functions for agent runtime.

use crate::core::agent::task::TaskStatus;
use crate::core::agent::types::ActionType;
#[allow(unused_imports)]
use serde_json::json;

/// Derives the system prompt with optional subtask context.
pub fn derive_system_prompt(
    agent_system_prompt: &str,
    current_subtask: &Option<(super::types::SubtaskSummary, usize)>,
) -> String {
    match current_subtask {
        Some((summary, total)) => {
            if summary.status == TaskStatus::InProgress {
                format!(
                    "{}\n\n## Current Task ({}/{}): {}\n\n{}\n\nFocus on completing this task. \
                        When done, call agent_advance_to_next_subtask. \
                        If agent_plan_subtasks is called again after planning, \
                        treat it as a no-op and continue the current plan.",
                    agent_system_prompt,
                    summary.order + 1,
                    total,
                    summary.title,
                    summary.mode.instruction_text()
                )
            } else {
                agent_system_prompt.to_string()
            }
        }
        None => agent_system_prompt.to_string(),
    }
}

/// Infers the action type from tool name and arguments.
pub fn infer_action_type(tool_name: &str, arguments: &serde_json::Value) -> ActionType {
    let path = arguments
        .get("path")
        .or_else(|| arguments.get("file_path"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Extract filename for document operations
    let filename = arguments
        .get("filename")
        .or_else(|| arguments.get("title"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let document_id = arguments
        .get("document_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let deck_id = arguments
        .get("deck_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let pdf_id = arguments
        .get("pdf_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    match tool_name {
        "docx_start" => ActionType::CreateDocx { filename },
        "docx_add_section" => ActionType::MutateDocxDraft { document_id },
        "docx_list_sections" => ActionType::InspectDocxDraft { document_id },
        "docx_finalize" => ActionType::FinalizeDocx { document_id },
        "pptx_start" => ActionType::CreatePptx { filename },
        "pptx_add_slide" | "pptx_update_slide" | "pptx_delete_slide" => {
            ActionType::MutatePptxDraft { deck_id }
        }
        "pptx_list_slides" | "pptx_get_slide" | "pptx_search_slides" => {
            ActionType::InspectPptxDraft { deck_id }
        }
        "pptx_finalize" => ActionType::FinalizePptx { deck_id },
        "pdf_start" => ActionType::CreatePdf { filename },
        "pdf_add_page" | "pdf_delete_page" => ActionType::MutatePdfDraft { pdf_id },
        "pdf_list_pages" | "pdf_get_page" => ActionType::InspectPdfDraft { pdf_id },
        "pdf_finalize" => ActionType::FinalizePdf { pdf_id },
        "fs_stat_path" | "fs_glob_files" | "fs_fuzzy_find_files" => ActionType::ReadFile { path },
        "fs_replace_in_file" | "fs_apply_patch" => ActionType::WriteFile {
            path,
            size_bytes: 0,
        },
        // Standard file operations
        name if name.contains("read") => ActionType::ReadFile { path },
        name if name.contains("write") || name.contains("create") => {
            let size = arguments
                .get("content")
                .and_then(|v| v.as_str())
                .map(|s| s.len() as u64)
                .unwrap_or(0);
            ActionType::WriteFile {
                path,
                size_bytes: size,
            }
        }
        name if name.contains("delete") || name.contains("remove") => {
            ActionType::DeleteFile { path }
        }
        name if name.contains("execute") || name.contains("command") || name.contains("shell") => {
            let command = arguments
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            ActionType::ExecuteCommand { command }
        }
        name if name.contains("fetch") || name.contains("http") || name.contains("web") => {
            let url = arguments
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            ActionType::WebRequest { url }
        }
        _ => ActionType::ReadFile { path },
    }
}

/// Generates a diff from two content strings.
pub fn generate_diff_from_content(path: &str, original: &str, current: &str) -> String {
    use std::path::Path;
    if let Some(diff) = crate::infrastructure::tools::fs::write_file::generate_diff(
        Path::new(path),
        original,
        current,
    ) {
        diff
    } else {
        // Fallback: simple diff generation
        let line_count = current.lines().count();
        let lines: String = current
            .lines()
            .map(|line| format!("+{}", line))
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "--- a/{}\n+++ b/{}\n@@ -0,0 +1,{} @@\n{}",
            path, path, line_count, lines
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_action_type_read() {
        let action = infer_action_type(
            "read_workspace_file",
            &serde_json::json!({"path": "/tmp/test.rs"}),
        );
        match action {
            ActionType::ReadFile { path } => assert_eq!(path, "/tmp/test.rs"),
            _ => panic!("Expected ReadFile"),
        }
    }

    #[test]
    fn test_infer_action_type_read_variations() {
        let tools = vec![
            "read_file",
            "read_workspace_file",
            "fs_read_file",
            "fs_stat_path",
            "fs_glob_files",
            "fs_fuzzy_find_files",
        ];

        for tool in tools {
            let action = infer_action_type(tool, &serde_json::json!({"path": "/test.txt"}));
            match action {
                ActionType::ReadFile { .. } => {}
                _ => panic!("Tool '{}' should infer ReadFile", tool),
            }
        }
    }

    #[test]
    fn test_infer_action_type_write() {
        let action = infer_action_type(
            "write_file",
            &serde_json::json!({"path": "/tmp/out.rs", "content": "hello"}),
        );
        match action {
            ActionType::WriteFile { path, size_bytes } => {
                assert_eq!(path, "/tmp/out.rs");
                assert_eq!(size_bytes, 5);
            }
            _ => panic!("Expected WriteFile"),
        }
    }

    #[test]
    fn test_infer_action_type_write_variations() {
        let tools = vec![
            "write_file",
            "fs_write_file",
            "create_file",
            "fs_replace_in_file",
            "fs_apply_patch",
        ];

        for tool in tools {
            let action = infer_action_type(
                tool,
                &serde_json::json!({"path": "/test.txt", "content": "x"}),
            );
            match action {
                ActionType::WriteFile { .. } => {}
                _ => panic!("Tool '{}' should infer WriteFile", tool),
            }
        }
    }

    #[test]
    fn test_infer_action_type_write_size_calculation() {
        let content = "hello world".repeat(10); // 110 characters
        let action = infer_action_type(
            "write_file",
            &serde_json::json!({"path": "/test.txt", "content": content}),
        );

        match action {
            ActionType::WriteFile { size_bytes, .. } => {
                assert_eq!(size_bytes, 110);
            }
            _ => panic!("Expected WriteFile"),
        }
    }

    #[test]
    fn test_infer_action_type_delete() {
        let tools = vec!["delete_file", "remove_file", "fs_delete_file"];

        for tool in tools {
            let action = infer_action_type(tool, &serde_json::json!({"path": "/test.txt"}));
            match action {
                ActionType::DeleteFile { .. } => {}
                _ => panic!("Tool '{}' should infer DeleteFile", tool),
            }
        }
    }

    #[test]
    fn test_infer_action_type_execute() {
        let action = infer_action_type(
            "execute_command",
            &serde_json::json!({"command": "cargo build"}),
        );
        match action {
            ActionType::ExecuteCommand { command } => assert_eq!(command, "cargo build"),
            _ => panic!("Expected ExecuteCommand"),
        }
    }

    #[test]
    fn test_infer_action_type_execute_variations() {
        let tools = vec!["execute_command", "system_execute_command", "shell_command"];

        for tool in tools {
            let action = infer_action_type(tool, &serde_json::json!({"command": "test"}));
            match action {
                ActionType::ExecuteCommand { .. } => {}
                _ => panic!("Tool '{}' should infer ExecuteCommand", tool),
            }
        }
    }

    #[test]
    fn test_infer_action_type_web_request() {
        let tools = vec!["fetch_url", "http_request", "web_fetch"];

        for tool in tools {
            let action =
                infer_action_type(tool, &serde_json::json!({"url": "https://example.com"}));
            match action {
                ActionType::WebRequest { .. } => {}
                _ => panic!("Tool '{}' should infer WebRequest", tool),
            }
        }
    }

    #[test]
    fn test_infer_action_type_unknown_defaults_to_read() {
        let action = infer_action_type("unknown_tool", &serde_json::json!({}));
        match action {
            ActionType::ReadFile { path } => assert_eq!(path, ""),
            _ => panic!("Expected ReadFile as default"),
        }
    }

    #[test]
    fn test_infer_action_type_file_path_extraction() {
        // Test that file_path is also checked as a fallback
        let action = infer_action_type(
            "read_file",
            &serde_json::json!({"file_path": "/alternate/path.txt"}),
        );
        match action {
            ActionType::ReadFile { path } => assert_eq!(path, "/alternate/path.txt"),
            _ => panic!("Expected ReadFile with file_path"),
        }
    }

    #[test]
    fn test_infer_action_type_missing_path() {
        let action = infer_action_type("write_file", &serde_json::json!({"content": "test"}));
        match action {
            ActionType::WriteFile { path, .. } => assert_eq!(path, ""),
            _ => panic!("Expected WriteFile with empty path"),
        }
    }

    #[test]
    fn test_derive_system_prompt_without_subtask() {
        let base_prompt = "You are a helpful assistant.";
        let result = derive_system_prompt(base_prompt, &None);
        assert_eq!(result, base_prompt);
    }

    #[test]
    fn test_derive_system_prompt_with_subtask() {
        use crate::infrastructure::agent::runtime::SubtaskSummary;

        let base_prompt = "You are a helpful assistant.";
        let summary = SubtaskSummary {
            title: "Test task".to_string(),
            mode: crate::core::agent::task::SubTaskMode::Read,
            status: TaskStatus::InProgress,
            order: 0,
        };
        let result = derive_system_prompt(base_prompt, &Some((summary, 3)));
        assert!(result.contains("## Current Task (1/3): Test task"));
        assert!(result.contains("Focus on completing this task"));
    }

    #[test]
    fn test_derive_system_prompt_not_in_progress() {
        use crate::infrastructure::agent::runtime::SubtaskSummary;

        let base_prompt = "You are a helpful assistant.";
        let summary = SubtaskSummary {
            title: "Test task".to_string(),
            mode: crate::core::agent::task::SubTaskMode::Read,
            status: TaskStatus::Pending,
            order: 0,
        };
        let result = derive_system_prompt(base_prompt, &Some((summary, 3)));
        // Should not add subtask context if not in progress
        assert_eq!(result, base_prompt);
    }

    #[test]
    fn test_derive_system_prompt_with_different_modes() {
        use crate::infrastructure::agent::runtime::SubtaskSummary;

        let base_prompt = "You are a helpful assistant.";

        for mode in &[
            crate::core::agent::task::SubTaskMode::Read,
            crate::core::agent::task::SubTaskMode::Write,
            crate::core::agent::task::SubTaskMode::Execute,
            crate::core::agent::task::SubTaskMode::FullAccess,
        ] {
            let summary = SubtaskSummary {
                title: "Test task".to_string(),
                mode: mode.clone(),
                status: TaskStatus::InProgress,
                order: 0,
            };
            let result = derive_system_prompt(base_prompt, &Some((summary, 1)));
            assert!(result.contains("## Current Task (1/1): Test task"));
            assert!(result.contains(&mode.instruction_text()));
        }
    }

    #[test]
    fn test_derive_system_prompt_multiple_subtasks() {
        use crate::infrastructure::agent::runtime::SubtaskSummary;

        let base_prompt = "You are a helpful assistant.";

        // Test with 5 subtasks, current is 3rd
        let summary = SubtaskSummary {
            title: "Third task".to_string(),
            mode: crate::core::agent::task::SubTaskMode::Write,
            status: TaskStatus::InProgress,
            order: 2, // 0-indexed, so this is the 3rd subtask
        };
        let result = derive_system_prompt(base_prompt, &Some((summary, 5)));
        assert!(result.contains("## Current Task (3/5): Third task"));
    }

    #[test]
    fn test_generate_diff_from_content() {
        let original = "line1\nline2";
        let current = "line1\nline2\nline3";
        let diff = generate_diff_from_content("test.txt", original, current);
        assert!(diff.contains("test.txt"));
        assert!(diff.contains("+line3"));
    }

    #[test]
    fn test_generate_diff_from_content_empty_original() {
        let original = "";
        let current = "line1\nline2";
        let diff = generate_diff_from_content("new.txt", original, current);
        assert!(diff.contains("new.txt"));
        assert!(diff.contains("/dev/null") || diff.contains("+line1"));
    }

    #[test]
    fn test_generate_diff_from_content_empty_current() {
        let original = "line1\nline2";
        let current = "";
        let diff = generate_diff_from_content("test.txt", original, current);
        // Should still produce a valid diff structure
        assert!(diff.contains("test.txt"));
    }

    #[test]
    fn test_generate_diff_from_content_no_change() {
        let content = "line1\nline2";
        let diff = generate_diff_from_content("test.txt", content, content);
        assert!(diff.contains("test.txt"));
    }

    #[test]
    fn test_generate_diff_from_content_multiline() {
        let original = "line1\nline2\nline3";
        let current = "line1\nmodified\nline3";
        let diff = generate_diff_from_content("test.txt", original, current);
        assert!(diff.contains("test.txt"));
        assert!(diff.contains("+modified") || diff.contains("-line2"));
    }

    #[test]
    fn test_action_type_serialization() {
        let actions = vec![
            ActionType::ReadFile {
                path: "/test.txt".to_string(),
            },
            ActionType::WriteFile {
                path: "/test.txt".to_string(),
                size_bytes: 100,
            },
            ActionType::DeleteFile {
                path: "/test.txt".to_string(),
            },
            ActionType::ExecuteCommand {
                command: "test".to_string(),
            },
            ActionType::WebRequest {
                url: "https://example.com".to_string(),
            },
            ActionType::CreateDocx {
                filename: "test.docx".to_string(),
            },
            ActionType::MutateDocxDraft {
                document_id: "docx_123".to_string(),
            },
            ActionType::InspectDocxDraft {
                document_id: "docx_123".to_string(),
            },
            ActionType::CreatePptx {
                filename: "test.pptx".to_string(),
            },
            ActionType::MutatePptxDraft {
                deck_id: "pptx_123".to_string(),
            },
            ActionType::InspectPptxDraft {
                deck_id: "pptx_123".to_string(),
            },
            ActionType::CreatePdf {
                filename: "test.pdf".to_string(),
            },
            ActionType::MutatePdfDraft {
                pdf_id: "pdf_123".to_string(),
            },
            ActionType::InspectPdfDraft {
                pdf_id: "pdf_123".to_string(),
            },
            ActionType::FinalizeDocx {
                document_id: "docx_123".to_string(),
            },
            ActionType::FinalizePptx {
                deck_id: "pptx_123".to_string(),
            },
            ActionType::FinalizePdf {
                pdf_id: "pdf_123".to_string(),
            },
        ];

        for action in actions {
            let serialized = serde_json::to_string(&action).unwrap();
            let deserialized: ActionType = serde_json::from_str(&serialized).unwrap();

            match (&action, &deserialized) {
                (ActionType::ReadFile { path: a }, ActionType::ReadFile { path: b }) => {
                    assert_eq!(a, b);
                }
                (ActionType::WriteFile { path: a, .. }, ActionType::WriteFile { path: b, .. }) => {
                    assert_eq!(a, b);
                }
                (ActionType::DeleteFile { path: a }, ActionType::DeleteFile { path: b }) => {
                    assert_eq!(a, b);
                }
                (
                    ActionType::ExecuteCommand { command: a },
                    ActionType::ExecuteCommand { command: b },
                ) => {
                    assert_eq!(a, b);
                }
                (ActionType::WebRequest { url: a }, ActionType::WebRequest { url: b }) => {
                    assert_eq!(a, b);
                }
                (
                    ActionType::CreateDocx { filename: a },
                    ActionType::CreateDocx { filename: b },
                ) => {
                    assert_eq!(a, b);
                }
                (
                    ActionType::MutateDocxDraft { document_id: a },
                    ActionType::MutateDocxDraft { document_id: b },
                ) => assert_eq!(a, b),
                (
                    ActionType::InspectDocxDraft { document_id: a },
                    ActionType::InspectDocxDraft { document_id: b },
                ) => assert_eq!(a, b),
                (
                    ActionType::CreatePptx { filename: a },
                    ActionType::CreatePptx { filename: b },
                ) => {
                    assert_eq!(a, b);
                }
                (
                    ActionType::MutatePptxDraft { deck_id: a },
                    ActionType::MutatePptxDraft { deck_id: b },
                ) => assert_eq!(a, b),
                (
                    ActionType::InspectPptxDraft { deck_id: a },
                    ActionType::InspectPptxDraft { deck_id: b },
                ) => assert_eq!(a, b),
                (ActionType::CreatePdf { filename: a }, ActionType::CreatePdf { filename: b }) => {
                    assert_eq!(a, b);
                }
                (
                    ActionType::MutatePdfDraft { pdf_id: a },
                    ActionType::MutatePdfDraft { pdf_id: b },
                ) => assert_eq!(a, b),
                (
                    ActionType::InspectPdfDraft { pdf_id: a },
                    ActionType::InspectPdfDraft { pdf_id: b },
                ) => assert_eq!(a, b),
                (
                    ActionType::FinalizeDocx { document_id: a },
                    ActionType::FinalizeDocx { document_id: b },
                ) => {
                    assert_eq!(a, b);
                }
                (
                    ActionType::FinalizePptx { deck_id: a },
                    ActionType::FinalizePptx { deck_id: b },
                ) => {
                    assert_eq!(a, b);
                }
                (ActionType::FinalizePdf { pdf_id: a }, ActionType::FinalizePdf { pdf_id: b }) => {
                    assert_eq!(a, b);
                }
                _ => panic!("Action types don't match"),
            }
        }
    }

    #[test]
    fn test_infer_action_type_docx_start() {
        let action = infer_action_type(
            "docx_start",
            &serde_json::json!({"filename": "report.docx"}),
        );
        match action {
            ActionType::CreateDocx { filename } => assert_eq!(filename, "report.docx"),
            _ => panic!("Expected CreateDocx"),
        }
    }

    #[test]
    fn test_infer_action_type_docx_finalize() {
        let action = infer_action_type(
            "docx_finalize",
            &serde_json::json!({"document_id": "docx_123"}),
        );
        match action {
            ActionType::FinalizeDocx { document_id } => assert_eq!(document_id, "docx_123"),
            _ => panic!("Expected FinalizeDocx"),
        }
    }

    #[test]
    fn test_infer_action_type_pptx_start() {
        let action = infer_action_type("pptx_start", &serde_json::json!({"filename": "deck.pptx"}));
        match action {
            ActionType::CreatePptx { filename } => assert_eq!(filename, "deck.pptx"),
            _ => panic!("Expected CreatePptx"),
        }
    }

    #[test]
    fn test_infer_action_type_pdf_finalize() {
        let action = infer_action_type("pdf_finalize", &serde_json::json!({"pdf_id": "pdf_123"}));
        match action {
            ActionType::FinalizePdf { pdf_id } => assert_eq!(pdf_id, "pdf_123"),
            _ => panic!("Expected FinalizePdf"),
        }
    }

    #[test]
    fn test_infer_action_type_docx_add_section_is_docx_mutation() {
        let action = infer_action_type(
            "docx_add_section",
            &serde_json::json!({"document_id": "abc", "content": {}}),
        );
        match action {
            ActionType::MutateDocxDraft { document_id } => assert_eq!(document_id, "abc"),
            _ => panic!("Expected MutateDocxDraft"),
        }
    }

    #[test]
    fn test_infer_action_type_pptx_list_slides_is_inspect() {
        let action = infer_action_type(
            "pptx_list_slides",
            &serde_json::json!({"deck_id": "pptx_123"}),
        );
        match action {
            ActionType::InspectPptxDraft { deck_id } => assert_eq!(deck_id, "pptx_123"),
            _ => panic!("Expected InspectPptxDraft"),
        }
    }

    #[test]
    fn test_infer_action_type_pdf_add_page_is_mutation() {
        let action = infer_action_type("pdf_add_page", &serde_json::json!({"pdf_id": "pdf_123"}));
        match action {
            ActionType::MutatePdfDraft { pdf_id } => assert_eq!(pdf_id, "pdf_123"),
            _ => panic!("Expected MutatePdfDraft"),
        }
    }
}
