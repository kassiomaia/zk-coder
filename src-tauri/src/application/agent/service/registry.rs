use crate::core::agent::entities::AgentDomainError;
use crate::core::agent::repositories::RepositoryResult;
use crate::core::agent::types::Agent;

use super::AgentServiceImpl;

impl AgentServiceImpl {
    pub(super) fn list_agents_impl(&self) -> RepositoryResult<Vec<Agent>> {
        Ok(self.block_on(self.registry.list()))
    }

    pub(super) fn get_agent_impl(&self, agent_id: &str) -> RepositoryResult<Agent> {
        self.block_on(self.registry.get(&self.to_agents_agent_id(agent_id)))
            .ok_or_else(|| AgentDomainError::AgentNotFound {
                id: agent_id.to_string(),
            })
    }
}
