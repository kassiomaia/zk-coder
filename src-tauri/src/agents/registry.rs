//! Agent registry for managing agent configurations.
//!
//! Contains built-in agents and supports runtime registration.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use tokio::sync::RwLock;

use super::types::{Agent, AgentId, TrustMode};

/// Registry of available agent configurations.
pub struct AgentRegistry {
    agents: Arc<RwLock<HashMap<AgentId, Agent>>>,
}

impl AgentRegistry {
    /// Create a new registry with built-in agents.
    pub fn new() -> Self {
        let mut agents = HashMap::new();

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
    /// Built-in Code Agent configuration.
    pub fn code_agent() -> Self {
        let now = Utc::now();
        Self {
            id: AgentId::new("code-agent"),
            name: "Code Agent".to_string(),
            description: "Assists with code reading, writing, and execution".to_string(),
            system_prompt: Self::code_agent_system_prompt().to_string(),
            allowed_tools: vec![
                "fs_read_file".to_string(),
                "fs_write_file".to_string(),
                "fs_list_files".to_string(),
                "search_grep".to_string(),
                "system_execute".to_string(),
                "agent_emit_status".to_string(),
                "agent_plan_subtasks".to_string(),
            ],
            default_model_id: "code".to_string(),
            trust_mode: TrustMode::Untrusted,
            created_at: now,
            updated_at: now,
        }
    }

    fn code_agent_system_prompt() -> &'static str {
        "You are a code assistant agent. You can read files, write code, search codebases, \
         and execute commands to help the user with software development tasks. \
         \
         PLANNING: For non-trivial tasks, use 'agent_plan_subtasks' early in execution to \
         break your work into logical steps. This shows the user your plan and lets them \
         track progress. Skip planning for simple tasks like reading a single file. \
         \
         COMMUNICATION: Use the 'agent_emit_status' tool periodically (every 10-30 seconds) \
         to keep the user informed of your progress. Call it with brief updates (2-6 words) \
         about what you're currently doing. Examples: 'Scanning dependencies...', \
         'Reading main.rs...', 'Planning refactoring...', 'Running tests...', \
         'Analyzing code structure...', 'Processing results...'. \
         \
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
        assert_eq!(agent.unwrap().name, "Code Agent");
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
}
