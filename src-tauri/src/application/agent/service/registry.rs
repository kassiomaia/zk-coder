use crate::core::agent::entities::{Agent as CoreAgent, AgentDomainError};
use crate::core::agent::repositories::RepositoryResult;

use super::AgentServiceImpl;

impl AgentServiceImpl {
    pub(super) fn list_agents_impl(&self) -> RepositoryResult<Vec<CoreAgent>> {
        Ok(self
            .block_on(self.registry.list())
            .into_iter()
            .map(crate::core::agent::entities::Agent::from)
            .collect())
    }

    pub(super) fn get_agent_impl(&self, agent_id: &str) -> RepositoryResult<CoreAgent> {
        let agent = self
            .block_on(self.registry.get(&self.to_agents_agent_id(agent_id)))
            .ok_or_else(|| AgentDomainError::AgentNotFound {
                id: agent_id.to_string(),
            })?;

        Ok(agent.into())
    }
}
