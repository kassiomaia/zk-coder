//! Static lookup table for agent meta-tool handlers.
//!
//! Mirrors the shape of the frontend's
//! `src-ui/src/lib/components/chat/messages/tool-renderers/registry.ts`:
//! a single `HashMap<&'static str, &'static dyn ToolHandler>` built
//! once via [`OnceLock`] and consulted via [`lookup`]. Adding a new
//! meta-tool is one new handler module plus one insertion in
//! [`init_registry`] — no other site needs to change.
//!
//! The map stores `&'static dyn ToolHandler` references that point at
//! the zero-sized unit structs (`PLAN_SUBTASKS`, `ADVANCE_SUBTASK`,
//! `MARK_SUBTASK_STATUS`, `NARRATE`) declared in each handler module.
//! No runtime allocation happens on the hot path — lookups are pure
//! hash-map reads.

use std::collections::HashMap;
use std::sync::OnceLock;

use super::advance_subtask::ADVANCE_SUBTASK;
use super::dispatcher::ToolHandler;
use super::mark_subtask_status::MARK_SUBTASK_STATUS;
use super::narrate::NARRATE;
use super::plan_subtasks::PLAN_SUBTASKS;

static REGISTRY: OnceLock<HashMap<&'static str, &'static dyn ToolHandler>> = OnceLock::new();

fn init_registry() -> HashMap<&'static str, &'static dyn ToolHandler> {
    let mut map: HashMap<&'static str, &'static dyn ToolHandler> = HashMap::new();
    let handlers: [&'static dyn ToolHandler; 4] = [
        &PLAN_SUBTASKS,
        &ADVANCE_SUBTASK,
        &MARK_SUBTASK_STATUS,
        &NARRATE,
    ];
    for handler in handlers {
        let prev = map.insert(handler.name(), handler);
        debug_assert!(
            prev.is_none(),
            "duplicate agent meta-tool name in registry: {}",
            handler.name(),
        );
    }
    map
}

/// Returns the handler registered under `name`, or `None` when the
/// call is for a regular (non-agent-meta) tool handled by the shared
/// `ToolRegistry`.
pub(super) fn lookup(name: &str) -> Option<&'static dyn ToolHandler> {
    REGISTRY.get_or_init(init_registry).get(name).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_all_expected_tools() {
        let map = REGISTRY.get_or_init(init_registry);
        let mut names: Vec<&'static str> = map.keys().copied().collect();
        names.sort();
        assert_eq!(
            names,
            vec![
                "agent_advance_to_next_subtask",
                "agent_mark_subtask_status",
                "agent_narrate",
                "agent_plan_subtasks",
            ],
        );
    }

    #[test]
    fn lookup_returns_none_for_unknown_tool() {
        assert!(lookup("definitely_not_a_tool").is_none());
    }

    #[test]
    fn lookup_returns_handler_for_known_tool() {
        let handler = lookup("agent_narrate").expect("narrate should be registered");
        assert_eq!(handler.name(), "agent_narrate");
    }
}
