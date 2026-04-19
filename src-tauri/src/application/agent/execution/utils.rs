//! Provides utility functions and constants for the agent execution process.

/// The maximum number of retries for an LLM call.
pub const LLM_MAX_RETRIES: usize = 2;
/// The base delay in milliseconds for LLM retry backoff.
pub const LLM_RETRY_BASE_MS: u64 = 1000;
/// The default maximum number of output tokens for agent LLM calls.
pub const LLM_MAX_OUTPUT_TOKENS: u32 = 16384;
/// The default temperature for the agent's LLM calls.
pub const DEFAULT_AGENT_TEMPERATURE: f32 = 0.0;
/// The environment variable for overriding the agent's temperature.
pub const ENV_AGENT_TEMPERATURE: &str = "CODER_AGENT_TEMPERATURE";

/// Base number of execution steps allowed for any task (Workstream β).
///
/// Covers trivial tasks and the up-front bootstrap overhead (planning,
/// initial tool catalog, first LLM round-trip) before per-subtask work
/// begins. Combined with `EXECUTION_STEPS_PER_SUBTASK` in
/// [`max_execution_steps`].
pub const EXECUTION_STEPS_BASE: u32 = 30;

/// Additional execution steps allocated per planned subtask (Workstream β).
///
/// Empirically, an MVP-scale subtask consumes ~15 steps end-to-end
/// (retrieval, narrate, 1–3 tool calls, advance). Scaling the budget
/// linearly with plan size keeps headroom proportional to work.
pub const EXECUTION_STEPS_PER_SUBTASK: u32 = 15;

/// Floor for the computed dynamic step budget.
///
/// Ensures trivial tasks (no subtasks) and 1-subtask plans still receive
/// a sane budget even though `BASE + 0 * PER = 30` and `BASE + 1 * PER = 45`.
pub const EXECUTION_STEPS_FLOOR: u32 = 45;

/// Ceiling for the computed dynamic step budget.
///
/// Caps pathological plans (hundreds of subtasks, hallucinated expansions)
/// from burning unbounded LLM spend before the human can intervene.
pub const EXECUTION_STEPS_CEILING: u32 = 600;

/// Hard cap on consecutive steps with only malformed-JSON tool calls.
///
/// When the provider cannot emit parseable arguments even after being told
/// the previous call failed, we abort rather than loop — this is the
/// fallback for the recovery path that the malformed-arg sentinel enables.
/// See `docs/issues/agent-plan-subtasks-malformed-json/issue.md`.
pub const MALFORMED_ARGS_ABORT_THRESHOLD: u32 = 3;

/// Computes the maximum number of execution steps allowed for a task
/// given the number of planned subtasks.
///
/// Implements the linear formula
/// `BASE + PER_SUBTASK * num_subtasks`, clamped to
/// `[EXECUTION_STEPS_FLOOR, EXECUTION_STEPS_CEILING]`.
///
/// The arithmetic is saturating so a hostile or buggy caller supplying
/// `u32::MAX` still collapses to the ceiling rather than panicking.
///
/// See Workstream β in
/// `docs/planning/agent-fidelity-followup/plan.md`.
#[must_use]
pub fn max_execution_steps(num_subtasks: u32) -> u32 {
    let computed = EXECUTION_STEPS_BASE
        .saturating_add(EXECUTION_STEPS_PER_SUBTASK.saturating_mul(num_subtasks));
    computed.clamp(EXECUTION_STEPS_FLOOR, EXECUTION_STEPS_CEILING)
}

/// Returns the temperature for the agent's LLM calls, reading from the environment variable if set.
#[must_use]
pub fn agent_temperature() -> f32 {
    let value = std::env::var(ENV_AGENT_TEMPERATURE)
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(DEFAULT_AGENT_TEMPERATURE);
    value.clamp(0.0, 2.0)
}

/// Derives a concise task title from the user's request content.
#[must_use]
pub fn derive_task_title(content: &str) -> String {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return "Agent Task".to_string();
    }

    let mut title = trimmed.lines().next().unwrap_or(trimmed).trim().to_string();

    if title.len() > 80 {
        title.truncate(80);
        title.push_str("...");
    }

    title
}

/// Truncates a string to a maximum length, appending an indicator if truncated.
#[must_use]
pub fn truncate_text(content: &str, max_len: usize) -> String {
    if content.chars().count() <= max_len {
        return content.to_string();
    }

    let truncated = content.chars().take(max_len).collect::<String>();
    format!("{truncated}...[truncated]")
}

#[cfg(test)]
mod tests {
    use super::{
        max_execution_steps, EXECUTION_STEPS_CEILING, EXECUTION_STEPS_FLOOR,
    };

    #[test]
    fn max_execution_steps_formula_boundaries() {
        // Floor wins for small plans.
        assert_eq!(max_execution_steps(0), EXECUTION_STEPS_FLOOR); // 30 → clamp to 45
        assert_eq!(max_execution_steps(1), EXECUTION_STEPS_FLOOR); // 30+15 = 45, floor tie
        // Linear region.
        assert_eq!(max_execution_steps(2), 60);
        assert_eq!(max_execution_steps(7), 135);
        assert_eq!(max_execution_steps(10), 180);
        assert_eq!(max_execution_steps(15), 255);
        assert_eq!(max_execution_steps(38), 600);
        // Ceiling clamps anything beyond the 38-subtask line.
        assert_eq!(max_execution_steps(40), EXECUTION_STEPS_CEILING);
        assert_eq!(max_execution_steps(u32::MAX), EXECUTION_STEPS_CEILING);
    }

    #[test]
    fn max_execution_steps_is_monotonic_up_to_ceiling() {
        // Small sanity: strictly non-decreasing across the curve so the
        // "cap grows" assertion in the merge path is structurally sound.
        let mut last = 0u32;
        for n in 0..60u32 {
            let current = max_execution_steps(n);
            assert!(
                current >= last,
                "max_execution_steps({n}) = {current} < previous {last}",
            );
            last = current;
        }
        assert_eq!(last, EXECUTION_STEPS_CEILING);
    }
}
