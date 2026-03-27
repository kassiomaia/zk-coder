use serde_json::Value;

pub(crate) fn is_internal_tool(tool_name: &str) -> bool {
    matches!(tool_name, "system_get_app_info") || tool_name.starts_with("agent_")
}

pub(crate) fn build_tool_call_feedback(tool_name: &str, arguments: &Value) -> Option<String> {
    if is_internal_tool(tool_name) {
        return None;
    }

    let feedback = match tool_name {
        "fs_read_file" | "read_file" | "read_workspace_file" => {
            let path = get_string(arguments, "path").unwrap_or("the requested file");
            format!("Let me read `{path}` and inspect its contents.")
        }
        "fs_list_files" => {
            let path = get_string(arguments, "path").unwrap_or("the workspace");
            format!("Let me inspect `{path}` to see what files are available.")
        }
        "fs_delete_file" | "delete_file" | "remove_file" => {
            let path = get_string(arguments, "path").unwrap_or("the requested file");
            format!("Let me remove `{path}`.")
        }
        "fs_write_file" => {
            let path = get_string(arguments, "path").unwrap_or("the requested file");
            format!("Let me update `{path}`.")
        }
        "fs_glob" => {
            let pattern = get_string(arguments, "pattern").unwrap_or("the requested pattern");
            format!("Let me scan the workspace for files matching `{pattern}`.")
        }
        "search_grep" => {
            let query = get_string(arguments, "query").unwrap_or("the requested pattern");
            format!("Let me search the codebase for `{query}`.")
        }
        "search_google_scrape" => {
            let query = get_string(arguments, "query").unwrap_or("that topic");
            format!("Let me look up `{query}` and gather the relevant results.")
        }
        "conversation_search_messages" => {
            let query = get_string(arguments, "query").unwrap_or("that detail");
            format!("Let me search the conversation history for `{query}`.")
        }
        "system_execute_command" | "execute_command" | "shell_command" => {
            let command = get_string(arguments, "command").unwrap_or("the requested command");
            format!("Let me run `{command}` and review the output.")
        }
        "web_fetch" | "web_request" | "fetch_url" | "http_request" => {
            let url = get_string(arguments, "url").unwrap_or("the requested URL");
            format!("Let me fetch `{url}` and inspect the response.")
        }
        "docx_start" => "Let me start a DOCX draft for this request.".to_string(),
        "docx_add_section" => "Let me add the next section to the DOCX draft.".to_string(),
        "docx_list_sections" => "Let me inspect the current DOCX draft sections.".to_string(),
        "docx_finalize" => "Let me finalize the DOCX draft.".to_string(),
        "pptx_start" => "Let me start a presentation draft for this request.".to_string(),
        "pptx_add_slide" => "Let me add the next slide to the presentation.".to_string(),
        "pptx_update_slide" => "Let me update the requested slide.".to_string(),
        "pptx_delete_slide" => "Let me remove the requested slide.".to_string(),
        "pptx_list_slides" | "pptx_get_slide" | "pptx_search_slides" => {
            "Let me inspect the presentation content.".to_string()
        }
        "pptx_finalize" => "Let me finalize the presentation draft.".to_string(),
        "pdf_start" => "Let me start a PDF draft for this request.".to_string(),
        "pdf_add_page" => "Let me add the next page to the PDF draft.".to_string(),
        "pdf_delete_page" => "Let me remove the requested PDF page.".to_string(),
        "pdf_list_pages" | "pdf_get_page" => "Let me inspect the PDF draft content.".to_string(),
        "pdf_finalize" => "Let me finalize the PDF draft.".to_string(),
        _ => format!("Let me use `{tool_name}` to gather the information I need."),
    };

    Some(feedback)
}

pub(crate) fn build_tool_result_feedback(
    tool_name: &str,
    result: &Value,
    is_error: bool,
) -> Option<String> {
    if is_internal_tool(tool_name) {
        return None;
    }

    if is_error {
        let error = result
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("the tool returned an error");
        return Some(format!("`{tool_name}` failed: {error}."));
    }

    let feedback = match tool_name {
        "fs_read_file" | "read_file" | "read_workspace_file" => {
            let path = get_string(result, "path").unwrap_or("the requested file");
            let truncated = result
                .get("truncated")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if truncated {
                format!("I read `{path}` and captured a truncated preview for review.")
            } else {
                format!("I read `{path}` and reviewed the file contents.")
            }
        }
        "fs_list_files" => {
            let path = get_string(result, "path").unwrap_or("the requested folder");
            let count = result
                .get("count")
                .and_then(Value::as_u64)
                .or_else(|| result.get("files").and_then(Value::as_array).map(|files| files.len() as u64))
                .unwrap_or(0);
            format!("I inspected `{path}` and found {count} item(s).")
        }
        "fs_delete_file" | "delete_file" | "remove_file" => {
            let path = get_string(result, "path").unwrap_or("the requested file");
            format!("I removed `{path}`.")
        }
        "fs_write_file" => {
            let path = get_string(result, "path").unwrap_or("the requested file");
            format!("I updated `{path}`.")
        }
        "fs_glob" => {
            let count = result
                .get("count")
                .and_then(Value::as_u64)
                .or_else(|| result.get("files").and_then(Value::as_array).map(|files| files.len() as u64))
                .unwrap_or(0);
            format!("I found {count} matching file(s).")
        }
        "search_grep" => {
            let count = result.get("count").and_then(Value::as_u64).unwrap_or(0);
            let query = get_string(result, "query").unwrap_or("the requested pattern");
            format!("I found {count} match(es) for `{query}`.")
        }
        "search_google_scrape" => {
            let count = result.get("count").and_then(Value::as_u64).unwrap_or(0);
            format!("I gathered {count} web result(s) for that search.")
        }
        "conversation_search_messages" => {
            let count = result.get("count").and_then(Value::as_u64).unwrap_or(0);
            format!("I found {count} matching message(s) in the conversation history.")
        }
        "system_execute_command" | "execute_command" | "shell_command" => {
            let exit_code = result
                .get("exit_code")
                .and_then(Value::as_i64)
                .map(|code| code.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            format!("The command finished with exit code {exit_code}.")
        }
        "web_fetch" | "web_request" | "fetch_url" | "http_request" => {
            let status = result
                .get("status")
                .and_then(Value::as_u64)
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            format!("I fetched the requested resource and got status {status}.")
        }
        "docx_finalize" | "pptx_finalize" | "pdf_finalize" => {
            let filename = get_string(result, "filename").unwrap_or("the generated document");
            format!("I finished generating `{filename}`.")
        }
        _ => format!("`{tool_name}` completed successfully."),
    };

    Some(feedback)
}

fn get_string<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{build_tool_call_feedback, build_tool_result_feedback, is_internal_tool};
    use serde_json::json;

    #[test]
    fn test_internal_tool_detection() {
        assert!(is_internal_tool("agent_plan_subtasks"));
        assert!(is_internal_tool("system_get_app_info"));
        assert!(!is_internal_tool("fs_read_file"));
    }

    #[test]
    fn test_build_tool_call_feedback_for_read_file() {
        let feedback = build_tool_call_feedback("fs_read_file", &json!({"path": "src/main.ts"}));
        assert_eq!(
            feedback.as_deref(),
            Some("Let me read `src/main.ts` and inspect its contents.")
        );
    }

    #[test]
    fn test_build_tool_result_feedback_for_list_files() {
        let feedback = build_tool_result_feedback(
            "fs_list_files",
            &json!({"path": "src", "count": 12}),
            false,
        );
        assert_eq!(
            feedback.as_deref(),
            Some("I inspected `src` and found 12 item(s).")
        );
    }

    #[test]
    fn test_build_tool_result_feedback_omits_internal_tools() {
        let feedback = build_tool_result_feedback("agent_narrate", &json!({}), false);
        assert!(feedback.is_none());
    }
}
