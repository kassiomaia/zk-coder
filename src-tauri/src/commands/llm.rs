//! LLM Tauri IPC commands.
//!
//! This module provides Tauri commands for LLM chat completions and streaming.

use futures::future::BoxFuture;
use regex::Regex;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use log::{error, info, warn};
use tauri::{AppHandle, Emitter, Manager};

use crate::commands::config::get_code_operation_model;
use crate::llm::{
    create_client, ChatRequest, ChatResponse, LlmClient, LlmError, LlmEvent, Message,
    ToolCall as LlmToolCall, ToolDefinition as LlmToolDefinition,
};
use crate::nlp::chat::{ChatMessage, MessageFeedback, MessageRole};
use crate::nlp::config::ApiType;
use crate::storage::ConfigStore;

// Re-export ChatState for use in this module
use super::ChatState;

/// Environment flag for enabling the orchestrated runner path.
const ORCHESTRATOR_FLAG_ENV: &str = "CODER_LLM_ORCHESTRATOR";
/// Environment flag for enabling real tool execution in the orchestrator.
const ORCHESTRATOR_TOOL_REGISTRY_ENV: &str = "CODER_LLM_TOOL_REGISTRY";
/// Optional workspace root override for workspace-scoped tool handlers.
const ORCHESTRATOR_WORKSPACE_ROOT_ENV: &str = "CODER_LLM_TOOL_WORKSPACE_ROOT";
/// Dev-only flag enabling keyless Google scraping tool registration.
const ORCHESTRATOR_GOOGLE_SCRAPE_TOOL_ENV: &str = "CODER_LLM_TOOL_GOOGLE_SCRAPE";
/// Safety bound for orchestrated request loops.
const ORCHESTRATOR_MAX_STEPS: usize = 6;
/// Max retries for transient provider failures during one orchestrator step.
const ORCHESTRATOR_LLM_MAX_RETRIES: usize = 2;
/// Base retry delay for transient provider failures.
const ORCHESTRATOR_LLM_RETRY_BASE_MS: u64 = 500;

/// Application state for LLM commands.
pub struct LlmState {
    /// Config store for provider/model lookup
    pub config_store: Arc<Mutex<ConfigStore>>,
    /// Active streaming tasks keyed by request ID for cancellation.
    pub active_requests: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    /// Conversation runner strategy (legacy single-turn by default).
    pub runner: Arc<dyn ConversationRunner>,
}

impl LlmState {
    /// Creates a new LLM state.
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be initialized.
    pub fn new(db_path: &str) -> Result<Self, String> {
        let config_store = ConfigStore::new(db_path).map_err(|e| e.to_string())?;
        Ok(Self {
            config_store: Arc::new(Mutex::new(config_store)),
            active_requests: Arc::new(Mutex::new(HashMap::new())),
            runner: select_runner_from_env(),
        })
    }

    /// Creates a new LLM state with an in-memory database (for testing).
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be initialized.
    pub fn in_memory() -> Result<Self, String> {
        let config_store = ConfigStore::in_memory().map_err(|e| e.to_string())?;
        Ok(Self {
            config_store: Arc::new(Mutex::new(config_store)),
            active_requests: Arc::new(Mutex::new(HashMap::new())),
            runner: select_runner_from_env(),
        })
    }
}

/// Abstraction for conversation execution strategy.
///
/// This enables branch-by-abstraction migration from the legacy single-turn
/// implementation to future orchestrated/tool-capable runners without changing
/// command signatures.
pub trait ConversationRunner: Send + Sync {
    /// Handles a non-streaming chat request.
    fn send_chat_message<'a>(
        &'a self,
        request: SendChatRequest,
        app_handle: AppHandle,
    ) -> BoxFuture<'a, Result<SendChatResponse, String>>;

    /// Handles a streaming chat request.
    fn send_chat_message_streaming<'a>(
        &'a self,
        request: SendChatRequest,
        app_handle: AppHandle,
    ) -> BoxFuture<'a, Result<String, String>>;
}

/// Legacy runner that preserves the current request/response behavior.
pub struct LegacyConversationRunner;

impl ConversationRunner for LegacyConversationRunner {
    fn send_chat_message<'a>(
        &'a self,
        request: SendChatRequest,
        app_handle: AppHandle,
    ) -> BoxFuture<'a, Result<SendChatResponse, String>> {
        Box::pin(async move { send_chat_message_legacy(request, app_handle).await })
    }

    fn send_chat_message_streaming<'a>(
        &'a self,
        request: SendChatRequest,
        app_handle: AppHandle,
    ) -> BoxFuture<'a, Result<String, String>> {
        Box::pin(async move { send_chat_message_streaming_legacy(request, app_handle).await })
    }
}

/// Orchestrated runner for bounded tool-capable turns.
pub struct OrchestratedConversationRunner;

impl ConversationRunner for OrchestratedConversationRunner {
    fn send_chat_message<'a>(
        &'a self,
        request: SendChatRequest,
        app_handle: AppHandle,
    ) -> BoxFuture<'a, Result<SendChatResponse, String>> {
        Box::pin(async move {
            info!(
                "[LLM] Orchestrated runner enabled: executing loop skeleton for non-streaming request"
            );
            let tool_executor = select_tool_executor_from_env(&request.conversation_id);
            match send_chat_message_orchestrated_non_stream(request.clone(), app_handle.clone(), tool_executor.as_ref()).await {
                Ok(response) => Ok(response),
                Err(e) => {
                    warn!(
                        "[LLM] Orchestrated runner failed, falling back to legacy: {}",
                        e
                    );
                    // Fallback to legacy runner
                    send_chat_message_legacy(request, app_handle).await
                }
            }
        })
    }

    fn send_chat_message_streaming<'a>(
        &'a self,
        request: SendChatRequest,
        app_handle: AppHandle,
    ) -> BoxFuture<'a, Result<String, String>> {
        Box::pin(async move {
            info!(
                "[LLM] Orchestrated runner enabled: streaming request using orchestrated loop path"
            );
            match send_chat_message_streaming_orchestrated(request.clone(), app_handle.clone()).await {
                Ok(response_id) => Ok(response_id),
                Err(e) => {
                    warn!(
                        "[LLM] Orchestrated streaming runner failed, falling back to legacy: {}",
                        e
                    );
                    // Fallback to legacy runner
                    send_chat_message_streaming_legacy(request, app_handle).await
                }
            }
        })
    }
}

/// Parses common truthy values from an environment variable.
/// Returns true by default if the variable is not set.
fn parse_env_flag(value: Option<&str>) -> bool {
    match value.map(str::trim).map(|v| v.to_ascii_lowercase()) {
        Some(v) => !matches!(v.as_str(), "0" | "false" | "no" | "off" | "disable"),
        None => true,
    }
}

/// Selects the runner strategy from environment configuration.
fn select_runner_from_env() -> Arc<dyn ConversationRunner> {
    if parse_env_flag(std::env::var(ORCHESTRATOR_FLAG_ENV).ok().as_deref()) {
        info!(
            "[LLM] {}=true -> using OrchestratedConversationRunner",
            ORCHESTRATOR_FLAG_ENV
        );
        Arc::new(OrchestratedConversationRunner)
    } else {
        info!("[LLM] Defaulting to OrchestratedConversationRunner (tools enabled)");
        Arc::new(OrchestratedConversationRunner)
    }
}

/// Selects the tool executor strategy from environment configuration.
fn select_tool_executor_from_env(conversation_id: &str) -> Box<dyn ToolExecutor> {
    if !parse_env_flag(
        std::env::var(ORCHESTRATOR_TOOL_REGISTRY_ENV)
            .ok()
            .as_deref(),
    ) {
        return Box::new(NoopToolExecutor);
    }

    match ToolRegistry::new() {
        Ok(registry) => {
            info!(
                "[LLM] {}=true -> enabling registry-backed tool execution ({} tools)",
                ORCHESTRATOR_TOOL_REGISTRY_ENV,
                registry.definitions().len()
            );
            Box::new(RegistryToolExecutor {
                registry: Arc::new(registry),
                context: ToolExecutionContext {
                    conversation_id: conversation_id.to_string(),
                },
            })
        }
        Err(err) => {
            warn!(
                "[LLM] Tool registry initialization failed, using noop executor. error={}",
                err
            );
            Box::new(NoopToolExecutor)
        }
    }
}

/// Internal representation of tool execution output.
#[derive(Debug, Clone, PartialEq)]
struct OrchestratorToolResult {
    call_id: String,
    name: String,
    output: serde_json::Value,
    is_error: bool,
}

/// Appends structured tool results to LLM context for the next model turn.
fn append_tool_results_to_llm_context(
    llm_messages: &mut Vec<Message>,
    results: &[OrchestratorToolResult],
) {
    for result in results {
        llm_messages.push(Message::tool_result(
            result.call_id.clone(),
            result.output.clone(),
            result.is_error,
        ));
    }
}

fn is_textual_tool_plan(content: &str) -> bool {
    fn is_tool_call_object(value: &serde_json::Value) -> bool {
        let serde_json::Value::Object(object) = value else {
            return false;
        };
        let Some(serde_json::Value::String(name)) = object.get("name") else {
            return false;
        };
        if name.trim().is_empty() {
            return false;
        }

        matches!(
            object.get("arguments"),
            Some(serde_json::Value::Object(_)) | Some(serde_json::Value::String(_))
        )
    }

    let trimmed = content.trim();
    if trimmed.is_empty() {
        return false;
    }

    let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) else {
        return false;
    };

    match value {
        serde_json::Value::Array(items) => {
            !items.is_empty() && items.iter().all(is_tool_call_object)
        }
        serde_json::Value::Object(object) => object
            .get("tool_calls")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|items| !items.is_empty() && items.iter().all(is_tool_call_object)),
        _ => false,
    }
}

/// Tool execution abstraction for orchestrated turns.
trait ToolExecutor: Send + Sync {
    fn definitions(&self) -> Vec<LlmToolDefinition>;

    fn execute<'a>(
        &'a self,
        call: LlmToolCall,
        app_handle: AppHandle,
    ) -> BoxFuture<'a, OrchestratorToolResult>;
}

/// Default executor placeholder.
struct NoopToolExecutor;

impl ToolExecutor for NoopToolExecutor {
    fn definitions(&self) -> Vec<LlmToolDefinition> {
        Vec::new()
    }

    fn execute<'a>(
        &'a self,
        call: LlmToolCall,
        _app_handle: AppHandle,
    ) -> BoxFuture<'a, OrchestratorToolResult> {
        Box::pin(async move {
            OrchestratorToolResult {
                call_id: call.id,
                name: call.name,
                output: serde_json::json!({
                    "error": "Tool execution not implemented yet in this environment",
                    "arguments": call.arguments
                }),
                is_error: true,
            }
        })
    }
}

/// Per-request context for tool execution.
#[derive(Debug, Clone)]
struct ToolExecutionContext {
    conversation_id: String,
}

/// Async handler signature for registry-backed tools.
type ToolHandler = fn(
    ToolExecutionContext,
    serde_json::Value,
    AppHandle,
    PathBuf,
) -> BoxFuture<'static, Result<serde_json::Value, String>>;

/// Registered tool metadata and handler.
#[derive(Clone)]
struct ToolRegistration {
    definition: LlmToolDefinition,
    handler: ToolHandler,
}

/// In-process registry for orchestrator tool execution.
#[derive(Clone)]
struct ToolRegistry {
    workspace_root: PathBuf,
    tools: HashMap<String, ToolRegistration>,
}

/// Registry-backed executor for orchestrated turns.
struct RegistryToolExecutor {
    registry: Arc<ToolRegistry>,
    context: ToolExecutionContext,
}

impl ToolExecutor for RegistryToolExecutor {
    fn definitions(&self) -> Vec<LlmToolDefinition> {
        self.registry.definitions()
    }

    fn execute<'a>(
        &'a self,
        call: LlmToolCall,
        app_handle: AppHandle,
    ) -> BoxFuture<'a, OrchestratorToolResult> {
        let registry = self.registry.clone();
        let context = self.context.clone();
        Box::pin(async move { registry.execute(context, call, app_handle).await })
    }
}

impl ToolRegistry {
    fn new() -> Result<Self, String> {
        let workspace_root = resolve_workspace_root()?;
        Ok(Self {
            workspace_root,
            tools: Self::build_builtin_tools(),
        })
    }

    fn definitions(&self) -> Vec<LlmToolDefinition> {
        self.tools
            .values()
            .map(|registration| registration.definition.clone())
            .collect()
    }

    async fn execute(
        &self,
        context: ToolExecutionContext,
        call: LlmToolCall,
        app_handle: AppHandle,
    ) -> OrchestratorToolResult {
        let Some(registration) = self.tools.get(&call.name) else {
            let mut available = self.tools.keys().cloned().collect::<Vec<_>>();
            available.sort();
            return OrchestratorToolResult {
                call_id: call.id,
                name: call.name,
                output: serde_json::json!({
                    "error": "Unknown tool name",
                    "available_tools": available,
                }),
                is_error: true,
            };
        };

        let handler = registration.handler;
        match handler(
            context,
            call.arguments,
            app_handle,
            self.workspace_root.clone(),
        )
        .await
        {
            Ok(output) => OrchestratorToolResult {
                call_id: call.id,
                name: call.name,
                output,
                is_error: false,
            },
            Err(error_message) => OrchestratorToolResult {
                call_id: call.id,
                name: call.name,
                output: serde_json::json!({
                    "error": error_message,
                }),
                is_error: true,
            },
        }
    }

    fn build_builtin_tools() -> HashMap<String, ToolRegistration> {
        let mut tools = HashMap::new();
        let get_app_info_name = "get_app_info".to_string();
        tools.insert(
            get_app_info_name.clone(),
            ToolRegistration {
                definition: LlmToolDefinition {
                    name: get_app_info_name,
                    description: "Return backend application metadata (name/version).".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "additionalProperties": false,
                        "properties": {}
                    }),
                },
                handler: get_app_info_tool,
            },
        );

        let read_file_name = "read_workspace_file".to_string();
        tools.insert(
            read_file_name.clone(),
            ToolRegistration {
                definition: LlmToolDefinition {
                    name: read_file_name,
                    description: "Read UTF-8 content from a file inside the workspace root."
                        .to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "additionalProperties": false,
                        "properties": {
                            "path": { "type": "string" },
                            "max_bytes": { "type": "integer", "minimum": 1, "maximum": 200000 }
                        },
                        "required": ["path"]
                    }),
                },
                handler: read_workspace_file_tool,
            },
        );

        let list_files_name = "list_workspace_files".to_string();
        tools.insert(
            list_files_name.clone(),
            ToolRegistration {
                definition: LlmToolDefinition {
                    name: list_files_name,
                    description:
                        "List files under a workspace path with bounded recursion and limit."
                            .to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "additionalProperties": false,
                        "properties": {
                            "path": { "type": "string" },
                            "max_depth": { "type": "integer", "minimum": 0, "maximum": 8 },
                            "limit": { "type": "integer", "minimum": 1, "maximum": 500 }
                        }
                    }),
                },
                handler: list_workspace_files_tool,
            },
        );

        let search_messages_name = "search_conversation_messages".to_string();
        tools.insert(
            search_messages_name.clone(),
            ToolRegistration {
                definition: LlmToolDefinition {
                    name: search_messages_name,
                    description:
                        "Search existing conversation messages for a query (case-insensitive)."
                            .to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "additionalProperties": false,
                        "properties": {
                            "query": { "type": "string" },
                            "limit": { "type": "integer", "minimum": 1, "maximum": 100 }
                        },
                        "required": ["query"]
                    }),
                },
                handler: search_conversation_messages_tool,
            },
        );

        if parse_env_flag(
            std::env::var(ORCHESTRATOR_GOOGLE_SCRAPE_TOOL_ENV)
                .ok()
                .as_deref(),
        ) {
            let google_search_name = "google_web_search_scrape".to_string();
            tools.insert(
                google_search_name.clone(),
                ToolRegistration {
                    definition: LlmToolDefinition {
                        name: google_search_name,
                        description:
                            "Dev-only keyless Google web search using HTML scraping (unstable)."
                                .to_string(),
                        input_schema: serde_json::json!({
                            "type": "object",
                            "additionalProperties": false,
                            "properties": {
                                "query": { "type": "string" },
                                "num_results": { "type": "integer", "minimum": 1, "maximum": 10 },
                                "language": { "type": "string" }
                            },
                            "required": ["query"]
                        }),
                    },
                    handler: google_web_search_scrape_tool,
                },
            );
        }

        tools
    }
}

const TOOL_DEFAULT_READ_MAX_BYTES: usize = 16_000;
const TOOL_MAX_READ_MAX_BYTES: usize = 200_000;
const TOOL_DEFAULT_LIST_DEPTH: usize = 3;
const TOOL_MAX_LIST_DEPTH: usize = 8;
const TOOL_DEFAULT_LIST_LIMIT: usize = 200;
const TOOL_MAX_LIST_LIMIT: usize = 500;
const TOOL_DEFAULT_MESSAGE_SEARCH_LIMIT: usize = 20;
const TOOL_MAX_MESSAGE_SEARCH_LIMIT: usize = 100;
const TOOL_DEFAULT_GOOGLE_RESULTS_LIMIT: usize = 5;
const TOOL_MAX_GOOGLE_RESULTS_LIMIT: usize = 10;
const TOOL_WEB_TIMEOUT_SECONDS: u64 = 12;

#[derive(Debug, serde::Deserialize)]
struct ReadWorkspaceFileArgs {
    path: String,
    #[serde(default = "default_tool_read_max_bytes")]
    max_bytes: usize,
}

#[derive(Debug, serde::Deserialize)]
struct ListWorkspaceFilesArgs {
    #[serde(default)]
    path: Option<String>,
    #[serde(default = "default_tool_list_depth")]
    max_depth: usize,
    #[serde(default = "default_tool_list_limit")]
    limit: usize,
}

#[derive(Debug, serde::Deserialize)]
struct SearchConversationMessagesArgs {
    query: String,
    #[serde(default = "default_tool_message_search_limit")]
    limit: usize,
}

#[derive(Debug, serde::Deserialize)]
struct GoogleWebSearchScrapeArgs {
    query: String,
    #[serde(default = "default_tool_google_results_limit")]
    num_results: usize,
    #[serde(default)]
    language: Option<String>,
}

const fn default_tool_read_max_bytes() -> usize {
    TOOL_DEFAULT_READ_MAX_BYTES
}

const fn default_tool_list_depth() -> usize {
    TOOL_DEFAULT_LIST_DEPTH
}

const fn default_tool_list_limit() -> usize {
    TOOL_DEFAULT_LIST_LIMIT
}

const fn default_tool_message_search_limit() -> usize {
    TOOL_DEFAULT_MESSAGE_SEARCH_LIMIT
}

const fn default_tool_google_results_limit() -> usize {
    TOOL_DEFAULT_GOOGLE_RESULTS_LIMIT
}

fn parse_tool_args<T>(tool_name: &str, arguments: serde_json::Value) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_value(arguments)
        .map_err(|err| format!("Invalid arguments for '{}': {}", tool_name, err))
}

fn resolve_workspace_root() -> Result<PathBuf, String> {
    let root_candidate = std::env::var(ORCHESTRATOR_WORKSPACE_ROOT_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .map_or_else(std::env::current_dir, Ok)
        .map_err(|err| format!("Failed to resolve current directory for tool root: {}", err))?;

    let canonical = root_candidate
        .canonicalize()
        .map_err(|err| format!("Failed to canonicalize tool root: {}", err))?;
    if !canonical.is_dir() {
        return Err(format!(
            "Tool workspace root is not a directory: {}",
            canonical.display()
        ));
    }

    Ok(canonical)
}

fn resolve_workspace_path(workspace_root: &Path, raw_path: &str) -> Result<PathBuf, String> {
    let trimmed = raw_path.trim();
    if trimmed.is_empty() {
        return Err("Path argument cannot be empty".to_string());
    }

    let candidate = if Path::new(trimmed).is_absolute() {
        PathBuf::from(trimmed)
    } else {
        workspace_root.join(trimmed)
    };

    let canonical = candidate
        .canonicalize()
        .map_err(|err| format!("Failed to resolve path '{}': {}", trimmed, err))?;
    if !canonical.starts_with(workspace_root) {
        return Err(format!(
            "Path '{}' resolves outside workspace root",
            trimmed
        ));
    }

    Ok(canonical)
}

fn workspace_relative_path(workspace_root: &Path, absolute_path: &Path) -> String {
    absolute_path.strip_prefix(workspace_root).map_or_else(
        |_| absolute_path.to_string_lossy().to_string(),
        |relative| {
            let value = relative.to_string_lossy().to_string();
            if value.is_empty() {
                ".".to_string()
            } else {
                value
            }
        },
    )
}

fn should_skip_directory(name: &str) -> bool {
    matches!(
        name,
        ".git" | "node_modules" | "target" | "dist" | "build" | ".next"
    )
}

fn collect_workspace_files(
    workspace_root: &Path,
    current_path: &Path,
    depth: usize,
    max_depth: usize,
    limit: usize,
    out: &mut Vec<String>,
) {
    if out.len() >= limit || depth > max_depth {
        return;
    }

    let entries = match fs::read_dir(current_path) {
        Ok(entries) => entries,
        Err(err) => {
            warn!(
                "[LLM] Skipping unreadable directory '{}': {}",
                current_path.display(),
                err
            );
            return;
        }
    };

    for entry in entries {
        if out.len() >= limit {
            break;
        }
        let Ok(entry) = entry else {
            continue;
        };
        let Ok(file_type) = entry.file_type() else {
            continue;
        };

        let path = entry.path();
        if file_type.is_dir() {
            let directory_name = entry.file_name().to_string_lossy().to_string();
            if should_skip_directory(&directory_name) {
                continue;
            }
            collect_workspace_files(workspace_root, &path, depth + 1, max_depth, limit, out);
            continue;
        }
        if file_type.is_file() {
            out.push(workspace_relative_path(workspace_root, &path));
        }
    }
}

fn get_app_info_tool(
    _context: ToolExecutionContext,
    arguments: serde_json::Value,
    _app_handle: AppHandle,
    _workspace_root: PathBuf,
) -> BoxFuture<'static, Result<serde_json::Value, String>> {
    Box::pin(async move {
        if !arguments.is_null() && arguments != serde_json::json!({}) {
            return Err("get_app_info does not accept arguments".to_string());
        }
        Ok(serde_json::json!({
            "name": env!("CARGO_PKG_NAME"),
            "version": env!("CARGO_PKG_VERSION"),
        }))
    })
}

fn read_workspace_file_tool(
    _context: ToolExecutionContext,
    arguments: serde_json::Value,
    _app_handle: AppHandle,
    workspace_root: PathBuf,
) -> BoxFuture<'static, Result<serde_json::Value, String>> {
    Box::pin(async move {
        let args: ReadWorkspaceFileArgs = parse_tool_args("read_workspace_file", arguments)?;
        let absolute_path = resolve_workspace_path(&workspace_root, &args.path)?;
        if !absolute_path.is_file() {
            return Err(format!("Path is not a file: {}", args.path));
        }

        let max_bytes = args.max_bytes.clamp(1, TOOL_MAX_READ_MAX_BYTES);
        let bytes = fs::read(&absolute_path)
            .map_err(|err| format!("Failed to read file '{}': {}", args.path, err))?;
        let size_bytes = bytes.len();
        let truncated = size_bytes > max_bytes;
        let content_bytes = if truncated {
            &bytes[..max_bytes]
        } else {
            &bytes
        };
        let content = String::from_utf8_lossy(content_bytes).to_string();

        Ok(serde_json::json!({
            "path": workspace_relative_path(&workspace_root, &absolute_path),
            "size_bytes": size_bytes,
            "truncated": truncated,
            "content": content,
        }))
    })
}

fn list_workspace_files_tool(
    _context: ToolExecutionContext,
    arguments: serde_json::Value,
    _app_handle: AppHandle,
    workspace_root: PathBuf,
) -> BoxFuture<'static, Result<serde_json::Value, String>> {
    Box::pin(async move {
        let args: ListWorkspaceFilesArgs = parse_tool_args("list_workspace_files", arguments)?;
        let max_depth = args.max_depth.min(TOOL_MAX_LIST_DEPTH);
        let limit = args.limit.clamp(1, TOOL_MAX_LIST_LIMIT);
        let base_path = args.path.unwrap_or_else(|| ".".to_string());
        let absolute_path = resolve_workspace_path(&workspace_root, &base_path)?;
        if !absolute_path.is_dir() {
            return Err(format!("Path is not a directory: {}", base_path));
        }

        let mut files = Vec::with_capacity(limit.min(128));
        collect_workspace_files(
            &workspace_root,
            &absolute_path,
            0,
            max_depth,
            limit,
            &mut files,
        );
        files.sort();

        Ok(serde_json::json!({
            "root": workspace_root.to_string_lossy().to_string(),
            "path": workspace_relative_path(&workspace_root, &absolute_path),
            "count": files.len(),
            "files": files,
        }))
    })
}

fn search_conversation_messages_tool(
    context: ToolExecutionContext,
    arguments: serde_json::Value,
    app_handle: AppHandle,
    _workspace_root: PathBuf,
) -> BoxFuture<'static, Result<serde_json::Value, String>> {
    Box::pin(async move {
        let args: SearchConversationMessagesArgs =
            parse_tool_args("search_conversation_messages", arguments)?;
        let needle = args.query.trim().to_ascii_lowercase();
        if needle.is_empty() {
            return Err("query cannot be empty".to_string());
        }
        let limit = args.limit.clamp(1, TOOL_MAX_MESSAGE_SEARCH_LIMIT);

        let chat_state = app_handle.state::<ChatState>();
        let chat_store = chat_state.store.lock().await;
        let history = chat_store
            .load_messages(&context.conversation_id)
            .map_err(|err| err.to_string())?;

        let mut matches = Vec::new();
        for message in history {
            if matches.len() >= limit {
                break;
            }
            if !message.content.to_ascii_lowercase().contains(&needle) {
                continue;
            }
            matches.push(serde_json::json!({
                "id": message.id,
                "role": message.role.as_str(),
                "thread_id": message.thread_id,
                "timestamp": message.timestamp,
                "content": message.content,
            }));
        }

        Ok(serde_json::json!({
            "conversation_id": context.conversation_id,
            "query": args.query,
            "count": matches.len(),
            "matches": matches,
        }))
    })
}

fn decode_html_entities(input: &str) -> String {
    input
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ")
}

fn strip_html_tags(input: &str) -> String {
    let without_tags = Regex::new(r"<[^>]+>").ok().map_or_else(
        || input.to_string(),
        |tag_re| tag_re.replace_all(input, "").to_string(),
    );
    let decoded = decode_html_entities(without_tags.as_ref());
    decoded.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn extract_google_result_url(encoded_href_query: &str) -> Option<String> {
    let decoded = decode_html_entities(encoded_href_query);
    let query = if decoded.starts_with("q=") {
        decoded
    } else {
        format!("q={}", decoded)
    };
    let url = reqwest::Url::parse(&format!("https://www.google.com/url?{}", query)).ok()?;
    for (key, value) in url.query_pairs() {
        if key == "q" && (value.starts_with("http://") || value.starts_with("https://")) {
            return Some(value.into_owned());
        }
    }
    None
}

fn parse_google_search_results(html: &str, limit: usize) -> Vec<serde_json::Value> {
    let anchor_prefix = "<a href=\"/url?q=";
    let mut results = Vec::new();
    let mut cursor = 0;

    while results.len() < limit {
        let Some(anchor_offset) = html[cursor..].find(anchor_prefix) else {
            break;
        };
        let anchor_start = cursor + anchor_offset;
        let href_start = anchor_start + anchor_prefix.len();
        let Some(href_end_rel) = html[href_start..].find('"') else {
            break;
        };
        let href_end = href_start + href_end_rel;
        let href_value = &html[href_start..href_end];
        cursor = href_end.saturating_add(1);

        let Some(url) = extract_google_result_url(href_value) else {
            continue;
        };

        let window_end = (anchor_start + 5000).min(html.len());
        let window = &html[anchor_start..window_end];

        let Some(h3_start_rel) = window.find("<h3") else {
            continue;
        };
        let h3_start = h3_start_rel;
        let Some(h3_open_end_rel) = window[h3_start..].find('>') else {
            continue;
        };
        let title_start = h3_start + h3_open_end_rel + 1;
        let Some(title_end_rel) = window[title_start..].find("</h3>") else {
            continue;
        };
        let title_end = title_start + title_end_rel;
        let title = strip_html_tags(&window[title_start..title_end]);
        if title.is_empty() {
            continue;
        };

        let snippet = window
            .find("VwiC3b")
            .and_then(|class_offset| window[class_offset..].find('>').map(|o| class_offset + o))
            .and_then(|snippet_start| {
                let content_start = snippet_start + 1;
                window[content_start..]
                    .find("</div>")
                    .map(|snippet_end_rel| (content_start, content_start + snippet_end_rel))
            })
            .map(|(start, end)| strip_html_tags(&window[start..end]))
            .unwrap_or_default();

        results.push(serde_json::json!({
            "title": title,
            "url": url,
            "snippet": snippet,
            "source": "google_scrape"
        }));
    }

    results
}

fn google_web_search_scrape_tool(
    _context: ToolExecutionContext,
    arguments: serde_json::Value,
    _app_handle: AppHandle,
    _workspace_root: PathBuf,
) -> BoxFuture<'static, Result<serde_json::Value, String>> {
    Box::pin(async move {
        if !parse_env_flag(
            std::env::var(ORCHESTRATOR_GOOGLE_SCRAPE_TOOL_ENV)
                .ok()
                .as_deref(),
        ) {
            return Err(format!(
                "google_web_search_scrape is disabled. Set {}=1 for dev/test use.",
                ORCHESTRATOR_GOOGLE_SCRAPE_TOOL_ENV
            ));
        }

        let args: GoogleWebSearchScrapeArgs =
            parse_tool_args("google_web_search_scrape", arguments)?;
        let query = args.query.trim().to_string();
        if query.is_empty() {
            return Err("query cannot be empty".to_string());
        }
        let num_results = args.num_results.clamp(1, TOOL_MAX_GOOGLE_RESULTS_LIMIT);

        let mut query_params = vec![
            ("q", query.clone()),
            ("num", num_results.to_string()),
            ("hl", args.language.unwrap_or_else(|| "en".to_string())),
            ("pws", "0".to_string()),
            ("safe", "off".to_string()),
        ];
        query_params.shrink_to_fit();

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(TOOL_WEB_TIMEOUT_SECONDS))
            .user_agent(
                "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 \
                 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36",
            )
            .build()
            .map_err(|err| format!("Failed to build web search client: {}", err))?;

        let response = client
            .get("https://www.google.com/search")
            .query(&query_params)
            .send()
            .await
            .map_err(|err| format!("Failed to execute Google scrape request: {}", err))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(format!(
                "Google scrape request failed with status {}: {}",
                status.as_u16(),
                body.chars().take(200).collect::<String>()
            ));
        }

        let html = response
            .text()
            .await
            .map_err(|err| format!("Failed to read Google scrape response: {}", err))?;
        let results = parse_google_search_results(&html, num_results);

        Ok(serde_json::json!({
            "query": query,
            "count": results.len(),
            "results": results,
            "warning": "Dev/test scraper: unstable markup and ToS-sensitive."
        }))
    })
}

// =============================================================================
// Request/Response Types
// =============================================================================

/// Request to send a chat message to an LLM.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendChatRequest {
    /// Conversation ID
    pub conversation_id: String,
    /// Thread ID (optional)
    pub thread_id: Option<String>,
    /// The user's message content
    pub content: String,
    /// Model ID to use (e.g., "openai-gpt-4"). If not provided, uses the 'code' operation default.
    #[serde(default)]
    pub model_id: Option<String>,
    /// System prompt (optional)
    pub system_prompt: Option<String>,
    /// Temperature (optional)
    pub temperature: Option<f32>,
    /// Max tokens (optional)
    pub max_tokens: Option<u32>,
}

/// Response from a non-streaming chat request.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendChatResponse {
    /// The generated response content
    pub content: String,
    /// The model used
    pub model: String,
    /// Usage statistics
    pub usage: Option<crate::llm::UsageStats>,
    /// The saved assistant message
    pub message: ChatMessage,
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Resolves the model ID to use for a chat request.
/// If model_id is provided, uses it directly. Otherwise, resolves from 'code' operation.
async fn resolve_model_id(
    model_id: Option<String>,
    config_store: &ConfigStore,
) -> Result<String, String> {
    if let Some(id) = model_id {
        return Ok(id);
    }

    // Resolve from 'code' operation default
    let model = get_code_operation_model(config_store)?;
    Ok(model.id)
}

/// Gets the API key for a provider from the database.
fn get_api_key_for_provider(
    provider_id: &str,
    config_store: &ConfigStore,
) -> Result<String, String> {
    config_store.retrieve_api_key(provider_id).map_err(|e| {
        format!(
            "API key not found for provider '{}'. Please configure it first. Error: {}",
            provider_id, e
        )
    })
}

/// Creates an LLM client for the given provider.
fn create_client_for_provider(
    api_type: ApiType,
    api_key: String,
    base_url: Option<String>,
) -> Box<dyn LlmClient> {
    create_client(api_type, api_key, base_url)
}

/// Executes one non-streaming chat request with bounded retries for transient errors.
async fn run_chat_with_retry(
    client: &dyn LlmClient,
    request: &ChatRequest,
) -> Result<ChatResponse, LlmError> {
    let mut attempt = 0usize;
    loop {
        match client.chat(request.clone()).await {
            Ok(response) => return Ok(response),
            Err(err) => {
                let can_retry = err.is_retryable() && attempt < ORCHESTRATOR_LLM_MAX_RETRIES;
                if !can_retry {
                    return Err(err);
                }

                let delay_ms = ORCHESTRATOR_LLM_RETRY_BASE_MS * (1u64 << attempt);
                warn!(
                    "[LLM] Transient LLM error (attempt {}/{}), retrying in {}ms: {}",
                    attempt + 1,
                    ORCHESTRATOR_LLM_MAX_RETRIES + 1,
                    delay_ms,
                    err
                );
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                attempt += 1;
            }
        }
    }
}

/// Converts chat messages from storage to LLM format.
fn convert_messages(messages: &[ChatMessage]) -> Vec<Message> {
    messages
        .iter()
        .filter_map(|msg| {
            let role = match msg.role {
                MessageRole::User => crate::llm::MessageRole::User,
                MessageRole::Assistant => crate::llm::MessageRole::Assistant,
                MessageRole::System => crate::llm::MessageRole::System,
            };
            Some(Message {
                role,
                content: crate::llm::MessageContent::Text(msg.content.clone()),
                tool_calls: Vec::new(),
                tool_result: None,
            })
        })
        .collect()
}

// =============================================================================
// Feedback Context Injection
// =============================================================================

/// System prompt explaining the feedback system.
/// This is injected when feedback exists for the conversation.
const FEEDBACK_SYSTEM_PROMPT: &str = r#"The user may provide feedback (like/dislike) on earlier assistant responses. A feedback summary section will be provided, including stable IDs for reference. Use that feedback to improve your response quality and avoid repeating disliked patterns."#;

/// Maximum number of feedback items to include in context.
const FEEDBACK_LIMIT: usize = 5;

/// Builds a feedback summary string referencing message IDs.
/// Returns None if no feedback messages are provided.
fn build_feedback_summary(feedback_messages: &[(ChatMessage, MessageFeedback)]) -> Option<String> {
    if feedback_messages.is_empty() {
        return None;
    }

    let feedback_lines: Vec<String> = feedback_messages
        .iter()
        .map(|(msg, feedback)| {
            let short_id = &msg.id[..8.min(msg.id.len())];
            let action = match feedback {
                MessageFeedback::Like => "LIKED",
                MessageFeedback::Dislike => "DISLIKED",
            };
            format!("- [ID:{}] {} by user", short_id, action)
        })
        .collect();

    Some(format!(
        "Feedback on previous messages:\n{}",
        feedback_lines.join("\n")
    ))
}

/// Builds the combined system prompt including feedback context.
/// Returns the original system prompt if no feedback exists.
fn build_system_prompt_with_feedback(
    original_prompt: &str,
    feedback_summary: Option<&str>,
) -> String {
    match feedback_summary {
        Some(summary) => {
            if original_prompt.is_empty() {
                format!(
                    "{}\n\n{}\n\n{}",
                    FEEDBACK_SYSTEM_PROMPT, summary, "Now continue the conversation below:"
                )
            } else {
                format!(
                    "{}\n\n{}\n\n{}\n\n{}",
                    original_prompt,
                    FEEDBACK_SYSTEM_PROMPT,
                    summary,
                    "Now continue the conversation below:"
                )
            }
        }
        None => original_prompt.to_string(),
    }
}

// =============================================================================
// Chat Commands
// =============================================================================

/// Final output from one orchestrated bounded loop execution.
struct OrchestratedLoopOutput {
    content: String,
    model: String,
    usage: Option<crate::llm::UsageStats>,
}

/// Runs the bounded orchestrated conversation loop and returns the final model response.
async fn run_orchestrated_loop(
    request: &SendChatRequest,
    app_handle: &AppHandle,
    tool_executor: &dyn ToolExecutor,
    include_thread_history: bool,
    persist_user_message: bool,
) -> Result<OrchestratedLoopOutput, String> {
    // Resolve model/provider as in the legacy path.
    let llm_state = app_handle.state::<LlmState>();
    let config_store = llm_state.config_store.lock().await;
    let model_id = resolve_model_id(request.model_id.clone(), &config_store).await?;

    let model = match config_store.get_model(&model_id) {
        Ok(Some(m)) => m,
        Ok(None) => {
            if let Some(dash_pos) = model_id.find('-') {
                let provider_id = &model_id[..dash_pos];
                let model_name = &model_id[dash_pos + 1..];
                config_store
                    .get_model_by_name(provider_id, model_name)
                    .map_err(|e| e.to_string())?
                    .ok_or_else(|| format!("Model '{}' not found", model_id))?
            } else {
                return Err(format!("Model '{}' not found", model_id));
            }
        }
        Err(e) => return Err(e.to_string()),
    };

    let provider = config_store
        .get_provider(&model.provider_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Provider '{}' not found", model.provider_id))?;

    let api_key = get_api_key_for_provider(&provider.id, &config_store)?;
    drop(config_store);

    let client = create_client_for_provider(provider.api_type, api_key, provider.base_url);
    let chat_state = app_handle.state::<ChatState>();
    let chat_store = chat_state.store.lock().await;

    let history = if include_thread_history {
        if let Some(thread_id) = &request.thread_id {
            chat_store
                .load_thread_messages(thread_id)
                .map_err(|e| e.to_string())?
        } else {
            chat_store
                .load_messages(&request.conversation_id)
                .map_err(|e| e.to_string())?
        }
    } else {
        chat_store
            .load_messages(&request.conversation_id)
            .map_err(|e| e.to_string())?
    };

    let user_message = ChatMessage::user(&request.conversation_id, &request.content);
    if persist_user_message {
        chat_store
            .add_message(&user_message)
            .map_err(|e| e.to_string())?;
    }

    drop(chat_store);

    let mut llm_context = convert_messages(&history);
    llm_context.push(Message::user(request.content.clone()));

    let system_prompt = request.system_prompt.clone().unwrap_or_default();
    let temperature = request.temperature.unwrap_or(0.7);
    let max_tokens = request.max_tokens.unwrap_or(4096);
    let tool_definitions = tool_executor.definitions();

    for step in 1..=ORCHESTRATOR_MAX_STEPS {
        info!(
            "[LLM] Orchestrator step {}/{}",
            step, ORCHESTRATOR_MAX_STEPS
        );

        let mut chat_request = ChatRequest::new(&model.name, llm_context.clone())
            .with_system(&system_prompt)
            .with_temperature(temperature)
            .with_max_tokens(max_tokens);
        if !tool_definitions.is_empty() {
            chat_request = chat_request.with_tools(tool_definitions.clone());
        }

        let response: ChatResponse = run_chat_with_retry(client.as_ref(), &chat_request)
            .await
            .map_err(|e| {
                error!(
                    "send_chat_message_orchestrated_non_stream: LLM error: {}",
                    e
                );
                e.to_string()
            })?;

        let tool_calls = response.tool_calls.clone();
        if tool_calls.is_empty() {
            if !tool_definitions.is_empty() && is_textual_tool_plan(&response.content) {
                return Err(
                    "Model returned a textual tool plan instead of native tool calls. \
Enable a model/provider with native tool-calling support for orchestrated execution."
                        .to_string(),
                );
            }
            return Ok(OrchestratedLoopOutput {
                content: response.content,
                model: response.model,
                usage: response.usage,
            });
        }

        // Preserve assistant tool-call turn in structured form.
        llm_context.push(Message::assistant_with_tool_calls(
            response.content,
            tool_calls.clone(),
        ));

        let mut tool_results = Vec::with_capacity(tool_calls.len());
        for call in tool_calls {
            let result = tool_executor.execute(call, app_handle.clone()).await;
            tool_results.push(result);
        }
        append_tool_results_to_llm_context(&mut llm_context, &tool_results);
    }

    Err(format!(
        "Orchestrator exceeded max steps ({}) without final response",
        ORCHESTRATOR_MAX_STEPS
    ))
}

/// Orchestrated non-streaming execution skeleton with bounded turn loop.
async fn send_chat_message_orchestrated_non_stream(
    request: SendChatRequest,
    app_handle: AppHandle,
    tool_executor: &dyn ToolExecutor,
) -> Result<SendChatResponse, String> {
    info!(
        "send_chat_message_orchestrated_non_stream: conversation_id={}, model_id={:?}",
        request.conversation_id, request.model_id
    );

    let final_output =
        run_orchestrated_loop(&request, &app_handle, tool_executor, false, true).await?;

    let assistant_message = ChatMessage::assistant(&request.conversation_id, &final_output.content);
    let chat_state = app_handle.state::<ChatState>();
    let chat_store = chat_state.store.lock().await;
    chat_store
        .add_message(&assistant_message)
        .map_err(|e| e.to_string())?;

    Ok(SendChatResponse {
        content: final_output.content,
        model: final_output.model,
        usage: final_output.usage,
        message: assistant_message,
    })
}

/// Legacy implementation for non-streaming chat requests.
async fn send_chat_message_legacy(
    request: SendChatRequest,
    app_handle: AppHandle,
) -> Result<SendChatResponse, String> {
    info!(
        "send_chat_message: conversation_id={}, model_id={:?}",
        request.conversation_id, request.model_id
    );

    // Get config store
    let llm_state = app_handle.state::<LlmState>();
    let config_store = llm_state.config_store.lock().await;

    // Resolve model ID (use code operation default if not provided)
    let model_id = resolve_model_id(request.model_id, &config_store).await?;

    // Get model info - try by ID first, then by name if ID lookup fails
    let model = match config_store.get_model(&model_id) {
        Ok(Some(m)) => m,
        Ok(None) => {
            // Try to parse provider_id and model name from the ID
            info!(
                "Model '{}' not found by ID, trying to resolve by name",
                model_id
            );

            if let Some(dash_pos) = model_id.find('-') {
                let provider_id = &model_id[..dash_pos];
                let model_name = &model_id[dash_pos + 1..];

                config_store
                    .get_model_by_name(provider_id, model_name)
                    .map_err(|e| e.to_string())?
                    .ok_or_else(|| format!("Model '{}' not found", model_id))?
            } else {
                return Err(format!("Model '{}' not found", model_id));
            }
        }
        Err(e) => return Err(e.to_string()),
    };

    // Get provider info
    let provider = config_store
        .get_provider(&model.provider_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Provider '{}' not found", model.provider_id))?;

    // Get API key
    let api_key = get_api_key_for_provider(&provider.id, &config_store)?;

    drop(config_store); // Release lock before async operations

    // Create client
    let client = create_client_for_provider(provider.api_type, api_key, provider.base_url);

    // Get chat state and load conversation history
    let chat_state = app_handle.state::<ChatState>();
    let chat_store = chat_state.store.lock().await;

    let history = chat_store
        .load_messages(&request.conversation_id)
        .map_err(|e| e.to_string())?;

    // Add the new user message to history for context
    let user_message = ChatMessage::user(&request.conversation_id, &request.content);
    let mut context_messages = history;
    context_messages.push(user_message.clone());

    // Save user message
    chat_store
        .add_message(&user_message)
        .map_err(|e| e.to_string())?;

    drop(chat_store); // Release lock before LLM call

    // Convert messages for LLM
    let llm_messages = convert_messages(&context_messages);

    // Create chat request
    let chat_request = ChatRequest::new(&model.name, llm_messages)
        .with_system(request.system_prompt.unwrap_or_default())
        .with_temperature(request.temperature.unwrap_or(0.7))
        .with_max_tokens(request.max_tokens.unwrap_or(4096));

    // Make LLM call
    let response: ChatResponse = client.chat(chat_request).await.map_err(|e| {
        error!("send_chat_message: LLM error: {}", e);
        e.to_string()
    })?;

    info!(
        "send_chat_message: Got response, content length={}",
        response.content.len()
    );

    // Save assistant message
    let assistant_message = ChatMessage::assistant(&request.conversation_id, &response.content);
    let chat_store = chat_state.store.lock().await;
    chat_store
        .add_message(&assistant_message)
        .map_err(|e| e.to_string())?;

    Ok(SendChatResponse {
        content: response.content,
        model: response.model,
        usage: response.usage,
        message: assistant_message,
    })
}

/// Sends a chat message and returns the complete response (non-streaming).
#[tauri::command]
pub async fn send_chat_message(
    request: SendChatRequest,
    app_handle: AppHandle,
) -> Result<SendChatResponse, String> {
    let runner = app_handle.state::<LlmState>().runner.clone();
    runner.send_chat_message(request, app_handle).await
}

/// Sends a chat message with streaming response.
/// Events are emitted: llm:started, llm:chunk, llm:complete, llm:error
///
/// Note: The frontend is responsible for saving the user message BEFORE calling this.
/// This function only saves the assistant message after streaming completes.
async fn send_chat_message_streaming_orchestrated(
    request: SendChatRequest,
    app_handle: AppHandle,
) -> Result<String, String> {
    let request_id = uuid::Uuid::new_v4().to_string();
    info!(
        "[LLM] Starting orchestrated streaming request: request_id={}, conversation_id={}, thread_id={:?}",
        request_id, request.conversation_id, request.thread_id
    );

    let llm_state = app_handle.state::<LlmState>();
    let active_requests = llm_state.active_requests.clone();

    let app_handle_clone = app_handle.clone();
    let request_id_clone = request_id.clone();
    let request_for_task = request.clone();
    let model_for_started = request
        .model_id
        .clone()
        .unwrap_or_else(|| "orchestrated".to_string());

    let active_requests_for_task = active_requests.clone();
    let request_id_for_cleanup = request_id.clone();

    let handle = tokio::spawn(async move {
        let started_event = LlmEvent::Started {
            request_id: request_id_clone.clone(),
            model: model_for_started,
        };
        if let Err(e) = app_handle_clone.emit("llm:event", &started_event) {
            error!("[LLM] Failed to emit orchestrated started event: {}", e);
        }

        let tool_executor = select_tool_executor_from_env(&request_for_task.conversation_id);
        match run_orchestrated_loop(
            &request_for_task,
            &app_handle_clone,
            tool_executor.as_ref(),
            true,
            false,
        )
        .await
        {
            Ok(final_output) => {
                if !final_output.content.is_empty() {
                    let chunk_event = LlmEvent::Chunk {
                        request_id: request_id_clone.clone(),
                        delta: final_output.content.clone(),
                    };
                    if let Err(e) = app_handle_clone.emit("llm:event", &chunk_event) {
                        error!("[LLM] Failed to emit orchestrated chunk event: {}", e);
                    }
                }

                let complete_event = LlmEvent::Complete {
                    request_id: request_id_clone.clone(),
                    content: final_output.content.clone(),
                    usage: final_output.usage.clone(),
                    finish_reason: Some("stop".to_string()),
                };
                if let Err(e) = app_handle_clone.emit("llm:event", &complete_event) {
                    error!("[LLM] Failed to emit orchestrated complete event: {}", e);
                }

                if !final_output.content.is_empty() {
                    let mut assistant_message = ChatMessage::assistant(
                        &request_for_task.conversation_id,
                        &final_output.content,
                    );
                    if let Some(thread_id) = &request_for_task.thread_id {
                        assistant_message = assistant_message.with_thread(thread_id);
                    }
                    assistant_message.metadata = Some(crate::nlp::chat::MessageMetadata {
                        model: Some(final_output.model),
                        tokens: final_output.usage.as_ref().map(|u| u.total_tokens as usize),
                        ..Default::default()
                    });

                    let chat_state = app_handle_clone.state::<ChatState>();
                    let chat_store = chat_state.store.lock().await;
                    if let Err(e) = chat_store.add_message(&assistant_message) {
                        error!("[LLM] Failed to save orchestrated assistant message: {}", e);
                    }
                }
            }
            Err(err) => {
                let event = LlmEvent::Error {
                    request_id: request_id_clone.clone(),
                    error: err,
                    retryable: false,
                };
                if let Err(e) = app_handle_clone.emit("llm:event", &event) {
                    error!("[LLM] Failed to emit orchestrated error event: {}", e);
                }
            }
        }

        let mut active = active_requests_for_task.lock().await;
        active.remove(&request_id_for_cleanup);
    });

    {
        let mut active = active_requests.lock().await;
        if let Some(previous) = active.insert(request_id.clone(), handle) {
            warn!(
                "[LLM] Replaced existing streaming task for request_id={}, aborting previous task",
                request_id
            );
            previous.abort();
        }
    }

    Ok(request_id)
}

async fn send_chat_message_streaming_legacy(
    request: SendChatRequest,
    app_handle: AppHandle,
) -> Result<String, String> {
    // Generate request ID
    let request_id = uuid::Uuid::new_v4().to_string();
    info!(
        "[LLM] Starting streaming request: request_id={}, conversation_id={}, thread_id={:?}",
        request_id, request.conversation_id, request.thread_id
    );

    // Get config store
    let llm_state = app_handle.state::<LlmState>();
    let active_requests = llm_state.active_requests.clone();
    let config_store = llm_state.config_store.lock().await;

    // Resolve model ID (use code operation default if not provided)
    let model_id = match resolve_model_id(request.model_id, &config_store).await {
        Ok(id) => id,
        Err(e) => {
            error!("[LLM] Failed to resolve model_id: {}", e);
            return Err(e);
        }
    };

    // Get model info - try by ID first, then by name if ID lookup fails
    let model = match config_store.get_model(&model_id) {
        Ok(Some(m)) => m,
        Ok(None) => {
            // Try to parse provider_id and model name from the ID
            // ID format is typically: "provider_id-model-name"
            info!(
                "[LLM] Model '{}' not found by ID, trying to resolve by name",
                model_id
            );

            // Find the first dash to split provider_id from model name
            if let Some(dash_pos) = model_id.find('-') {
                let provider_id = &model_id[..dash_pos];
                let model_name = &model_id[dash_pos + 1..];

                config_store
                    .get_model_by_name(provider_id, model_name)
                    .map_err(|e| e.to_string())?
                    .ok_or_else(|| {
                        format!(
                            "Model '{}' not found (also tried name lookup: provider={}, name={})",
                            model_id, provider_id, model_name
                        )
                    })?
            } else {
                return Err(format!("Model '{}' not found", model_id));
            }
        }
        Err(e) => return Err(e.to_string()),
    };

    // Get provider info
    let provider = config_store
        .get_provider(&model.provider_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Provider '{}' not found", model.provider_id))?;

    // Get API key
    let api_key = match get_api_key_for_provider(&provider.id, &config_store) {
        Ok(key) => key,
        Err(e) => {
            error!("[LLM] Failed to get API key: {}", e);
            return Err(e);
        }
    };

    drop(config_store); // Release lock before async operations

    // Create client
    let client = create_client_for_provider(provider.api_type, api_key, provider.base_url);

    // Get chat state and load conversation history
    let chat_state = app_handle.state::<ChatState>();
    let chat_store = chat_state.store.lock().await;

    // Load thread messages if thread_id is provided, otherwise load all conversation messages
    let history = if let Some(ref thread_id) = request.thread_id {
        chat_store
            .load_thread_messages(thread_id)
            .map_err(|e| e.to_string())?
    } else {
        chat_store
            .load_messages(&request.conversation_id)
            .map_err(|e| e.to_string())?
    };

    // Load feedback for the current thread (only if thread_id exists)
    let feedback_messages = if let Some(ref thread_id) = request.thread_id {
        chat_store
            .load_thread_feedback(thread_id, FEEDBACK_LIMIT)
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    // Build feedback summary if feedback exists
    let feedback_summary = build_feedback_summary(&feedback_messages);
    let has_feedback = feedback_summary.is_some();

    if has_feedback {
        info!(
            "[LLM] Injecting feedback context: {} feedback items for thread_id={:?}",
            feedback_messages.len(),
            request.thread_id
        );
    }

    // DO NOT save user message here - frontend already saved it
    // Just add to context for LLM
    let user_context_message = ChatMessage::user(&request.conversation_id, &request.content);
    let mut context_messages = history;
    context_messages.push(user_context_message);

    drop(chat_store); // Release lock before streaming

    // Convert messages for LLM.
    // Feedback context is provided via system prompt only; message bodies are not rewritten.
    let llm_messages = convert_messages(&context_messages);

    // Build system prompt with feedback context if available
    let system_prompt = build_system_prompt_with_feedback(
        &request.system_prompt.unwrap_or_default(),
        feedback_summary.as_deref(),
    );

    // Create chat request with streaming enabled
    let chat_request = ChatRequest::new(&model.name, llm_messages)
        .with_system(&system_prompt)
        .with_temperature(request.temperature.unwrap_or(0.7))
        .with_max_tokens(request.max_tokens.unwrap_or(4096))
        .with_stream();

    // Get app handle for async task
    let app_handle_clone = app_handle.clone();
    let request_id_clone = request_id.clone();
    let conversation_id = request.conversation_id.clone();
    let thread_id = request.thread_id.clone();
    let model_name_for_log = model.name.clone();

    // Spawn streaming task
    let active_requests_for_task = active_requests.clone();
    let request_id_for_cleanup = request_id.clone();

    let handle = tokio::spawn(async move {
        let stream = client.chat_stream(chat_request);
        let events = crate::llm::stream_to_events(
            request_id_clone.clone(),
            model_name_for_log.clone(),
            stream,
        );

        #[allow(clippy::expect_used)]
        let mut events = std::pin::pin!(events);
        let mut final_content = String::new();
        let mut final_usage = None;

        use futures::StreamExt;

        while let Some(event) = events.next().await {
            match &event {
                LlmEvent::Chunk { delta, .. } => {
                    final_content.push_str(delta);
                }
                LlmEvent::Complete { content, usage, .. } => {
                    final_content = content.clone();
                    final_usage = usage.clone();
                }
                LlmEvent::Error { error, .. } => {
                    error!("[LLM] Stream error: {}", error);
                }
                _ => {}
            }

            // Emit event to frontend
            if let Err(e) = app_handle_clone.emit("llm:event", &event) {
                error!("[LLM] Failed to emit event: {}", e);
            }
        }

        info!(
            "[LLM] Stream completed: request_id={}, content_length={}",
            request_id_clone,
            final_content.len()
        );

        // Save assistant message after streaming completes
        if !final_content.is_empty() {
            let mut assistant_message = ChatMessage::assistant(&conversation_id, &final_content);

            // Set thread_id if provided - this is critical for proper message organization
            if let Some(tid) = thread_id {
                assistant_message = assistant_message.with_thread(&tid);
            }

            // Add metadata about model and tokens
            assistant_message.metadata = Some(crate::nlp::chat::MessageMetadata {
                model: Some(model_name_for_log.clone()),
                tokens: final_usage.as_ref().map(|u| u.total_tokens as usize),
                ..Default::default()
            });

            let chat_state = app_handle_clone.state::<ChatState>();
            let chat_store = chat_state.store.lock().await;
            if let Err(e) = chat_store.add_message(&assistant_message) {
                error!("[LLM] Failed to save assistant message: {}", e);
            }
        }

        let mut active = active_requests_for_task.lock().await;
        active.remove(&request_id_for_cleanup);
    });

    {
        let mut active = active_requests.lock().await;
        if let Some(previous) = active.insert(request_id.clone(), handle) {
            warn!(
                "[LLM] Replaced existing streaming task for request_id={}, aborting previous task",
                request_id
            );
            previous.abort();
        }
    }

    Ok(request_id)
}

/// Sends a chat message with streaming response.
/// Events are emitted: llm:started, llm:chunk, llm:complete, llm:error
#[tauri::command]
pub async fn send_chat_message_streaming(
    request: SendChatRequest,
    app_handle: AppHandle,
) -> Result<String, String> {
    let runner = app_handle.state::<LlmState>().runner.clone();
    runner
        .send_chat_message_streaming(request, app_handle)
        .await
}

/// Cancels an in-progress streaming request.
#[tauri::command]
pub async fn cancel_chat_request(request_id: String, app_handle: AppHandle) -> Result<(), String> {
    let llm_state = app_handle.state::<LlmState>();
    let mut active_requests = llm_state.active_requests.lock().await;

    if let Some(handle) = active_requests.remove(&request_id) {
        info!("[LLM] Cancelling active stream request_id={}", request_id);
        handle.abort();
    } else {
        info!(
            "[LLM] cancel_chat_request: no active stream found for request_id={}",
            request_id
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nlp::chat::MessageFeedback;
    use std::fs as std_fs;

    #[test]
    fn test_llm_state_creation() {
        let state = LlmState::in_memory();
        assert!(state.is_ok());
    }

    #[test]
    fn test_parse_env_flag_truthy_values() {
        for value in ["1", "true", "TRUE", "yes", "on", " On "] {
            assert!(parse_env_flag(Some(value)), "expected truthy for {}", value);
        }
    }

    #[test]
    fn test_parse_env_flag_falsy_values() {
        for value in ["0", "false", "no", "off", "", "unexpected"] {
            assert!(!parse_env_flag(Some(value)), "expected falsy for {}", value);
        }
        assert!(!parse_env_flag(None));
    }

    #[test]
    fn test_tool_registry_builds_expected_tools() {
        let registry = ToolRegistry {
            workspace_root: PathBuf::from("/tmp"),
            tools: ToolRegistry::build_builtin_tools(),
        };
        let mut names = registry
            .definitions()
            .into_iter()
            .map(|tool| tool.name)
            .collect::<Vec<_>>();
        names.sort();

        assert!(names.contains(&"get_app_info".to_string()));
        assert!(names.contains(&"list_workspace_files".to_string()));
        assert!(names.contains(&"read_workspace_file".to_string()));
        assert!(names.contains(&"search_conversation_messages".to_string()));
    }

    #[test]
    fn test_resolve_workspace_path_rejects_escape_attempts() {
        let workspace_root =
            std::env::temp_dir().join(format!("coder-tool-test-{}", uuid::Uuid::new_v4()));
        std_fs::create_dir_all(workspace_root.join("nested"))
            .expect("failed to create test workspace");
        std_fs::write(workspace_root.join("nested/file.txt"), "hello")
            .expect("failed to write test file");

        let canonical_root = workspace_root
            .canonicalize()
            .expect("failed to canonicalize root");
        let allowed = resolve_workspace_path(&canonical_root, "nested/file.txt")
            .expect("expected nested file to resolve");
        assert!(allowed.starts_with(&canonical_root));

        let escaped = resolve_workspace_path(&canonical_root, "../");
        assert!(escaped.is_err());

        let _ = std_fs::remove_dir_all(&workspace_root);
    }

    #[test]
    fn test_parse_google_search_results_extracts_items() {
        let html = r#"
            <html><body>
                <a href="/url?q=https%3A%2F%2Fexample.com%2Fdocs&sa=U&ved=2ahUKE">
                  <h3>Example <em>Docs</em></h3>
                  <div class="VwiC3b yXK7lf">Developer docs for examples.</div>
                </a>
                <a href="/url?q=https%3A%2F%2Fexample.org%2Fblog&sa=U&ved=2ahUKE">
                  <h3>Example Org Blog</h3>
                  <div class="VwiC3b yXK7lf">Release notes and updates.</div>
                </a>
            </body></html>
        "#;

        let results = parse_google_search_results(html, 5);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0]["url"], "https://example.com/docs");
        assert_eq!(results[0]["title"], "Example Docs");
        assert_eq!(results[0]["snippet"], "Developer docs for examples.");
    }

    #[test]
    fn test_extract_google_result_url_invalid_href() {
        assert!(extract_google_result_url("not-a-query").is_none());
        assert!(extract_google_result_url("q=javascript:void(0)").is_none());
    }

    #[test]
    fn test_extract_google_result_url_valid_href() {
        let href = "https%3A%2F%2Fexample.com%2Fdocs&sa=U&ved=2ahUKE";
        assert_eq!(
            extract_google_result_url(href).as_deref(),
            Some("https://example.com/docs")
        );
    }

    #[test]
    fn test_append_tool_results_to_llm_context() {
        let mut context = vec![Message::user("Hi")];
        let results = vec![OrchestratorToolResult {
            call_id: "c1".to_string(),
            name: "search_code".to_string(),
            output: serde_json::json!({"ok": true}),
            is_error: false,
        }];
        append_tool_results_to_llm_context(&mut context, &results);
        assert_eq!(context.len(), 2);
        assert_eq!(context[1].role, crate::llm::MessageRole::Tool);
        let tool_result = context[1]
            .tool_result
            .as_ref()
            .expect("tool result should be present");
        assert_eq!(tool_result.call_id, "c1");
        assert_eq!(tool_result.output, serde_json::json!({"ok": true}));
        assert!(!tool_result.is_error);
    }

    #[test]
    fn test_is_textual_tool_plan_array_shape() {
        let content = r#"
        [
          {
            "name": "google_web_search_scrape",
            "arguments": "{\"query\":\"tokio joinhandle\",\"num_results\":5}"
          }
        ]
        "#;
        assert!(is_textual_tool_plan(content));
    }

    #[test]
    fn test_is_textual_tool_plan_wrapper_shape() {
        let content = r#"
        {
          "tool_calls": [
            {
              "name": "list_workspace_files",
              "arguments": {"path":"."}
            }
          ]
        }
        "#;
        assert!(is_textual_tool_plan(content));
    }

    #[test]
    fn test_is_textual_tool_plan_false_for_regular_text() {
        assert!(!is_textual_tool_plan("Here is your answer."));
    }

    #[test]
    fn test_convert_messages() {
        let messages = vec![
            ChatMessage::user("conv-1", "Hello"),
            ChatMessage::assistant("conv-1", "Hi there!"),
        ];

        let llm_messages = convert_messages(&messages);

        assert_eq!(llm_messages.len(), 2);
        assert_eq!(llm_messages[0].role, crate::llm::MessageRole::User);
        assert_eq!(llm_messages[1].role, crate::llm::MessageRole::Assistant);
    }

    #[test]
    fn test_build_feedback_summary_empty() {
        let feedback: Vec<(ChatMessage, MessageFeedback)> = vec![];
        let summary = build_feedback_summary(&feedback);
        assert!(summary.is_none());
    }

    #[test]
    fn test_build_feedback_summary_with_likes() {
        let msg1 = ChatMessage::assistant("conv-1", "Response 1");
        let msg2 = ChatMessage::assistant("conv-1", "Response 2");

        let feedback = vec![
            (msg1, MessageFeedback::Like),
            (msg2, MessageFeedback::Dislike),
        ];

        let summary = build_feedback_summary(&feedback);
        assert!(summary.is_some());

        let summary = summary.unwrap();
        assert!(summary.contains("Feedback on previous messages:"));
        assert!(summary.contains("LIKED"));
        assert!(summary.contains("DISLIKED"));
        assert!(summary.contains("[ID:"));
    }

    #[test]
    fn test_build_system_prompt_with_feedback() {
        let original = "You are a helpful assistant.";
        let feedback_summary =
            Some("Feedback on previous messages:\n- [ID:abc12345] LIKED by user");

        let combined = build_system_prompt_with_feedback(original, feedback_summary);

        assert!(combined.contains("You are a helpful assistant."));
        assert!(combined.contains("including stable IDs for reference"));
        assert!(combined.contains("Feedback on previous messages:"));
    }

    #[test]
    fn test_build_system_prompt_without_feedback() {
        let original = "You are a helpful assistant.";
        let combined = build_system_prompt_with_feedback(original, None);

        // Should return original when no feedback
        assert_eq!(combined, original);
    }

    #[test]
    fn test_build_system_prompt_with_feedback_and_empty_original() {
        let feedback_summary =
            Some("Feedback on previous messages:\n- [ID:abc12345] DISLIKED by user");

        let combined = build_system_prompt_with_feedback("", feedback_summary);

        assert!(combined.starts_with(FEEDBACK_SYSTEM_PROMPT));
        assert!(combined.contains("Feedback on previous messages:"));
        assert!(combined.contains("Now continue the conversation below:"));
    }

    #[test]
    fn test_build_feedback_summary_uses_stable_short_ids() {
        let msg = ChatMessage::assistant("conv-1", "Response");
        let full_id = msg.id.clone();

        let summary = build_feedback_summary(&[(msg, MessageFeedback::Like)])
            .expect("summary should exist for one feedback item");

        let short_id = &full_id[..8];
        assert!(summary.contains(&format!("[ID:{}]", short_id)));
        assert!(!summary.contains(&format!("[ID:{}]", full_id)));
    }
}
