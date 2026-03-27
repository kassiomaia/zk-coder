pub const LLM_MAX_RETRIES: usize = 2;
pub const LLM_RETRY_BASE_MS: u64 = 1000;
pub const DEFAULT_AGENT_TEMPERATURE: f32 = 0.0;
pub const ENV_AGENT_TEMPERATURE: &str = "CODER_AGENT_TEMPERATURE";
pub const MAX_EXECUTION_STEPS: u32 = 120;

pub fn agent_temperature() -> f32 {
    let value = std::env::var(ENV_AGENT_TEMPERATURE)
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(DEFAULT_AGENT_TEMPERATURE);
    value.clamp(0.0, 2.0)
}

pub fn derive_task_title(content: &str) -> String {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return "Agent Task".to_string();
    }

    let mut title = trimmed
        .lines()
        .next()
        .unwrap_or(trimmed)
        .trim()
        .to_string();

    if title.len() > 80 {
        title.truncate(80);
        title.push_str("...");
    }

    title
}

pub fn truncate_text(content: &str, max_len: usize) -> String {
    if content.chars().count() <= max_len {
        return content.to_string();
    }

    let truncated = content.chars().take(max_len).collect::<String>();
    format!("{truncated}...[truncated]")
}
