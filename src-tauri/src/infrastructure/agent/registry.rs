//! Agent registry for managing agent configurations.
//!
//! Contains built-in agents and supports runtime registration.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use tokio::sync::RwLock;

use crate::core::agent::types::{Agent, AgentId, TrustMode};

/// Registry of available agent configurations.
pub struct AgentRegistry {
    agents: Arc<RwLock<HashMap<AgentId, Agent>>>,
}

impl AgentRegistry {
    /// Create a new registry with built-in agents.
    #[must_use] 
    pub fn new() -> Self {
        let mut agents = HashMap::<AgentId, Agent>::new();

        // Register the built-in code agent
        let code_agent = Agent::code_agent();
        agents.insert(code_agent.id.clone(), code_agent);

        Self {
            agents: Arc::new(RwLock::new(agents)),
        }
    }

    /// Get an agent by ID.
    pub async fn get(&self, id: &AgentId) -> Option<Agent> {
        let agents = self.agents.read().await;
        agents.get(id).cloned()
    }

    /// List all available agents.
    pub async fn list(&self) -> Vec<Agent> {
        let agents = self.agents.read().await;
        agents.values().cloned().collect()
    }

    /// Register a custom agent.
    pub async fn register(&self, agent: Agent) {
        let mut agents = self.agents.write().await;
        agents.insert(agent.id.clone(), agent);
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl Agent {
    /// Built-in Agent configuration.
    #[must_use] 
    pub fn code_agent() -> Self {
        let now = Utc::now();
        Self {
            id: AgentId::new("code-agent"),
            name: "Agent".to_string(),
            description: "Assists with code reading, writing, and execution".to_string(),
            system_prompt: Self::code_agent_system_prompt().to_string(),
            allowed_tools: vec![
                "conversation_search_messages".to_string(),
                "fs_read_file".to_string(),
                "fs_stat_path".to_string(),
                "fs_glob_files".to_string(),
                "fs_fuzzy_find_files".to_string(),
                "fs_write_file".to_string(),
                "fs_replace_in_file".to_string(),
                "fs_apply_patch".to_string(),
                "fs_list_files".to_string(),
                "search_grep".to_string(),
                "system_get_app_info".to_string(),
                "system_execute_command".to_string(),
                "agent_emit_status".to_string(),
                "agent_narrate".to_string(),
                "agent_plan_subtasks".to_string(),
                "agent_mark_subtask_status".to_string(),
                "agent_advance_to_next_subtask".to_string(),
                "docx_start".to_string(),
                "docx_add_section".to_string(),
                "docx_list_sections".to_string(),
                "docx_finalize".to_string(),
                "pptx_start".to_string(),
                "pptx_add_slide".to_string(),
                "pptx_update_slide".to_string(),
                "pptx_delete_slide".to_string(),
                "pptx_list_slides".to_string(),
                "pptx_get_slide".to_string(),
                "pptx_search_slides".to_string(),
                "pptx_finalize".to_string(),
                "pdf_start".to_string(),
                "pdf_add_page".to_string(),
                "pdf_delete_page".to_string(),
                "pdf_list_pages".to_string(),
                "pdf_get_page".to_string(),
                "pdf_finalize".to_string(),
            ],
            default_model_id: "code".to_string(),
            trust_mode: TrustMode::Untrusted,
            created_at: now,
            updated_at: now,
        }
    }

    const fn code_agent_system_prompt() -> &'static str {
        "You are a code assistant agent. You can read files, write code, search codebases, \
         and execute commands to help the user with software development tasks.

         ## TOOL USAGE STRATEGY (KEEP CONTEXT CLEAN)

         Work progressively and choose the cheapest tool that can answer the current question.
         Avoid broad exploration, repeated reads, huge outputs, and unnecessary command execution.

         Preferred discovery order:
         1. If you already know the likely path, use `fs_stat_path` first to confirm type/size before reading.
         2. If you know part of a filename or path, use `fs_fuzzy_find_files`.
         3. If you know a pattern like `*.rs` or `src/**/mod.ts`, use `fs_glob_files`.
         4. Use `fs_list_files` only for small, scoped directories with tight depth/limit.
         5. Use `search_grep` for exact text or symbol searches, with a narrow `path` and low `max_results`.
         6. Use `conversation_search_messages` when past chat context may answer the question without re-reading files.

         Reading rules:
         - Do NOT read full files by default.
         - Read only after you have identified a promising target file.
         - Start with a small `max_bytes` and only read more when the first read is insufficient.
         - Prefer a few high-signal files over many low-value files.
         - Avoid reading generated, compiled, vendored, lock, dist, build, or other large low-signal files unless they are directly relevant.

         Editing rules:
         - Prefer `fs_replace_in_file` for small exact edits.
         - Prefer `fs_apply_patch` for localized multi-line edits.
         - Use `fs_write_file` for new files or full rewrites only when patch/replace is not appropriate.
         - Do NOT rewrite an entire file to make a small change.

         Command rules:
         - Use `system_execute_command` only when file/search tools cannot answer the question, or when you need validation such as tests/builds.
         - Keep commands scoped to the relevant directory and task.
         - Do NOT use shell commands for simple file discovery, file reading, or text search when dedicated tools can do it with less noise.

         Web/app info rules:
         - Use `system_get_app_info` only if application metadata is specifically needed.
         - Use web search only as a last resort when the workspace and conversation do not contain the answer and the tool is available.

         Stop exploring as soon as you have enough evidence to act. Summarize findings briefly instead of collecting excessive raw content.

         ## DOCUMENT GENERATION (IMPORTANT)

         When users ask you to create documents (Word/DOCX, PowerPoint/PPTX, or PDF), \
         you MUST use the internal document generation tools. DO NOT write scripts \
         (TypeScript, Python, bash, etc.) to generate documents.

         For Word documents (.docx): Use `docx_start` → `docx_add_section` → `docx_finalize`
         For PowerPoint (.pptx): Use `pptx_start` → `pptx_add_slide` → `pptx_finalize`
         For PDF (.pdf): Use `pdf_start` → `pdf_add_page` → `pdf_finalize`

         The document will be automatically saved to the artifact store and a download \
         link will appear in the chat.

         ## MANDATORY FIRST STEP: PLANNING

         For ANY non-trivial task (anything beyond reading a single file), your FIRST action \
         MUST be to call 'agent_plan_subtasks' to break down the work into 3-8 logical steps.

         This is CRITICAL: Do NOT start working until you have created a plan. The user needs \
         to see what you'll be doing.

         Simple tasks that DON'T need planning:
         - Greetings / acknowledgements / small talk (e.g., 'Hi', 'Hello', 'Thanks', 'Ok')
         - Reading a single file
         - Answering a simple question
         - Explaining how something works
         - Single-step requests that do not require code changes or command execution

         Everything else requires a plan first.

         When planning, include only the investigation needed to unblock the task. Do NOT add broad repository exploration unless it is necessary.
         If a discovery step is needed, state the narrow target and the stopping condition.

         If the user input is conversational, DO NOT create a plan. \
         Reply directly in natural language.

         ## EXECUTING YOUR PLAN

         After creating your plan, follow it step by step:

         1. Start with the first subtask (it's automatically marked as in_progress)
         2. Work on that subtask until complete
         3. Call 'agent_advance_to_next_subtask' to mark current as 'completed' and next as 'in_progress'
         4. Repeat until all subtasks are completed

         Alternatively, you can use 'agent_mark_subtask_status' with specific index and status.

         ## COMMUNICATION

         You have two communication tools:

         **agent_emit_status** — Short spinner phrases (2-6 words). Use every 10-30 seconds during active work.
         Examples: 'Scanning dependencies...', 'Reading main.rs...', 'Running tests...'

         **agent_narrate** — Natural-language updates (1-2 sentences) that persist in chat history.
         Use at meaningful moments: starting a phase, making a discovery, completing significant work, explaining a decision.
         Examples:
         - 'I'm scanning the repository structure first to understand how the codebase is organized.'
         - 'I found the main entry point in src/main.rs and the routing logic in src/router.rs. Now tracing the data flow.'
         - 'The refactoring is done — I updated 5 files and kept the existing tests passing.'
         - 'That command failed because the sandbox blocks network access. I'll use a different approach.'

         Rules:
         - Do NOT narrate every file read or small action. Focus on decisions, findings, and completions.
         - The user should be able to follow your work by reading the chat alone.
         - Always narrate: plan creation, phase transitions, important discoveries, errors and recovery, completion summary.

         ## PLAN REUSE AND REPLANNING

         Once you have created your initial plan with 'agent_plan_subtasks', \
         follow your original plan to completion. If planning is called again \
         after subtasks already exist, treat it as a no-op and continue.

         ## FINAL STEP

         When ALL subtasks are completed, provide a summary of what was accomplished.

         Always explain what you're about to do before taking action. \
         For destructive operations (file writes, deletes, command execution), \
         the user will be asked to approve before the action is executed."
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_code_agent() {
        let registry = AgentRegistry::new();
        let agent = registry.get(&AgentId::new("code-agent")).await;
        assert!(agent.is_some());
        assert_eq!(agent.unwrap().name, "Agent");
    }

    #[tokio::test]
    async fn test_list_agents() {
        let registry = AgentRegistry::new();
        let agents = registry.list().await;
        assert!(!agents.is_empty());
    }

    #[tokio::test]
    async fn test_get_nonexistent() {
        let registry = AgentRegistry::new();
        let agent = registry.get(&AgentId::new("nonexistent")).await;
        assert!(agent.is_none());
    }

    #[tokio::test]
    async fn test_code_agent_includes_fs_stat_path() {
        let registry = AgentRegistry::new();
        let agent = registry.get(&AgentId::new("code-agent")).await.unwrap();
        assert!(agent.allowed_tools.contains(&"fs_stat_path".to_string()));
    }

    #[tokio::test]
    async fn test_code_agent_includes_fs_glob_files() {
        let registry = AgentRegistry::new();
        let agent = registry.get(&AgentId::new("code-agent")).await.unwrap();
        assert!(agent.allowed_tools.contains(&"fs_glob_files".to_string()));
    }

    #[tokio::test]
    async fn test_code_agent_includes_fs_replace_in_file() {
        let registry = AgentRegistry::new();
        let agent = registry.get(&AgentId::new("code-agent")).await.unwrap();
        assert!(agent
            .allowed_tools
            .contains(&"fs_replace_in_file".to_string()));
    }

    #[tokio::test]
    async fn test_code_agent_includes_progressive_discovery_tools() {
        let registry = AgentRegistry::new();
        let agent = registry.get(&AgentId::new("code-agent")).await.unwrap();

        assert!(agent
            .allowed_tools
            .contains(&"conversation_search_messages".to_string()));
        assert!(agent
            .allowed_tools
            .contains(&"fs_fuzzy_find_files".to_string()));
        assert!(agent.allowed_tools.contains(&"fs_apply_patch".to_string()));
        assert!(agent
            .allowed_tools
            .contains(&"system_get_app_info".to_string()));
    }

    #[test]
    fn test_code_agent_prompt_includes_token_efficient_tool_guidance() {
        let prompt = Agent::code_agent().system_prompt;

        assert!(prompt.contains("TOOL USAGE STRATEGY"));
        assert!(prompt.contains("fs_stat_path"));
        assert!(prompt.contains("fs_fuzzy_find_files"));
        assert!(prompt.contains("search_grep"));
        assert!(prompt.contains("conversation_search_messages"));
        assert!(prompt.contains("fs_replace_in_file"));
        assert!(prompt.contains("fs_apply_patch"));
        assert!(prompt.contains("system_execute_command"));
        assert!(prompt.contains("Do NOT read full files by default"));
    }
}
