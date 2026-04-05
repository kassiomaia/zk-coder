#![cfg(all(test, feature = "integration-test"))]
use super::*;
use std::fs;
use tempfile::TempDir;

fn mock_app_handle() -> tauri::AppHandle {
    crate::test_support::mock_app_handle()
}

// === Tool metadata tests ===

#[test]
fn test_execute_command_meta() {
    let tool = ExecuteCommandTool;
    let meta = tool.meta();
    assert_eq!(meta.id.full_name(), "system_execute_command");
    assert!(meta.requires_workspace);
}
#[test]
fn test_tool_id_format() {
    let tool = ExecuteCommandTool;
    let meta = tool.meta();

    assert_eq!(meta.id.category, "system");
    assert_eq!(meta.id.name, "execute_command");
}

#[test]
fn test_input_schema() {
    let tool = ExecuteCommandTool;
    let meta = tool.meta();
    let schema = &meta.input_schema;

    assert_eq!(schema["type"], "object");
    assert!(schema["additionalProperties"] == false);

    let required = schema["required"].as_array().unwrap();
    assert!(required.contains(&serde_json::json!("command")));
}

// === Arguments parsing tests ===

#[test]
fn test_parse_args_valid() {
    let args = serde_json::json!({
        "command": "echo hello",
        "working_dir": "src"
    });

    let parsed: ExecuteCommandArgs = serde_json::from_value(args).unwrap();
    assert_eq!(parsed.command, "echo hello");
    assert_eq!(parsed.working_dir, Some("src".to_string()));
}

#[test]
fn test_parse_args_without_working_dir() {
    let args = serde_json::json!({
        "command": "ls -la"
    });

    let parsed: ExecuteCommandArgs = serde_json::from_value(args).unwrap();
    assert_eq!(parsed.command, "ls -la");
    assert!(parsed.working_dir.is_none());
}

#[test]
fn test_parse_args_missing_command() {
    let args = serde_json::json!({
        "working_dir": "src"
    });

    let parsed: Result<ExecuteCommandArgs, _> = serde_json::from_value(args);
    assert!(parsed.is_err());
}

// === Integration tests with temporary directory ===

#[tokio::test]
async fn test_execute_simple_command() {
    let temp_dir = TempDir::new().unwrap();

    let tool = ExecuteCommandTool;
    let context = ToolExecutionContext::new("test-conversation");
    let app_handle = mock_app_handle();

    let args = serde_json::json!({
        "command": "echo test output"
    });

    let result = tool
        .execute(
            context,
            args,
            app_handle,
            Some(temp_dir.path().to_path_buf()),
        )
        .await;

    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output["command"], "echo test output");
    assert_eq!(output["success"], true);
    assert_eq!(output["exit_code"], 0);
    assert!(output["stdout"].as_str().unwrap().contains("test output"));
}

#[tokio::test]
async fn test_execute_program_launch() {
    let temp_dir = TempDir::new().unwrap();

    let tool = ExecuteCommandTool;
    let context = ToolExecutionContext::new("test-conversation");
    let app_handle = mock_app_handle();

    let args = serde_json::json!({
        "command": "",
        "program": "echo",
        "args": ["program output"]
    });

    let result = tool
        .execute(
            context,
            args,
            app_handle,
            Some(temp_dir.path().to_path_buf()),
        )
        .await;

    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output["mode"], "program");
    assert_eq!(output["program"], "echo");
    assert_eq!(output["args"], serde_json::json!(["program output"]));
    assert!(output["stdout"]
        .as_str()
        .unwrap()
        .contains("program output"));
}

#[tokio::test]
async fn test_execute_command_with_nonzero_exit_code() {
    let temp_dir = TempDir::new().unwrap();

    let tool = ExecuteCommandTool;
    let context = ToolExecutionContext::new("test-conversation");
    let app_handle = mock_app_handle();

    let args = serde_json::json!({
        "command": "exit 42"
    });

    let result = tool
        .execute(
            context,
            args,
            app_handle,
            Some(temp_dir.path().to_path_buf()),
        )
        .await;

    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output["success"], false);
    assert_eq!(output["exit_code"], 42);
}

#[tokio::test]
async fn test_execute_command_with_working_dir() {
    let temp_dir = TempDir::new().unwrap();
    let subdir = temp_dir.path().join("subdir");
    fs::create_dir(&subdir).unwrap();
    fs::write(subdir.join("test.txt"), b"content").unwrap();

    let tool = ExecuteCommandTool;
    let context = ToolExecutionContext::new("test-conversation");
    let app_handle = mock_app_handle();

    let args = serde_json::json!({
        "command": "cat test.txt",
        "working_dir": "subdir"
    });

    let result = tool
        .execute(
            context,
            args,
            app_handle,
            Some(temp_dir.path().to_path_buf()),
        )
        .await;

    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output["stdout"].as_str().unwrap().contains("content"));
}

#[tokio::test]
async fn test_execute_command_empty_command_fails() {
    let temp_dir = TempDir::new().unwrap();

    let tool = ExecuteCommandTool;
    let context = ToolExecutionContext::new("test-conversation");
    let app_handle = mock_app_handle();

    let args = serde_json::json!({
        "command": "   "
    });

    let result = tool
        .execute(
            context,
            args,
            app_handle,
            Some(temp_dir.path().to_path_buf()),
        )
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("empty"));
}

#[tokio::test]
async fn test_execute_command_no_workspace_fails() {
    let tool = ExecuteCommandTool;
    let context = ToolExecutionContext::new("test-conversation");
    let app_handle = mock_app_handle();

    let args = serde_json::json!({
        "command": "echo test"
    });

    let result = tool.execute(context, args, app_handle, None).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("workspace"));
}

#[tokio::test]
async fn test_execute_command_working_dir_outside_workspace_fails() {
    let temp_dir = TempDir::new().unwrap();

    let tool = ExecuteCommandTool;
    let context = ToolExecutionContext::new("test-conversation");
    let app_handle = mock_app_handle();

    let args = serde_json::json!({
        "command": "echo test",
        "working_dir": "../../../etc"
    });

    let result = tool
        .execute(
            context,
            args,
            app_handle,
            Some(temp_dir.path().to_path_buf()),
        )
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("outside workspace"));
}

#[tokio::test]
async fn test_execute_command_nonexistent_working_dir_fails() {
    let temp_dir = TempDir::new().unwrap();

    let tool = ExecuteCommandTool;
    let context = ToolExecutionContext::new("test-conversation");
    let app_handle = mock_app_handle();

    let args = serde_json::json!({
        "command": "echo test",
        "working_dir": "nonexistent_dir"
    });

    let result = tool
        .execute(
            context,
            args,
            app_handle,
            Some(temp_dir.path().to_path_buf()),
        )
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("does not exist"));
}

#[tokio::test]
async fn test_execute_command_stderr_captured() {
    let temp_dir = TempDir::new().unwrap();

    let tool = ExecuteCommandTool;
    let context = ToolExecutionContext::new("test-conversation");
    let app_handle = mock_app_handle();

    let args = serde_json::json!({
        "command": "sh -c 'echo error >&2'"
    });

    let result = tool
        .execute(
            context,
            args,
            app_handle,
            Some(temp_dir.path().to_path_buf()),
        )
        .await;

    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output["stderr"].as_str().unwrap().contains("error"));
}

#[tokio::test]
async fn test_execute_command_with_pipe() {
    let temp_dir = TempDir::new().unwrap();

    let tool = ExecuteCommandTool;
    let context = ToolExecutionContext::new("test-conversation");
    let app_handle = mock_app_handle();

    let args = serde_json::json!({
        "command": "echo hello world | grep hello"
    });

    let result = tool
        .execute(
            context,
            args,
            app_handle,
            Some(temp_dir.path().to_path_buf()),
        )
        .await;

    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output["stdout"].as_str().unwrap().contains("hello"));
}

#[tokio::test]
async fn test_execute_command_list_files() {
    let temp_dir = TempDir::new().unwrap();
    fs::write(temp_dir.path().join("file1.txt"), b"content1").unwrap();
    fs::write(temp_dir.path().join("file2.txt"), b"content2").unwrap();

    let tool = ExecuteCommandTool;
    let context = ToolExecutionContext::new("test-conversation");
    let app_handle = mock_app_handle();

    let args = serde_json::json!({
        "command": "ls"
    });

    let result = tool
        .execute(
            context,
            args,
            app_handle,
            Some(temp_dir.path().to_path_buf()),
        )
        .await;

    assert!(result.is_ok());
    let output = result.unwrap();
    let stdout = output["stdout"].as_str().unwrap();
    assert!(stdout.contains("file1.txt") || stdout.contains("file2.txt"));
}

#[tokio::test]
async fn test_execute_command_multiline_output() {
    let temp_dir = TempDir::new().unwrap();

    let tool = ExecuteCommandTool;
    let context = ToolExecutionContext::new("test-conversation");
    let app_handle = mock_app_handle();

    let args = serde_json::json!({
        "command": "echo line1 && echo line2 && echo line3"
    });

    let result = tool
        .execute(
            context,
            args,
            app_handle,
            Some(temp_dir.path().to_path_buf()),
        )
        .await;

    assert!(result.is_ok());
    let output = result.unwrap();
    let stdout = output["stdout"].as_str().unwrap();
    assert!(stdout.contains("line1"));
    assert!(stdout.contains("line2"));
    assert!(stdout.contains("line3"));
}

#[tokio::test]
async fn test_execute_command_environment_variable() {
    let temp_dir = TempDir::new().unwrap();

    let tool = ExecuteCommandTool;
    let context = ToolExecutionContext::new("test-conversation");
    let app_handle = mock_app_handle();

    let args = serde_json::json!({
        "command": "echo $HOME"
    });

    let result = tool
        .execute(
            context,
            args,
            app_handle,
            Some(temp_dir.path().to_path_buf()),
        )
        .await;

    assert!(result.is_ok());
    let output = result.unwrap();
    // HOME should be set
    let stdout = output["stdout"].as_str().unwrap();
    assert!(!stdout.trim().is_empty());
}

// === Constants tests ===

#[test]
fn test_max_output_bytes() {
    assert_eq!(MAX_OUTPUT_BYTES, 512 * 1024);
}

#[test]
fn test_command_timeout_secs() {
    assert_eq!(COMMAND_TIMEOUT_SECS, 60);
}

// === Output structure tests ===

#[tokio::test]
async fn test_execute_command_output_structure() {
    let temp_dir = TempDir::new().unwrap();

    let tool = ExecuteCommandTool;
    let context = ToolExecutionContext::new("test-conversation");
    let app_handle = mock_app_handle();

    let args = serde_json::json!({
        "command": "echo test"
    });

    let result = tool
        .execute(
            context,
            args,
            app_handle,
            Some(temp_dir.path().to_path_buf()),
        )
        .await;

    assert!(result.is_ok());
    let output = result.unwrap();

    // Verify all expected fields
    assert!(output.get("command").is_some());
    assert!(output.get("exit_code").is_some());
    assert!(output.get("stdout").is_some());
    assert!(output.get("stderr").is_some());
    assert!(output.get("success").is_some());
    assert!(output.get("truncated").is_some());

    // Verify types
    assert!(output["command"].is_string());
    assert!(output["exit_code"].is_number());
    assert!(output["stdout"].is_string());
    assert!(output["stderr"].is_string());
    assert!(output["success"].is_boolean());
    assert!(output["truncated"].is_boolean());
}

#[tokio::test]
async fn test_execute_command_output_is_json_serializable() {
    let temp_dir = TempDir::new().unwrap();

    let tool = ExecuteCommandTool;
    let context = ToolExecutionContext::new("test-conversation");
    let app_handle = mock_app_handle();

    let args = serde_json::json!({
        "command": "echo test"
    });

    let result = tool
        .execute(
            context,
            args,
            app_handle,
            Some(temp_dir.path().to_path_buf()),
        )
        .await;

    assert!(result.is_ok());
    let output = result.unwrap();

    // Verify we can serialize back to JSON string
    let json_string = serde_json::to_string(&output);
    assert!(json_string.is_ok());
}

// === Context tests ===

#[tokio::test]
async fn test_execute_with_session_context() {
    let temp_dir = TempDir::new().unwrap();

    let tool = ExecuteCommandTool;
    let context = ToolExecutionContext::new("test-conversation").with_session("test-session");
    let app_handle = mock_app_handle();

    let args = serde_json::json!({
        "command": "echo test"
    });

    let result = tool
        .execute(
            context,
            args,
            app_handle,
            Some(temp_dir.path().to_path_buf()),
        )
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_execute_with_thread_context() {
    let temp_dir = TempDir::new().unwrap();

    let tool = ExecuteCommandTool;
    let context = ToolExecutionContext::new("test-conversation").with_thread("test-thread");
    let app_handle = mock_app_handle();

    let args = serde_json::json!({
        "command": "echo test"
    });

    let result = tool
        .execute(
            context,
            args,
            app_handle,
            Some(temp_dir.path().to_path_buf()),
        )
        .await;

    assert!(result.is_ok());
}
