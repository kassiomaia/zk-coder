//! Defines the permission system for controlling agent tool execution.
//!
//! This module is responsible for determining whether a specific tool call
//! should be executed automatically, queued for user approval, or blocked.
//! The decision is based on the agent's trust mode and any configured overrides.

use std::collections::HashMap;

use super::types::{ActionType, PermissionLevel, TrustMode};

/// Resolves the permission level for tool calls based on trust mode and overrides.
///
/// The `PermissionResolver` is used to determine the appropriate `PermissionLevel`
/// for a given tool call by considering the overall trust mode of the agent,
/// as well as any specific overrides for tools or tool categories.
#[derive(Debug, Clone)]
pub struct PermissionResolver {
    /// The overall trust mode for the agent.
    trust_mode: TrustMode,
    /// A map of permission overrides for specific tools, indexed by tool name.
    tool_overrides: HashMap<String, PermissionLevel>,
    /// A map of permission overrides for tool categories, indexed by category name.
    category_overrides: HashMap<String, PermissionLevel>,
}

impl PermissionResolver {
    /// Creates a new `PermissionResolver` with the specified trust mode.
    ///
    /// # Arguments
    ///
    /// * `trust_mode` - The trust mode to be used for resolving permissions.
    #[must_use]
    pub fn new(trust_mode: TrustMode) -> Self {
        Self {
            trust_mode,
            tool_overrides: HashMap::new(),
            category_overrides: HashMap::new(),
        }
    }

    /// Sets a permission override for a specific tool.
    ///
    /// This allows for fine-grained control over individual tools, overriding
    /// the default behavior determined by the trust mode or category.
    ///
    /// # Arguments
    ///
    /// * `tool_name` - The full name of the tool to override.
    /// * `level` - The `PermissionLevel` to be applied to the tool.
    pub fn set_tool_permission(&mut self, tool_name: &str, level: PermissionLevel) {
        self.tool_overrides.insert(tool_name.to_string(), level);
    }

    /// Sets a permission override for an entire category of tools.
    ///
    /// This is useful for applying a consistent permission level to a group of
    /// related tools (e.g., all filesystem tools).
    ///
    /// # Arguments
    ///
    /// * `category` - The name of the category to override.
    /// * `level` - The `PermissionLevel` to be applied to the category.
    pub fn set_category_permission(&mut self, category: &str, level: PermissionLevel) {
        self.category_overrides.insert(category.to_string(), level);
    }

    /// Checks the permission level for a given tool and action type.
    ///
    /// The permission is resolved in the following order of precedence:
    /// 1. Trust mode override (`Trusted` -> `AutoRun`, `Restricted` -> `ApprovalRequired`)
    /// 2. Per-tool override
    /// 3. Per-category override
    /// 4. Default permission based on the `ActionType`
    ///
    /// # Arguments
    ///
    /// * `tool_name` - The name of the tool being checked.
    /// * `action` - The type of action the tool is performing.
    ///
    /// # Returns
    ///
    /// The resolved `PermissionLevel` for the tool call.
    #[must_use]
    pub fn check(&self, tool_name: &str, action: &ActionType) -> PermissionLevel {
        let tool_override = self.tool_overrides.get(tool_name).copied();
        let category_override = tool_name
            .split('_')
            .next()
            .and_then(|category| self.category_overrides.get(category))
            .copied();

        if matches!(tool_override, Some(PermissionLevel::Disabled))
            || matches!(category_override, Some(PermissionLevel::Disabled))
        {
            return PermissionLevel::Disabled;
        }

        // Trust mode takes precedence
        match self.trust_mode {
            TrustMode::Trusted => return PermissionLevel::AutoRun,
            TrustMode::Restricted => return PermissionLevel::ApprovalRequired,
            TrustMode::Untrusted => {}
        }

        // Check tool-specific override
        if let Some(level) = tool_override {
            return level;
        }

        // Check category override (extract category from "category_name" format)
        if let Some(level) = category_override {
            return level;
        }

        // Default based on action type
        Self::default_for_action(action)
    }

    /// Determines the default permission level based on the action type.
    ///
    /// By default, read operations are auto-run, while writes, deletes, and commands
    /// require user approval. Document creation and finalization also require approval.
    const fn default_for_action(action: &ActionType) -> PermissionLevel {
        match action {
            ActionType::ReadFile { .. } => PermissionLevel::AutoRun,
            ActionType::WriteFile { .. } => PermissionLevel::ApprovalRequired,
            ActionType::DeleteFile { .. } => PermissionLevel::ApprovalRequired,
            ActionType::ExecuteCommand { .. } => PermissionLevel::ApprovalRequired,
            ActionType::WebRequest { .. } => PermissionLevel::AutoRun,
            // Document tools operate on internal drafts/artifacts, not workspace files.
            ActionType::CreateDocx { .. } => PermissionLevel::AutoRun,
            ActionType::MutateDocxDraft { .. } => PermissionLevel::AutoRun,
            ActionType::InspectDocxDraft { .. } => PermissionLevel::AutoRun,
            ActionType::CreatePptx { .. } => PermissionLevel::AutoRun,
            ActionType::MutatePptxDraft { .. } => PermissionLevel::AutoRun,
            ActionType::InspectPptxDraft { .. } => PermissionLevel::AutoRun,
            ActionType::CreatePdf { .. } => PermissionLevel::AutoRun,
            ActionType::MutatePdfDraft { .. } => PermissionLevel::AutoRun,
            ActionType::InspectPdfDraft { .. } => PermissionLevel::AutoRun,
            ActionType::FinalizeDocx { .. } => PermissionLevel::AutoRun,
            ActionType::FinalizePptx { .. } => PermissionLevel::AutoRun,
            ActionType::FinalizePdf { .. } => PermissionLevel::AutoRun,
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
        assert_eq!(
            resolver.check("fs_delete_file", &action),
            PermissionLevel::AutoRun
        );
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
    fn test_trusted_mode_respects_disabled_tool_override() {
        let mut resolver = PermissionResolver::new(TrustMode::Trusted);
        resolver.set_tool_permission("fs_write_file", PermissionLevel::Disabled);
        let action = ActionType::WriteFile {
            path: "/tmp/test".to_string(),
            size_bytes: 10,
        };
        assert_eq!(
            resolver.check("fs_write_file", &action),
            PermissionLevel::Disabled
        );
    }

    #[test]
    fn test_untrusted_mode_auto_runs_reads() {
        let resolver = PermissionResolver::new(TrustMode::Untrusted);
        let action = ActionType::ReadFile {
            path: "/tmp/test".to_string(),
        };
        assert_eq!(
            resolver.check("fs_read_file", &action),
            PermissionLevel::AutoRun
        );
    }

    #[test]
    fn test_untrusted_mode_auto_runs_stat_path() {
        let resolver = PermissionResolver::new(TrustMode::Untrusted);
        let action = ActionType::ReadFile {
            path: "/tmp/test".to_string(),
        };
        assert_eq!(
            resolver.check("fs_stat_path", &action),
            PermissionLevel::AutoRun
        );
    }

    #[test]
    fn test_untrusted_mode_auto_runs_glob_files() {
        let resolver = PermissionResolver::new(TrustMode::Untrusted);
        let action = ActionType::ReadFile {
            path: "/tmp".to_string(),
        };
        assert_eq!(
            resolver.check("fs_glob_files", &action),
            PermissionLevel::AutoRun
        );
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
    fn test_untrusted_mode_requires_approval_for_replace_in_file() {
        let resolver = PermissionResolver::new(TrustMode::Untrusted);
        let action = ActionType::WriteFile {
            path: "/tmp/test".to_string(),
            size_bytes: 0,
        };
        assert_eq!(
            resolver.check("fs_replace_in_file", &action),
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
        assert_eq!(
            resolver.check("fs_write_file", &action),
            PermissionLevel::AutoRun
        );
    }

    #[test]
    fn test_category_override() {
        let mut resolver = PermissionResolver::new(TrustMode::Untrusted);
        resolver.set_category_permission("fs", PermissionLevel::AutoRun);

        let action = ActionType::WriteFile {
            path: "/tmp/test".to_string(),
            size_bytes: 100,
        };
        assert_eq!(
            resolver.check("fs_write_file", &action),
            PermissionLevel::AutoRun
        );
    }

    #[test]
    fn test_execute_command_requires_approval() {
        let resolver = PermissionResolver::new(TrustMode::Untrusted);
        let action = ActionType::ExecuteCommand {
            command: "rm -rf /".to_string(),
        };
        assert_eq!(
            resolver.check("system_execute_command", &action),
            PermissionLevel::ApprovalRequired
        );
    }

    #[test]
    fn test_untrusted_mode_auto_runs_document_draft_actions() {
        let resolver = PermissionResolver::new(TrustMode::Untrusted);
        let actions = vec![
            ActionType::CreateDocx {
                filename: "report.docx".to_string(),
            },
            ActionType::MutateDocxDraft {
                document_id: "docx_1".to_string(),
            },
            ActionType::InspectDocxDraft {
                document_id: "docx_1".to_string(),
            },
            ActionType::CreatePptx {
                filename: "slides.pptx".to_string(),
            },
            ActionType::MutatePptxDraft {
                deck_id: "pptx_1".to_string(),
            },
            ActionType::InspectPptxDraft {
                deck_id: "pptx_1".to_string(),
            },
            ActionType::CreatePdf {
                filename: "report.pdf".to_string(),
            },
            ActionType::MutatePdfDraft {
                pdf_id: "pdf_1".to_string(),
            },
            ActionType::InspectPdfDraft {
                pdf_id: "pdf_1".to_string(),
            },
            ActionType::FinalizeDocx {
                document_id: "docx_1".to_string(),
            },
            ActionType::FinalizePptx {
                deck_id: "pptx_1".to_string(),
            },
            ActionType::FinalizePdf {
                pdf_id: "pdf_1".to_string(),
            },
        ];

        for action in actions {
            assert_eq!(
                resolver.check("document_tool", &action),
                PermissionLevel::AutoRun
            );
        }
    }
}
