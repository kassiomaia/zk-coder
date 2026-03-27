use super::AgentState;

/// List all available agents.
#[tauri::command]
pub async fn list_agents(
    state: tauri::State<'_, AgentState>,
) -> Result<Vec<crate::core::agent::types::Agent>, String> {
    Ok(state.service.agent_catalog().await.agents)
}

/// Get a specific agent by ID.
#[tauri::command]
pub async fn get_agent(
    agent_id: String,
    state: tauri::State<'_, AgentState>,
) -> Result<crate::core::agent::types::Agent, String> {
    state
        .service
        .find_runtime_agent(&agent_id)
        .await
        .ok_or_else(|| format!("Agent not found: {agent_id}"))
}
