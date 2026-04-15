# PDF Authoring Tool — Planning Index

**Status**: DRAFT
**Last reviewed**: 2026-04-15
**Purpose**: Single entry point for the PDF-authoring-tool initiative. Keeps
the plan as small and executable as possible. Start at
[`03-execution.md`](03-execution.md) if you are here to build.

---

## Goal in one sentence

Give the agent a typed, testable, Rust-native tool surface for producing
**professional, publication-quality PDF documents** from natural-language
prompts, integrated into the existing Tauri + Svelte product and its
design system.

---

## Start here

- **Implementer?** Open [`03-execution.md`](03-execution.md). It is the
  single source of truth for what to build next.
- **Reviewer?** Read this file + [`01-vision.md`](01-vision.md). The
  vision's Success Criteria are the final acceptance gate.
- **Curious about why we chose X?** [`02-architecture.md`](02-architecture.md)
  and [`research/libraries.md`](research/libraries.md).

---

## Documents in this folder

| Document | Status | Purpose |
|---|---|---|
| [`03-execution.md`](03-execution.md) | DRAFT | **The execution plan.** 3 milestones, 19 tasks, each one PR. This is what we work from. |
| [`00-as-is.md`](00-as-is.md) | DRAFT | Ground-truth audit of the current PDF tool. Enumerates the 20 numbered gaps the execution plan closes. |
| [`01-vision.md`](01-vision.md) | DRAFT | What "production-grade" means. Capabilities, user journeys, non-goals, success criteria. North star only — not an execution plan. |
| [`02-architecture.md`](02-architecture.md) | DRAFT | Layered proposal: typed domain model, renderer trait, theme model, communication layer. Technical reference for tasks in `03-execution.md`. |
| [`research/libraries.md`](research/libraries.md) | DRAFT | Evidence-backed comparison of `printpdf`, `lopdf`, `genpdf`, `oxidize-pdf`. |
| [`research/industry.md`](research/industry.md) | DRAFT | Firecrawl Fire-PDF lessons and other industry signals we actually use. |

### Revision notes — 2026-04-15

The initial plan had **8 documents** including separate `03-poc-plan.md`
and `04-roadmap.md`. That created a "plan the plan" problem: POCs and
roadmap phases overlapped, gates were spread across two files, and a
Phase 5 covering PDF editing was speculative scope.

**Revised plan**:

- POC plan and roadmap merged into a single `03-execution.md`.
- 5 phases collapsed into **3 milestones**, each shipping real
  user-visible value.
- Speculative Phase 5 deleted. Not on the plan until explicitly asked.
- Vision trimmed to a north star; execution detail lives in
  `03-execution.md`.
- Gap tracker lives inside `03-execution.md` so tasks and gaps stay in
  the same file.

Result: **6 documents instead of 8**, one source of truth for execution,
and every task explicitly retires at least one `00-as-is.md` gap.

---

## The plan in one screen

Full detail in [`03-execution.md`](03-execution.md). Here is the shape.

```
M1 — Typed core + honest rendering            (8 tasks)
     Ships: honest typography, themes, no truncation, background render
     Exit gate: 4-page golden PDF locked

M2 — Structural documents                     (7 tasks)
     Ships: Journey A end-to-end (cover, TOC, headers/footers,
     tables, images, hyperlinks, bookmarks, structural validator)
     Exit gate: Journey A golden PDF locked + validator rewritten

M3 — UX integration                           (4 tasks)
     Ships: live progress events, card shows structural facts,
     theme picker in Settings, docs marked COMPLETE
     Exit gate: all 7 success criteria in 01-vision.md are true
```

**19 tasks total. Each task is one PR. Each task is test-first. Each
task retires a numbered gap.**

---

## Non-negotiables (inherited from `RULES.md` and quality guidelines)

- Test-first. Failing tests are the first commit on the task branch.
- Delete the superseded code in the same PR that introduces the
  replacement. Per `docs/quality/guidelines/rust-backend.md § Refactor
  process`.
- No new design-system tokens. The authoring UI uses only tokens from
  `src-ui/src/lib/styles/design-system.css` and shared files.
- Every file ≤ 500 LoC soft limit per `RULES.md § Quality Assurance`.
- `cargo test` and `cargo clippy` clean before any task is considered
  done. `npm run check` and `npx vitest run src/lib/styles/css-integrity.test.ts`
  for frontend tasks.
- Artifacts live under `AppHandle::path().app_local_data_dir()`, never
  `$HOME` (enforced in T1.7).

---

## Mandatory reading order before executing any task

Per `AGENTS.md` and `docs/quality/guidelines/README.md`:

1. `AGENTS.md`
2. `RULES.md`
3. `docs/quality/guidelines/README.md`
4. Every applicable guide in `docs/quality/guidelines/` (at minimum
   `rust-backend.md`; `svelte.md` + `typescript.md` for M3 tasks)
5. This file
6. [`03-execution.md`](03-execution.md)
7. [`00-as-is.md`](00-as-is.md) — for the gap being closed
8. [`02-architecture.md`](02-architecture.md) — for the layer boundary
9. [`01-vision.md`](01-vision.md) — only before a milestone exit gate
10. `research/` — only if the task changes a dependency

---

## Traceability to the original request

| Ask from the prompt | Where it lives |
|---|---|
| Explore what we have done | [`00-as-is.md`](00-as-is.md) |
| Understand libraries (lopdf, genpdf, oxidize) | [`research/libraries.md`](research/libraries.md) |
| Firecrawl article lessons | [`research/industry.md`](research/industry.md) |
| "As is" and "what to do" | `00-as-is.md` + `01-vision.md` + `03-execution.md` |
| Small evolutionary tasks with real value | [`03-execution.md § Milestones`](03-execution.md) |
| TDD | Every task in `03-execution.md` has a "Tests (first)" field |
| POC → evidence → criticize → iterate | Execution milestones use golden-file exit gates; no separate POC doc |
| Align with Rust/Tauri/Svelte stack | [`02-architecture.md § Guiding principles`](02-architecture.md) |
| Design system and visual consistency | `03-execution.md § T3.2` and the Non-negotiables above |
| Communication layer | `02-architecture.md § Communication layer` + T1.8 + T3.1 |
| Clear DoD per phase | `03-execution.md § M1/M2/M3 exit gates` |
| User journey | [`01-vision.md § User journey`](01-vision.md) |
| Solution for AI agent and professionals | [`01-vision.md § Statement of intent`](01-vision.md) |
| Reduce complexity, clarify phases | This revision — see _Revision notes_ above |

---

## Next step

1. Land this folder on a feature branch (`feature/pdf-authoring-tool`).
2. Start **T1.1** from [`03-execution.md`](03-execution.md). Write the
   failing tests first.
3. Do not look past M1 until M1's exit gate is green.
