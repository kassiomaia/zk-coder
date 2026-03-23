//! Permission system for agent tool execution.
//!
//! Determines whether a tool call should be auto-executed,
//! queued for user approval, or blocked entirely.

use std::collections::HashMap;

use super::types::{ActionType, PermissionLevel, TrustMode};

/// Resolves permissions for tool calls based on trust mode and overrides.
#[derive(Debug, Clone)]
pub struct PermissionResolver {
    trust_mode: TrustMode,
    /// Per-tool permission overrides (tool full name -> level).
    tool_overrides: HashMap<String, PermissionLevel>,
    /// Per-category permission overrides (category -> level).
    category_overrides: HashMap<String, PermissionLevel>,
}

impl PermissionResolver {
    /// Create a new resolver with the given trust mode.
    pub fn new(trust_mode: TrustMode) -> Self {
        Self {
            trust_mode,
            tool_overrides: HashMap::new(),
            category_overrides: HashMap::new(),
        }
    }

    /// Set a permission override for a specific tool.
    pub fn set_tool_permission(&mut self, tool_name: &str, level: PermissionLevel) {
        self.tool_overrides.insert(tool_name.to_string(), level);
    }

    /// Set a permission override for an entire tool category.
    pub fn set_category_permission(&mut self, category: &str, level: PermissionLevel) {
        self.category_overrides.insert(category.to_string(), level);
    }

    /// Check the permission level for a given tool and action.
    ///
    /// Resolution order:
    /// 1. Trust mode override (Trusted -> AutoRun, Restricted -> ApprovalRequired)
    /// 2. Per-tool override
    /// 3. Per-category override
    /// 4. Default based on action type
    pub fn check(&self, tool_name: &str, action: &ActionType) -> PermissionLevel {
        // Trust mode takes precedence
        match self.trust_mode {
            TrustMode::Trusted => return PermissionLevel::AutoRun,
            TrustMode::Restricted => return PermissionLevel::ApprovalRequired,
            TrustMode::Untrusted => {}
        }

        // Check tool-specific override
        if let Some(level) = self.tool_overrides.get(tool_name) {
            return *level;
        }

        // Check category override (extract category from "category_name" format)
        if let Some(category) = tool_name.split('_').next() {
            if let Some(level) = self.category_overrides.get(category) {
                return *level;
            }
        }

        // Default based on action type
        Self::default_for_action(action)
    }

    /// Default permission based on the type of action.
    /// Read operations are auto-run; writes, deletes, and commands require approval.
    fn default_for_action(action: &ActionType) -> PermissionLevel {
        match action {
            ActionType::ReadFile { .. } => PermissionLevel::AutoRun,
            ActionType::WriteFile { .. } => PermissionLevel::ApprovalRequired,
            ActionType::DeleteFile { .. } => PermissionLevel::ApprovalRequired,
            ActionType::ExecuteCommand { .. } => PermissionLevel::ApprovalRequired,
            ActionType::WebRequest { .. } => PermissionLevel::AutoRun,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trusted_mode_auto_runs_everything() {
        let resolver = PermissionResolver::new(TrustMode::Trusted);
        let action = ActionType::DeleteFile {
            path: "/tmp/test".to_string(),
        };
        assert_eq!(resolver.check("fs_delete_file", &action), PermissionLevel::AutoRun);
    }

    #[test]
    fn test_restricted_mode_requires_approval_for_everything() {
        let resolver = PermissionResolver::new(TrustMode::Restricted);
        let action = ActionType::ReadFile {
            path: "/tmp/test".to_string(),
        };
        assert_eq!(
            resolver.check("fs_read_file", &action),
            PermissionLevel::ApprovalRequired
        );
    }

    #[test]
    fn test_untrusted_mode_auto_runs_reads() {
        let resolver = PermissionResolver::new(TrustMode::Untrusted);
        let action = ActionType::ReadFile {
            path: "/tmp/test".to_string(),
        };
        assert_eq!(resolver.check("fs_read_file", &action), PermissionLevel::AutoRun);
    }

    #[test]
    fn test_untrusted_mode_requires_approval_for_writes() {
        let resolver = PermissionResolver::new(TrustMode::Untrusted);
        let action = ActionType::WriteFile {
            path: "/tmp/test".to_string(),
            size_bytes: 100,
        };
        assert_eq!(
            resolver.check("fs_write_file", &action),
            PermissionLevel::ApprovalRequired
        );
    }

    #[test]
    fn test_tool_override_takes_precedence() {
        let mut resolver = PermissionResolver::new(TrustMode::Untrusted);
        resolver.set_tool_permission("fs_write_file", PermissionLevel::AutoRun);

        let action = ActionType::WriteFile {
            path: "/tmp/test".to_string(),
            size_bytes: 100,
        };
        assert_eq!(resolver.check("fs_write_file", &action), PermissionLevel::AutoRun);
    }

    #[test]
    fn test_category_override() {
        let mut resolver = PermissionResolver::new(TrustMode::Untrusted);
        resolver.set_category_permission("fs", PermissionLevel::AutoRun);

        let action = ActionType::WriteFile {
            path: "/tmp/test".to_string(),
            size_bytes: 100,
        };
        assert_eq!(resolver.check("fs_write_file", &action), PermissionLevel::AutoRun);
    }

    #[test]
    fn test_execute_command_requires_approval() {
        let resolver = PermissionResolver::new(TrustMode::Untrusted);
        let action = ActionType::ExecuteCommand {
            command: "rm -rf /".to_string(),
        };
        assert_eq!(
            resolver.check("system_execute", &action),
            PermissionLevel::ApprovalRequired
        );
    }
}
