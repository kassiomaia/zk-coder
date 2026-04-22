//! Agent registry for managing agent configurations.
//!
//! Contains built-in agents and supports runtime registration.
//!
//! # System-prompt architecture (Workstream H.1)
//!
//! The agent's static system prompt decomposes into three separately hashable
//! sections, composed once at agent construction and frozen in the `Agent`
//! struct:
//!
//! 1. **Harness contract** (`harness_contract_prompt`) — invariant across
//!    tasks. Teaches the execution model, subtask contracts, scope-drift
//!    consequences, narration-intent semantics, failure-memory scoping,
//!    clean-exit verification, and compaction awareness. Stable across
//!    subtask transitions, mode changes, and task boundaries.
//!
//! 2. **Quality baseline** (`quality_baseline_prompt`) — invariant,
//!    genre-neutral. 5–8 lines that set a professional bar without domain
//!    vocabulary: verify before asserting, write in the voice the task calls
//!    for, stop rather than fabricate, treat output as shippable.
//!
//! 3. **Default code framing** (`default_code_framing`) — currently emitted
//!    for every agent. Carries code-specific defaults (stack, file-size
//!    limits, typing rules, code Definition of Done). Until H.2 wires these
//!    to per-task `TaskFraming`, they are concatenated here so the default
//!    code-work behavior is preserved.
//!
//! See `docs/planning/agent-fidelity/system-prompt-architecture.md` for the
//! full design rationale.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::Arc;

use chrono::Utc;
use tokio::sync::RwLock;

use crate::core::agent::task::TaskFraming;
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

/// Builder that composes an agent's static system prompt from three
/// separately hashable sections.
///
/// This is the assembly point for Workstream H.1: the agent-level system
/// prompt is frozen at agent construction time (not recomputed per step) and
/// is composed of the harness contract, the quality baseline, and the
/// optional default code framing. Keeping assembly in one place lets
/// `prompt_stability_tests.rs` and the runtime telemetry (`llm_request_prepared`)
/// attribute cache-invalidating drift to the exact section that moved.
///
/// The separator between sections is a deliberate empty line. Changing it
/// moves every composed-prompt hash and must be done in lockstep with the
/// golden-file test.
pub struct SystemPromptBuilder;

impl SystemPromptBuilder {
    /// Separator between sections. A blank line is the minimum readable
    /// boundary; longer separators waste tokens.
    pub const SECTION_SEPARATOR: &'static str = "\n\n";

    /// Compose the default built-in agent prompt:
    /// `harness_contract + quality_baseline + default_code_framing`.
    ///
    /// Phase 1 emits all three sections for the single built-in agent. When
    /// H.2 lands, the third section will be replaced by a per-task
    /// `TaskFraming` record; `harness_contract + quality_baseline` will
    /// become the agent-level invariant and framing will arrive dynamically
    /// with the task.
    #[must_use]
    pub fn compose_default_code_agent() -> String {
        Self::compose(
            harness_contract_prompt(),
            quality_baseline_prompt(),
            Some(default_code_framing()),
        )
    }

    /// Compose the three sections. `framing` is optional: when `None`, only
    /// harness + quality are assembled (the invariant half of H.2).
    #[must_use]
    pub fn compose(harness: &str, quality: &str, framing: Option<&str>) -> String {
        match framing {
            Some(framing) => format!(
                "{harness}{sep}{quality}{sep}{framing}",
                sep = Self::SECTION_SEPARATOR,
            ),
            None => format!(
                "{harness}{sep}{quality}",
                sep = Self::SECTION_SEPARATOR,
            ),
        }
    }

    /// Compose the agent-level prompt for a task (Workstream H.2.c).
    ///
    /// When `task_framing` is `Some`, its rendered form
    /// ([`framing_prompt`]) replaces [`default_code_framing`] as the third
    /// section. When `None`, the composition falls back to the default
    /// code framing so legacy (pre-H.2) tasks — and tasks the planner did
    /// not declare a framing for — keep the exact prompt they have today.
    ///
    /// The returned string is the concatenation
    /// `harness + quality + framing` with the standard section separator,
    /// so `framing_hash` can be computed directly against the rendered
    /// framing section without re-deriving the boundaries.
    #[must_use]
    pub fn compose_for_task(task_framing: Option<&TaskFraming>) -> String {
        match task_framing {
            Some(framing) => {
                let rendered = framing_prompt(framing);
                Self::compose(
                    harness_contract_prompt(),
                    quality_baseline_prompt(),
                    Some(rendered.as_str()),
                )
            }
            None => Self::compose_default_code_agent(),
        }
    }

    /// Returns the framing section that will be concatenated into the
    /// task-level system prompt, including the genre-default fallback.
    ///
    /// Mirrors [`Self::compose_for_task`]: when `task_framing` is `Some`
    /// the rendered framing is returned; when `None` the genre-default
    /// code framing is returned unchanged. `framing_hash` is computed
    /// against this value.
    #[must_use]
    pub fn active_framing_section(task_framing: Option<&TaskFraming>) -> String {
        match task_framing {
            Some(framing) => framing_prompt(framing),
            None => default_code_framing().to_string(),
        }
    }
}

/// Render a task-declared `TaskFraming` into the framing section body
/// (Workstream H.2.c).
///
/// Sections included, in order:
///
/// 1. `## TASK FRAMING` banner so the section is visually distinct from
///    the harness contract and quality baseline.
/// 2. `Role: ...` — the short descriptor supplied by the planner.
/// 3. `Voice constraints` — the opinionated bulleted list.
/// 4. `Domain priors` — bulleted hints (omitted when absent or empty).
/// 5. `Forbidden patterns` — bulleted regex list (omitted when empty).
///
/// The rendering is deterministic: identical input produces identical
/// output byte-for-byte, so `framing_hash` is stable across steps in the
/// same task and only moves at task boundaries (validated by
/// `prompt_stability_tests`).
#[must_use]
pub fn framing_prompt(framing: &TaskFraming) -> String {
    let mut out = String::from("## TASK FRAMING\n\n");
    let _ = writeln!(out, "Role: {}", framing.role.trim());
    out.push('\n');
    out.push_str("Voice constraints:\n");
    for constraint in &framing.voice_constraints {
        let _ = writeln!(out, "- {}", constraint.trim());
    }
    if let Some(priors) = &framing.domain_priors {
        if !priors.hints.is_empty() {
            out.push('\n');
            out.push_str("Domain priors:\n");
            for hint in &priors.hints {
                let _ = writeln!(out, "- {}", hint.trim());
            }
        }
    }
    if !framing.forbidden_patterns.is_empty() {
        out.push('\n');
        out.push_str("Forbidden patterns (regex; outputs must not match):\n");
        for pattern in &framing.forbidden_patterns {
            let _ = writeln!(out, "- {pattern}");
        }
    }
    // Trim trailing newline so the section joins cleanly with the
    // separator. The composer adds `\n\n` between sections.
    while out.ends_with('\n') {
        out.pop();
    }
    out
}

/// The harness contract — invariant across tasks, sessions, and subtasks.
///
/// Teaches the execution model (step-based loop, context compaction),
/// subtask contracts (even before Workstream B lands, the *concept* is
/// taught), scope-drift consequences, narration-intent semantics,
/// failure-memory scoping, clean-exit verification, and tool-economy
/// constraints. Wording follows the guidance in
/// `docs/planning/agent-fidelity/system-prompt-architecture.md` §2.1–§2.8.
///
/// Stable across subtask transitions and mode changes — locked by
/// `prompt_stability_tests::harness_section_hash_is_stable_*` and the
/// golden-file test added for Workstream H.6
/// (`harness_contract_matches_h6_golden`). Edits to this string must
/// update the golden in lockstep.
///
/// Workstream H.6 rewrote every sentence in consequence language:
/// every line names either a mechanism the runtime enforces or a
/// runtime-visible artifact the model must cooperate with. Hedging
/// language ("try to", "when possible", "meaningful moments", "focus
/// on") is removed; where the target machinery does not yet exist
/// (subtask contracts are Workstream B; narration intent is Workstream
/// E), the prompt names the concept by the identifier that will land
/// so the model's vocabulary stabilizes ahead of the behavior.
#[must_use]
pub const fn harness_contract_prompt() -> &'static str {
    "## HARNESS CONTRACT

## EXECUTION MODEL

You run inside a step-based execution harness. At each step you emit tool calls or narration; the runtime executes them and returns control. Between steps, the runtime may compact, reorder, or drop messages from your context according to a deterministic policy. The artifacts that survive compaction are: this system prompt, the active-subtask appendix when present, the `ContextPolicyV2 execution brief`, the `ContextPolicyV2 session memory reference`, and the most recent `ContextPolicyV2 failure memory`. Your context is a working set, not a log — if an observation must survive compaction, it must be written into one of these artifacts or into an on-disk output.

## PLANNING AND SUBTASKS

For any non-trivial task (anything beyond reading a single file, answering a simple question, or a conversational reply), your first action is `agent_plan_subtasks`. The runtime will not proceed with multi-step work until subtasks exist. Calling `agent_plan_subtasks` again after subtasks already exist is treated as a no-op by the runtime.

Simple requests that do not require a plan:
- Greetings, acknowledgements, or small talk ('Hi', 'Thanks', 'Ok')
- Reading a single file
- Answering a factual question or explaining how something works
- Single-step requests that require no code changes or command execution

Your work is decomposed into subtasks. Each subtask has a title, a mode (Read / Write / Execute / FullAccess), a status (Pending / InProgress / Completed / Failed), and — once Workstream B lands — a contract declaring the artifact paths its outputs must match. The active subtask appears in the appendix at the bottom of this prompt and in the `ContextPolicyV2 active subtask` system message; both are refreshed on every transition.

When a task begins, the runtime has produced a skeleton plan (titles + descriptions). Your first action must be `agent_plan_subtasks` to attach contracts (expected_artifacts) and, if applicable, a task-level framing. Skipping this step disables scope-drift detection and clean-exit verification for the remainder of the task.

Writing to paths that do not match the active subtask's declared scope will be flagged as scope drift and injected as a correction in the next step's context. Repeated scope drift causes the drifting tool call to be rejected by the runtime.

Advance with `agent_advance_to_next_subtask` — the runtime marks the current subtask Completed and the next InProgress. If a subtask cannot be completed, call `agent_mark_subtask_status` with `Failed` and a reason. The runtime does not silently advance; neither should you.

## COMMUNICATION

Two tools exist for user-visible updates. They are not interchangeable.

- `agent_emit_status` — 2–6 word spinner phrases visible only while the step is active. Emit one per substantive tool batch: 'Scanning dependencies...', 'Reading main.rs...', 'Running tests...'. Status does not persist in chat history.

- `agent_narrate` — durable 1–2 sentence updates that persist in chat history. Narrate: plan creation, phase transitions, discoveries that change your approach, errors and how you are recovering, and the final completion summary. Do not narrate every file read or small action — the user follows your work through the chat alone, and narration noise drowns the signal.

Once Workstream E lands, `agent_narrate` will accept `intent: narration | question`. `intent: narration` lets execution continue; `intent: question` pauses the runtime until the user replies or the clarification timeout expires. Until that schema change ships, narrate questions as plain sentences and stop rather than guess — fabricating placeholders like `[Product Name]` or `[COMPANY NAME]` produces output the user cannot ship.

## FAILURE MEMORY

The `ContextPolicyV2 failure memory` system message carries the most recent tool-call failure scoped to the active subtask (Workstream C.2). A failure from a previous subtask does not surface in the current subtask's memory unless it was recorded without a subtask scope. Each entry carries a `FailureClass`:

- `FileNotFound` — the resource does not exist. Retrying with the same path will fail identically; change the path before retrying.
- `PermissionDenied` — the resource exists but is not accessible as requested. Retrying will fail identically until access changes; revise the approach.
- `Timeout` — the operation exceeded its budget. Retry is only productive with a different approach or scope.
- `ValidationFailed` — arguments were rejected by the tool's schema. Fix the arguments before retrying.
- `NotAllowedByMode` — the tool is disallowed by the current subtask's mode. Retrying under this subtask will fail identically regardless of arguments. Wait for a later subtask that permits the tool, or call `agent_mark_subtask_status(Failed, reason)`.
- `NotAllowedByArgs` — the tool is permitted, but the arguments hit an argument-level policy. Change the arguments before retrying.
- `Other` — unclassified. Treat with the same caution as `Timeout`; prefer revising the approach over blind retry.

Repeated tool calls with identical arguments within a short window are classified as loops by the runtime and trigger a forced tool-choice override on the next step.

## MEMORY ARTIFACT AND EXECUTION BRIEF

The `ContextPolicyV2 session memory reference` carries a compact pointer to the durable working-memory artifact (task goal, active plan, completed findings, open questions, extracted facts). The artifact survives compaction; the inline block carries counts and truncated fields only. The runtime refreshes memory on subtask completion, on retrieval bursts, and on compaction boundaries. You do not write to memory directly — contributions flow through completing subtasks and emitting narrations.

The `ContextPolicyV2 execution brief` is the compact 'what matters now' snapshot placed near the attention tail: `goal`, `active` subtask, `recent` completions, and `blocker` if a failure belongs to the active subtask. The `next` line is suppressed in the current release (Workstream D.1); Workstream B will re-enable it once a transition-readiness gate exists. Do not begin work on a later subtask while the active one is InProgress — the runtime will classify the writes as scope drift.

## TOOL ECONOMY

Call the cheapest tool that can answer the current question. Broad exploration, repeated reads of already-seen files, and unnecessary command execution inflate the context and accelerate compaction. Stop exploring once the evidence is sufficient to act; summarize findings in narration rather than re-collecting raw content.

Destructive operations — file writes, deletes, command execution — route through user approval unless the session runs in FullAccess mode. State the intent in narration before the call; the user reads the narration when deciding to approve.

## EXIT

A task is Done when every subtask reaches a terminal status (Completed, Failed, or Cancelled). If any subtask ended Failed, the session is flagged for review. In every termination path, close with a narration that summarizes what was delivered, what was deferred or skipped, and which subtasks (if any) ended Failed."
}

/// The quality baseline — invariant, genre-neutral.
///
/// 5–8 lines that set a professional bar without domain vocabulary. This
/// section intentionally carries no stack defaults, style rules, or
/// language-specific guidance: those live in the per-task framing (or, in
/// Phase 1, in `default_code_framing`).
///
/// Stable across tasks and subtasks — locked by
/// `prompt_stability_tests::quality_baseline_hash_is_stable_*`.
#[must_use]
pub const fn quality_baseline_prompt() -> &'static str {
    "## QUALITY BASELINE

You are a careful senior professional working on behalf of the user. You verify facts before asserting them, and you inspect existing material before modifying it. You write in the voice the current task calls for, not a default voice. You stop rather than fabricate when you lack information. You treat your output as something the user will ship, not a draft for later cleanup. You make minimal, coherent changes and you do not duplicate logic, leave partial states, or claim work succeeded when it has not been validated."
}

/// Default code-work framing.
///
/// Carries the code-specific defaults (stack, file-size limits, typing
/// rules, code Definition of Done) that used to live inline in the agent
/// prompt. Until Workstream H.2 lands per-task `TaskFraming`, this string is
/// concatenated with the harness contract and quality baseline so the
/// default code-work behavior is preserved. When H.2 ships, non-code tasks
/// will substitute their own framing here instead of inheriting code
/// defaults.
///
/// The boundary is deliberate: only content that is *only* true for code
/// work belongs in this string. Narration, planning, subtasks, and failure
/// memory live in the harness contract because they are universal.
#[must_use]
pub const fn default_code_framing() -> &'static str {
    "## ROLE AND PRIORITIES (CODE WORK)

You are a senior full-stack engineering agent. You can read files, write code, search codebases, and execute commands to help the user with software development tasks. Your priority is correctness, safety, maintainability, and clarity — not just generating code. Inspect existing code before modifying, preserve working behavior unless change is required, and never behave like a blind code generator.

## DEFAULT STACK (WHEN UNSPECIFIED)

Backend: Bun + Elysia, Drizzle ORM, SQLite (default)
Frontend: React + TypeScript + Vite, Tailwind + shadcn/ui
Shared: Zod (validation), TanStack Query, React Hook Form, Vitest, Playwright

## TOOL USAGE STRATEGY (KEEP CONTEXT CLEAN)

Work progressively and choose the cheapest tool that can answer the current question. Avoid broad exploration, repeated reads, huge outputs, and unnecessary command execution.

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
- Prefer direct `program` + `args` launches for simple executables and scripts; use `command` only when shell syntax is required.
- Avoid shell expansion, pipes, redirects, and compound command chains unless they are essential to the task.
- Keep commands scoped to the relevant directory and task.
- Do NOT use shell commands for simple file discovery, file reading, or text search when dedicated tools can do it with less noise.

## SAFETY RULES (ANTI-CORRUPTION)

- Do not modify unrelated code
- Do not perform large refactors unless necessary
- Do not break public interfaces without reason
- Do not leave partial or inconsistent states
- Do not duplicate logic
- Do not fake implementations or results
- Understand dependencies before editing
- Keep changes consistent across files
- Preserve backward compatibility when possible

## ARCHITECTURE & BOUNDARIES

Backend: routes → request/response, services → business logic, repositories → data access, schemas → validation
Frontend: components → UI only, hooks → logic/state, services/api → server communication
Rules: no business logic in UI, no DB access in controllers, low coupling, high cohesion

## FILE SIZE & QUALITY

- Hard limit: 900 lines per file, target 200–500 lines
- If >600 lines: split file, extract modules/functions, group by responsibility
- Avoid: god files, mixed responsibilities, uncontrolled file growth

## TYPE SAFETY

- No \"any\" unless unavoidable
- Define reusable types/interfaces, centralize shared types
- Align Zod schemas with TS types for end-to-end consistency

## TESTING & VALIDATION

After changes: run typecheck, lint, unit/integration tests (Vitest), E2E if needed (Playwright)
Test behavior not implementation, include edge cases, never claim tests passed if not executed.

Web/app info rules:
- Use `system_get_app_info` only if application metadata is specifically needed.
- Use web search only as a last resort when the workspace and conversation do not contain the answer and the tool is available.

## DOCUMENT GENERATION (MANDATORY)

When users ask you to create documents (Word/DOCX, PowerPoint/PPTX, or PDF), you MUST use the internal document generation tools:

For PDF (.pdf): Use `pdf_start` → `pdf_add_page` → `pdf_finalize`
For Word (.docx): Use `docx_start` → `docx_add_section` → `docx_finalize`
For PowerPoint (.pptx): Use `pptx_start` → `pptx_add_slide` → `pptx_finalize`

NEVER generate documents via `system_execute_command`. Specifically:
- Do NOT run pandoc, pdflatex, xelatex, lualatex, weasyprint, wkhtmltopdf, or any external document-conversion tool.
- Do NOT write Python, TypeScript, Node.js, or bash scripts that produce document files.
- Do NOT use `fs_write_file` to write raw PDF/DOCX/PPTX bytes.

The internal tools produce professional output with embedded fonts, bookmarks, headers/footers, and theme-driven styling. External tools are unnecessary and unsupported. The document will be automatically saved to the artifact store and a download link will appear in the chat.

PDF STYLING: `pdf_start` accepts an optional `theme` object that overrides the bundled defaults. Every field is optional — supply only the knobs the user asked to change. Supported sections:
- `typography`: `body_font_size_pt` (6–72, default 11), `heading_font_size_pt` (8–96, default 18), `line_leading` (1.0–2.5, default 1.3; use 2.0 for double-spaced).
- `palette`: RGB triples (each channel in [0,1]) for `text`, `heading`, `code_bg`, `code_text`, `callout_bg`, `callout_border`, `table_header_bg`, `table_border`.
- `spacing`: `block_gap`, `list_indent_mm`, `block_padding_mm`, `table_cell_padding_mm`.
- `page_defaults.margin_mm` (5–80, default 20).
- `decorations`: `header` and `footer` templates (tokens: `{title}`, `{page}`, `{total_pages}`, `{section}`) — pass explicit `null` to drop a strip; `suppress_on_first_page` (boolean); `font_size_pt`; `margin_mm`.
- `fonts`: filenames from the bundled set — `LiberationSans-Regular.ttf`, `LiberationSans-Bold.ttf`, `LiberationSans-Italic.ttf`, `LiberationMono-Regular.ttf`, `DejaVuSans.ttf`.

Example — pass the following as the pdf_start arguments: filename=invoice.pdf, title=Invoice, theme.typography.body_font_size_pt=13, theme.palette.heading={r:0.1, g:0.2, b:0.8}, theme.decorations.footer='Acme — {page}/{total_pages}'. Out-of-range values are rejected with a descriptive error naming the offending field so you can correct on the next step.

PDF PAGE CONTENT TYPES: `pdf_add_page` accepts four content variants — 'markdown' (primary), 'text' (plain unformatted), 'image' (embeds a PNG/JPEG from disk), and 'cover' (vertically-centred title page). For formal documents open with a cover as the first call: {type:'cover', title, subtitle?, author?, date?}; the renderer forces a dedicated page and auto-suppresses header/footer on it via the theme's suppress_on_first_page. For images, pass width_mm and/or height_mm in millimetres to control display size; when both are set use fit='contain' (default, preserves aspect and letterboxes) or fit='stretch' (uses both dimensions literally).

## BACKUP & ROLLBACK

Before changes: prefer git checkpoint/branch, backup files before risky edits
For large changes: work incrementally, validate step-by-step, revert if instability occurs
Database: use migrations, avoid destructive changes unless necessary, always consider rollback

## DEFINITION OF DONE

Done means:
- Request fulfilled with correct, safe, maintainable code
- Architectural consistency maintained
- Type safety enforced, no \"any\" unless unavoidable
- No duplication, no dead code
- Edge cases handled
- Tests updated where needed
- Docs updated if relevant
- Risks clearly communicated

Goal: safe, clean, maintainable full-stack changes."
}

impl Agent {
    /// Built-in Agent configuration.
    ///
    /// The `system_prompt` is composed once at construction from the three
    /// H.1 sections and frozen. It is *not* recomputed per step — see the
    /// module-level docstring for the rationale.
    #[must_use]
    pub fn code_agent() -> Self {
        let now = Utc::now();
        Self {
            id: AgentId::new("code-agent"),
            name: "Agent".to_string(),
            description: "Assists with code reading, writing, and execution".to_string(),
            system_prompt: SystemPromptBuilder::compose_default_code_agent(),
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

    // ------------------------------------------------------------------------
    // H.1 characterization: content preservation and section attribution.
    //
    // These tests lock in the invariant that splitting the legacy monolithic
    // prompt into three sections preserves the user-visible behavior
    // (every token the old prompt relied on is still present in the composed
    // output) and that each section's boundary is correct: harness wording
    // does not leak into the code framing, and code-work tokens do not
    // pollute the harness or quality baseline.
    // ------------------------------------------------------------------------

    #[test]
    fn composed_prompt_is_deterministic_across_invocations() {
        let first = SystemPromptBuilder::compose_default_code_agent();
        let second = SystemPromptBuilder::compose_default_code_agent();
        assert_eq!(first, second);
    }

    #[test]
    fn composed_prompt_is_concatenation_of_three_sections() {
        let composed = SystemPromptBuilder::compose_default_code_agent();
        let expected = format!(
            "{harness}{sep}{quality}{sep}{framing}",
            harness = harness_contract_prompt(),
            quality = quality_baseline_prompt(),
            framing = default_code_framing(),
            sep = SystemPromptBuilder::SECTION_SEPARATOR,
        );
        assert_eq!(composed, expected);
    }

    #[test]
    fn composed_prompt_preserves_token_efficient_tool_guidance() {
        // Characterization from the pre-split prompt: these markers must
        // still appear in the composed output.
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

    #[test]
    fn composed_prompt_preserves_harness_mandatory_planning_language() {
        let prompt = Agent::code_agent().system_prompt;
        assert!(prompt.contains("agent_plan_subtasks"));
        assert!(prompt.contains("agent_advance_to_next_subtask"));
        assert!(prompt.contains("agent_mark_subtask_status"));
        assert!(prompt.contains("agent_emit_status"));
        assert!(prompt.contains("agent_narrate"));
    }

    #[test]
    fn composed_prompt_preserves_document_generation_rules() {
        let prompt = Agent::code_agent().system_prompt;
        assert!(prompt.contains("pdf_start"));
        assert!(prompt.contains("docx_start"));
        assert!(prompt.contains("pptx_start"));
        assert!(prompt.contains("NEVER generate documents via `system_execute_command`"));
    }

    #[test]
    fn harness_contract_contains_no_code_specific_tokens() {
        // The harness contract is the invariant across tasks. It must not
        // carry stack defaults, typing rules, or code-specific file-size
        // limits. When H.2 lands and non-code tasks stop inheriting code
        // framing, the harness contract must still work unchanged — so any
        // code-specific token here is a latent bug.
        let harness = harness_contract_prompt();
        for token in [
            "Bun",
            "Elysia",
            "Drizzle",
            "React",
            "Tailwind",
            "shadcn",
            "Zod",
            "TanStack",
            "Playwright",
            "900 lines",
            "pandoc",
        ] {
            assert!(
                !harness.contains(token),
                "harness contract must not contain code-specific token `{token}` — \
                 move it to default_code_framing",
            );
        }
    }

    #[test]
    fn quality_baseline_contains_no_code_specific_tokens() {
        let quality = quality_baseline_prompt();
        for token in [
            "Bun",
            "Elysia",
            "Drizzle",
            "React",
            "TypeScript",
            "Tailwind",
            "shadcn",
            "Zod",
            "Playwright",
            "Vitest",
            "pandoc",
        ] {
            assert!(
                !quality.contains(token),
                "quality baseline must be genre-neutral; token `{token}` is code-specific",
            );
        }
    }

    #[test]
    fn default_code_framing_carries_the_code_specific_content() {
        // Mirror image of the harness test: this is where stack defaults
        // and file-size limits live in Phase 1.
        let framing = default_code_framing();
        assert!(framing.contains("Bun"));
        assert!(framing.contains("Drizzle"));
        assert!(framing.contains("React"));
        assert!(framing.contains("TypeScript"));
        assert!(framing.contains("900 lines"));
        assert!(framing.contains("DEFINITION OF DONE"));
    }

    // ------------------------------------------------------------------------
    // Workstream H.2.c: task-declared framing composition.
    //
    // These tests lock in the invariant that when a task declares its own
    // `TaskFraming`, the composed agent-level prompt carries the declared
    // framing in place of the default code framing. When no framing is
    // declared, the composition falls back to `default_code_framing()` so
    // every pre-H.2 task renders identically.
    // ------------------------------------------------------------------------

    fn sample_communication_framing() -> crate::core::agent::task::TaskFraming {
        use crate::core::agent::task::{DomainPriors, TaskFraming};
        TaskFraming {
            role: "senior communication professional".to_string(),
            voice_constraints: vec![
                "one claim per paragraph".to_string(),
                "no filler openers".to_string(),
            ],
            domain_priors: Some(DomainPriors {
                hints: vec!["press-release voice".to_string()],
            }),
            forbidden_patterns: vec![r"(?i)game[- ]changing".to_string()],
        }
    }

    #[test]
    fn compose_for_task_without_framing_matches_default_code_agent() {
        let with_default = SystemPromptBuilder::compose_for_task(None);
        let current = SystemPromptBuilder::compose_default_code_agent();
        assert_eq!(
            with_default, current,
            "pre-H.2 tasks without framing must match the legacy composed prompt exactly",
        );
    }

    #[test]
    fn compose_for_task_with_framing_replaces_code_framing_with_declared_framing() {
        let framing = sample_communication_framing();
        let composed = SystemPromptBuilder::compose_for_task(Some(&framing));

        // Harness and quality baseline still present.
        assert!(composed.contains("HARNESS CONTRACT"));
        assert!(composed.contains("QUALITY BASELINE"));

        // Declared framing content appears.
        assert!(composed.contains("TASK FRAMING"));
        assert!(composed.contains("senior communication professional"));
        assert!(composed.contains("one claim per paragraph"));
        assert!(composed.contains("press-release voice"));

        // Default code framing must NOT leak into a communication task.
        assert!(
            !composed.contains("Bun"),
            "code defaults must not appear when a non-code framing is declared",
        );
        assert!(!composed.contains("Drizzle"));
        assert!(!composed.contains("900 lines"));
    }

    #[test]
    fn framing_prompt_is_deterministic_for_identical_input() {
        let framing = sample_communication_framing();
        let a = framing_prompt(&framing);
        let b = framing_prompt(&framing);
        assert_eq!(a, b);
    }

    #[test]
    fn framing_prompt_omits_empty_domain_priors_section() {
        use crate::core::agent::task::TaskFraming;
        let framing = TaskFraming {
            role: "writer".to_string(),
            voice_constraints: vec![
                "short sentences".to_string(),
                "no adjectives without a concrete number".to_string(),
            ],
            domain_priors: None,
            forbidden_patterns: vec![],
        };
        let rendered = framing_prompt(&framing);
        assert!(!rendered.contains("Domain priors"));
        assert!(!rendered.contains("Forbidden patterns"));
        assert!(rendered.contains("TASK FRAMING"));
        assert!(rendered.contains("writer"));
    }

    #[test]
    fn active_framing_section_matches_composition_branch() {
        // The hash helper is computed against `active_framing_section`;
        // when no framing is declared, the helper returns
        // `default_code_framing()` so `framing_hash` stays stable against
        // the pre-H.2 prompt.
        assert_eq!(
            SystemPromptBuilder::active_framing_section(None),
            default_code_framing(),
        );
        let framing = sample_communication_framing();
        assert_eq!(
            SystemPromptBuilder::active_framing_section(Some(&framing)),
            framing_prompt(&framing),
        );
    }

    #[test]
    fn compose_without_framing_drops_the_third_section() {
        let with_framing = SystemPromptBuilder::compose(
            harness_contract_prompt(),
            quality_baseline_prompt(),
            Some(default_code_framing()),
        );
        let without_framing = SystemPromptBuilder::compose(
            harness_contract_prompt(),
            quality_baseline_prompt(),
            None,
        );
        assert!(with_framing.len() > without_framing.len());
        // The two-section variant must not carry any code framing token.
        assert!(!without_framing.contains("Bun"));
        assert!(!without_framing.contains("Drizzle"));
        // But it must still carry harness + quality content.
        assert!(without_framing.contains("HARNESS CONTRACT"));
        assert!(without_framing.contains("QUALITY BASELINE"));
    }
}
