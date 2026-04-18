# System Prompt Architecture for a Harness-Based Agent

**Status**: DESIGN
**Last reviewed**: 2026-04-18
**Purpose**: Define what a harness-based agent's system prompt must contain to make the model a cooperative participant in the harness's invariants. Source of truth for workstream H in `./plan.md`.

---

## Why this doc exists

The current prompt (`src-tauri/src/infrastructure/agent/registry.rs:112-314`) is a single 200-line string that mixes a code-specific persona with descriptive harness guidance. The 2026-04-18 session trace (`a597debb-c5b7-49eb-913e-7b11c7e4ac84`) showed the model violating every soft harness instruction it was given. The root cause is not the model's disobedience — it's that the prompt:

- Tells the model to "focus on this task" but never defines what finishing it means.
- Describes narration as *"meaningful moments"* but gives no mechanism for asking a blocking question.
- Mentions `agent_advance_to_next_subtask` but not that advancement can be rejected.
- Has no vocabulary for contracts, scope, artifacts, compaction, or memory refresh — the very machinery surrounding the model in every step.

The fix is not a longer prompt. The fix is a **layered prompt** whose content mirrors the runtime's invariants. Everything the runtime enforces must be stated in the prompt; everything the prompt claims must be enforced. Drift between those two surfaces is where fabrication and scope violation live.

---

## Design principles

### 1. Separate what changes from what doesn't

Three change rates:

- **Invariant (the harness contract + quality baseline)** — stable across tasks, across sessions, across subtasks. Only changes when the harness itself changes.
- **Task-variant (the declared framing)** — stable for the duration of a task; changes at task boundaries only. Carries role, voice constraints, domain priors, forbidden patterns.
- **Subtask-variant (the active subtask appendix)** — changes at subtask transitions, unchanged within a subtask.

These three rates demand three concatenated sections. Fusing them — as today — forces the whole prompt hash to move whenever any field changes, burning the Anthropic prompt cache and making behavior changes impossible to attribute cleanly.

### 2. Consequence language, not suggestion language

Every sentence in the harness section should state a **runtime consequence**:

- Bad: *"Focus on completing this task."*
- Good: *"Writing outputs outside this subtask's contract will be flagged as scope drift and corrected in the next step's context."*

- Bad: *"Use `agent_narrate` at meaningful moments."*
- Good: *"`agent_narrate` accepts `intent: question | narration`. Questions pause execution until the user replies. Narrations do not."*

- Bad: *"Stop exploring as soon as you have enough evidence."*
- Good: *"Repeated tool calls with identical arguments within a 5-step window are classified as loops and trigger a forced tool-choice override."*

The model will respect what it believes is enforced. Descriptive prose about "good practices" is prompt noise the model learns to discount.

### 3. State mechanisms, not manners

The prompt should teach the model the **names and shapes of the system messages** it will see. The model already encounters:

- `ContextPolicyV2 task summary`
- `ContextPolicyV2 active subtask`
- `ContextPolicyV2 failure memory`
- `ContextPolicyV2 session memory reference`
- `ContextPolicyV2 execution brief`

None of these are explained in the current prompt. The model has to infer their meanings. Explaining them once at the top reduces per-step confusion, lets the model reason about its own context, and costs almost nothing in tokens once cached.

### 4. Minimize imperatives; teach invariants

Imperatives age badly and stack up. Invariants are evergreen. Prefer:

- *"Your older messages may be replaced by a compaction summary. Record findings via the session memory artifact when they must survive."*

over:

- *"Don't forget to write findings before compaction."*

### 5. Persona is a property of the task, not the agent

The agent is an execution engine. Persona is task framing. Baking a persona into the agent — as today's `code_agent_system_prompt()` does — forces every task to inherit the same framing, which is wrong for any task whose work is not code. Adding more agents (code-agent + communication-agent + document-agent) scales O(N roles) and still requires upfront classification.

Instead, persona arrives with the task as a `TaskFraming` record supplied by the planning phase (the model declares it; the user can override). The record carries role, voice constraints, domain priors, and optional forbidden patterns. It becomes the task-variant section of the prompt and hashes as `framingHash`.

The invariant half of the prompt — harness contract plus a short, genre-neutral quality baseline — must work unchanged for any framing. It must **not** carry stack defaults, style rules, file-size limits, or anything else that is only true for code work.

### 6. Stable across subtasks, except at the appendix

The concatenation `harness_contract + domain + active_subtask_appendix` must hash identically across every step of the same subtask. Only the appendix moves on subtask transitions. This is a hard cache-warmth requirement validated by `prompt_stability_tests.rs`.

### 7. The active-subtask appendix is the one place to encode the *current* contract

Contract wording belongs in the appendix — not the invariant half — because the contract changes per subtask. Appendix wording is the only place where consequence language gets *specific*: "Expected outputs: `docs/communications/social/*.md`. Writes outside this set will be flagged."

---

## Layered structure

Every harness-based agent system prompt has four conceptual layers. Layers 1–3 concatenate into the static system message; layer 4 is per-request context, not part of the cached prefix.

```
┌─────────────────────────────────────────────────────────────┐
│ Layer 1: Harness contract      (invariant across tasks)     │
│        + Quality baseline      (genre-neutral, invariant)   │
├─────────────────────────────────────────────────────────────┤
│ Layer 2: Task framing          (task-variant, declared at   │
│   role, voice, domain,          planning time, stable for   │
│   forbidden patterns            the duration of the task)   │
├─────────────────────────────────────────────────────────────┤
│ Layer 3: Active subtask        (changes at transitions)     │
│          appendix                                           │
└─────────────────────────────────────────────────────────────┘
          │
          ▼  concatenated into `system` role message
┌─────────────────────────────────────────────────────────────┐
│ Layer 4: Dynamic context       (per-request system msgs)    │
│   • ContextPolicyV2 task summary                            │
│   • ContextPolicyV2 active subtask                          │
│   • ContextPolicyV2 failure memory                          │
│   • ContextPolicyV2 session memory reference                │
│   • ContextPolicyV2 execution brief                         │
└─────────────────────────────────────────────────────────────┘
```

---

## Layer 1 — Harness contract + quality baseline (invariant)

Short, genre-neutral. No role. No stack defaults. No task-specific style rules. The quality baseline is 5–8 lines that set a professional bar without domain:

> You are a careful senior professional working on behalf of the user. You verify facts before asserting them. You write in the voice the current task calls for, not a default voice. You stop rather than fabricate when you lack information. You treat your output as something the user will ship, not a draft for later cleanup.

Everything task-type-specific — stack defaults, file-size limits, style rules, domain vocabulary — has moved out of this layer and into the per-task framing (Layer 2, next).

---

## Layer 1 (detail) — Harness contract sections

The rest of Layer 1. Identical across tasks. Organized by the mechanism each section explains.

### 2.1 Execution model

> You run in a step-based loop. At each step you may call tools or emit narration. Between steps, the runtime may compact, reorder, or drop messages from your context according to a deterministic policy. The artifacts that survive compaction are: the system prompt, this active-subtask appendix, the execution brief, the session memory reference, and the most recent failure memory.
>
> Your context is not a log. Treat it as a working set.

### 2.2 Subtasks and contracts

> Your work is decomposed into subtasks. Each subtask has:
> - a **title** and **mode** (Read / Write / Execute / FullAccess)
> - a **contract** describing its expected outputs (paths, artifact kinds)
> - a **status** (Pending / InProgress / Completed / Failed)
>
> The active subtask appears in the appendix at the bottom of this prompt and in a `ContextPolicyV2 active subtask` system message. Outputs that do not match the active subtask's contract will be flagged as **scope drift** and corrected in the next step's context. Repeated scope drift causes the drifting tool call to be rejected.
>
> To advance past a subtask, its contract artifacts must exist on disk. Call `agent_advance_to_next_subtask` only when they do. If a subtask cannot be completed, call `agent_mark_subtask_status` with `Failed` and a reason — never silently advance.

### 2.3 Communication

> You have two communication tools, used for different purposes:
>
> - `agent_emit_status` — short spinner phrases (2–6 words). Visible only while active. Use for liveness signalling during long work: "Scanning dependencies…", "Running tests…". Does not persist.
>
> - `agent_narrate` — durable natural-language updates, 1–2 sentences, persist in chat history. Accepts `intent: narration | question`.
>   - `intent: narration` — you inform the user of progress, findings, or completions. Execution continues.
>   - `intent: question` — you need information to proceed. **Execution pauses** until the user replies or the clarification timeout expires. Use this when you cannot proceed honestly — never fabricate missing information as placeholders like `[Product Name]`.

### 2.4 Failure memory

> You may see a `ContextPolicyV2 failure memory` system message carrying the most recent tool-call failure. Each failure is **scoped to the subtask in which it occurred** and carries a `FailureClass`:
>
> - `FileNotFound` — the resource doesn't exist. Retry is useful if you change the path.
> - `PermissionDenied` — the resource exists but you can't access it as requested.
> - `Timeout` — the operation exceeded its budget. Retry is only useful with a different approach.
> - `ValidationFailed` — your arguments were malformed. Fix the arguments before retrying.
> - `NotAllowedByMode` — the tool is disallowed by this subtask's mode. Do not retry. Wait for a later subtask or mark this one `Failed` if it cannot proceed without the tool.
> - `NotAllowedByArgs` — the tool is allowed but your arguments were rejected. Change arguments before retrying.
>
> A failure shown in memory does not mean you should retry it. Read its `FailureClass` to decide.

### 2.5 Memory artifact

> The `ContextPolicyV2 session memory reference` is a compact pointer to a persistent artifact holding your task goal, active plan, completed findings, open questions, and extracted facts. It survives compaction. The runtime refreshes it on subtask completion, on explicit triggers, and on compaction boundaries. You do not write to it directly; you contribute to it indirectly by completing subtasks and emitting narrations.

### 2.6 Execution brief

> The `ContextPolicyV2 execution brief` is a compact snapshot of where you are:
>
> - `goal` — the original user request
> - `active` — your current subtask
> - `recent` — completed subtasks
> - `blocker` — a failure only if it belongs to the active subtask
> - `next` (conditional) — the next subtask's title, shown only when the active subtask's contract is satisfied and you are ready to advance
>
> `next` is informational. Do not begin work on the next subtask while the active one is in progress.

### 2.7 Tool economy

> Prefer the cheapest tool that can answer the current question. Repeated tool calls with identical arguments within a short window are classified as loops and will cause the runtime to force a different tool choice. If a tool keeps failing for the same reason, stop retrying: narrate the observation, revise the approach, or mark the subtask `Failed`.

### 2.8 Exit

> A task is Done when every subtask is in `Completed` state and the contract artifacts for each exist. If any subtask is `Failed`, the task completes but the session is flagged `needsReview`. In both cases, close with a narration summarizing what was delivered and what, if anything, was deferred.

---

## Layer 2 — Task framing

Declared at planning time, carried for the duration of the task, hashed as `framingHash`. Shape:

- `role` — short descriptor ("senior full-stack engineer", "senior communication professional", "technical writer")
- `voice_constraints` — opinionated, specific. Bland one-liners like "professional and clear" are rejected at planning time. Example: *"one claim per paragraph; no adjectives without a concrete number; no filler openers like 'we're thrilled to…'"*
- `domain_priors` — optional, structured. Stack defaults for code, style references for writing, artifact conventions. This is where the old code-agent text lives **when the framing is coding work**.
- `forbidden_patterns` — optional regex list. Example for launch copy: *"the future of"*, *"pioneering"*, *"empower"*, *"unprecedented"*, *"game-changing"*.

Framing is declared by the model in the planning phase and is visible to the user for override before execution starts. It must not redefine how subtasks, narration, failure memory, or advancement work — those are Layer 1 concerns.

---

## Layer 3 — Active subtask appendix

Rendered by `derive_system_prompt()`. Present only when a subtask is `InProgress`.

Current rendering:

```
## Current Task ({order}/{total}): {title}

{mode.instruction_text()}

Focus on completing this task. When done, call agent_advance_to_next_subtask.
```

Target rendering (after H.3, dependent on B.1):

```
## Current Subtask ({order}/{total}): {title}

**Mode**: {mode} — {mode.instruction_text()}

**Expected outputs**:
{for each artifact_spec in contract.expected_artifacts:}
- {spec.kind}: {spec.path_pattern}

**Contract**:
- Writes to paths outside the expected outputs will be flagged as scope drift.
- Advancing past this subtask requires the expected outputs to exist.
- If you cannot produce the expected outputs, call agent_mark_subtask_status(Failed, reason).
- If you need information you do not have, call agent_narrate with intent=question — execution will pause.
```

When `contract` is absent (legacy plans pre-B), fall back to the current rendering to preserve behavior.

---

## Layer 4 — Dynamic context

Not part of the cached system prompt. Injected per-request as additional `system` messages by `ContextPolicyV2`. Already implemented; this doc lists them only so Layer 2 can reference them by name.

Ordering is fixed and documented in `../agent-context-engineering/plan.md`:

1. `ContextPolicyV2 task summary`
2. `ContextPolicyV2 active subtask`
3. `ContextPolicyV2 failure memory` (scoped; post-workstream C)
4. `ContextPolicyV2 session memory reference`
5. `ContextPolicyV2 execution brief` (gated `next`; post-workstream D)

The model sees these every step. The Layer 2 harness contract explains each one once, letting the model reason about them instead of inferring.

---

## What a good harness prompt does *not* contain

- **Stack defaults, style rules, or anything task-type-specific.** They belong in Layer 2 (task framing), not Layer 1.
- **Rules with no enforcement.** If the runtime doesn't enforce it, stating it invites drift.
- **Hedging language** (*"try to"*, *"ideally"*, *"when possible"*). Either the harness enforces it or the model will discount it.
- **Long examples of model-facing prose.** Examples train style, not mechanism. Use them sparingly and only for the communication tools.
- **References to file paths in this repository.** The prompt ships with the binary; internal paths rot.
- **Anything the user has already said.** The original request is already in context.

---

## Validation plan

The principles above become testable in workstreams H.4 and H.5:

1. `harness_contract_prompt()` and `quality_baseline_prompt()` hash identically across tasks — caught by H.4.
2. A sentence in Layer 2 that claims a runtime consequence must point to a code path that enforces it — caught by doc-and-code review during H.6.
3. A PR that changes Layer 2 text must update the golden file and attach measurement results — caught by H.6's golden-file test.
4. Per-request telemetry separates `harnessSectionHash` from `domainSectionHash` — caught by H.5.

---

## Summary — the seven things a harness system prompt must have

1. **A layered structure** whose change rates match the runtime's change rates.
2. **Persona declared per-task (framing), not baked into the agent.**
3. **Consequence language** for every invariant the runtime enforces.
4. **Named mechanisms** the model can cite back to itself ("ContextPolicyV2 active subtask", "FailureClass::NotAllowedByMode").
5. **Explicit mention of every system message the model will see** and what it means.
6. **An active-subtask appendix** that localizes the current contract, rendered only when a subtask is `InProgress`.
7. **Stable prefix hashes** verified by prompt-stability tests, so behavior changes attribute cleanly to their cause.

Everything else is task-specific (belongs in Layer 2 framing) or dynamic (belongs in Layer 4).
