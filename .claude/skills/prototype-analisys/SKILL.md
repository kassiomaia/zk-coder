---
name: prototype-analysis
description: Analyze a prototype developed with HTML, CSS and Javascript and create implementation plans for Svelte 5 components. Use when reviewing any prototype referenced by the user as a prototype of a feature that will be added to the application
---

# Prototype Analysis Skill

Analyze prototypes created with the `prototype-interfaces` skill and generate comprehensive implementation plans for Svelte 5 components.

## Purpose

This skill bridges the gap between prototypes and production code by:
1. **Validating Prototype Quality** - Ensuring prototypes meet standards before planning implementation
2. **Analyzing Component Structure** - Understanding UI components, state, and interactions
3. **Identifying Reuse Opportunities** - Finding existing components that can be extended or reused
4. **Planning Implementation** - Creating detailed, actionable plans for frontend and backend
5. **Flagging Uncertainties** - Highlighting areas that need clarification before implementation

## When to Use

- User references a prototype folder for implementation
- Planning conversion of HTML/Alpine.js prototype to Svelte components
- Reviewing prototype readiness for production development
- Identifying component reuse opportunities across prototypes

## Prototype Location (MANDATORY)

**All prototypes MUST be located at `./prototypes/` in the project root.**

Valid path: `./prototypes/<prototype-name>/`
Invalid path: Any other location

If a prototype is referenced from any other location, STOP and inform the user that prototypes must be in the `./prototypes/` directory.

## Prototype Structure (Expected)

```
./prototypes/
└── <prototype-name>/
    ├── README.md           # Feature specification (NOT implementation docs)
    ├── index.html          # Entry point with Alpine.js directives
    ├── css/
    │   ├── design-system.css   # Design tokens (copy from project)
    │   ├── components.css      # Component-specific styles
    │   └── animations.css      # Keyframes and transitions (optional)
    └── js/
        └── state.js            # Alpine.js state management
```

## Rules

This skill follows specific rules defined in `./rules/`. Read and apply these rules in order:

| Rule | File | Purpose |
|------|------|---------|
| **Location** | `00-location-rule.md` | Prototype must be at `./prototypes/` |
| **Validation** | `01-validation.md` | Quality checks before planning |
| **Clarification** | `02-clarification.md` | Mandatory questions to ask |
| **Component Mapping** | `03-component-mapping.md` | Alpine.js to Svelte conversion |
| **Plan Generation** | `04-plan-generation.md` | Plan structure and content |
| **Backend Planning** | `05-backend-planning.md` | Tauri backend requirements |
| **Golden Rules** | `06-golden-rules.md` | Inviolable principles |
| **Parallel Analysis** | `07-parallel-analysis.md` | Task orchestration and prompts |

**All rules MUST be followed when analyzing prototypes.**

---

## Parallel Analysis Architecture

This skill uses parallel tasks to efficiently analyze prototypes. The orchestrator coordinates specialized analysis tasks.

### Flow Diagram

```
                           ┌─────────────────────────┐
                           │      USER REQUEST       │
                           │  "Analyze prototype X"  │
                           └───────────┬─────────────┘
                                       │
                                       ▼
                           ┌─────────────────────────┐
                           │    LOCATION CHECK       │
                           │  ./prototypes/<name>/   │
                           └───────────┬─────────────┘
                                       │
                            ┌──────────┴──────────┐
                            │                     │
                     Valid Location        Invalid Location
                            │                     │
                            ▼                     ▼
                   ┌────────────────┐    ┌────────────────┐
                   │  ORCHESTRATOR  │    │  STOP & WARN   │
                   │  LAUNCH TASKS  │    │  Wrong path!   │
                   └────────┬───────┘    └────────────────┘
                            │
         ┌──────────────────┼──────────────────┬──────────────────┐
         │                  │                  │                  │
         ▼                  ▼                  ▼                  ▼
┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
│    TASK 1       │ │    TASK 2       │ │    TASK 3       │ │    TASK 4       │
│    README       │ │    STYLES       │ │    LOGIC        │ │    STRUCTURE    │
│    Analysis     │ │    Analysis     │ │    Analysis     │ │    Analysis     │
│                 │ │                 │ │                 │ │                 │
│  Read README.md │ │  Read CSS files │ │  Read state.js  │ │  Read HTML      │
│  Extract scope  │ │  Check tokens   │ │  Map state      │ │  Find components│
│  User journeys  │ │  Find hardcoded │ │  Find methods   │ │  Alpine.js dirs │
└────────┬────────┘ └────────┬────────┘ └────────┬────────┘ └────────┬────────┘
         │                  │                  │                  │
         └──────────────────┴──────────────────┴──────────────────┘
                                       │
                                       ▼
                           ┌─────────────────────────┐
                           │      ORCHESTRATOR       │
                           │   Aggregate Reports     │
                           │   Cross-Validate        │
                           └───────────┬─────────────┘
                                       │
                         ┌─────────────┴─────────────┐
                         │                           │
                    All Checks Pass           Issues Found
                         │                           │
                         ▼                           ▼
              ┌─────────────────────┐    ┌─────────────────────┐
              │  ASK CLARIFICATION  │    │  ASK CLARIFICATION  │
              │  QUESTIONS          │    │  + FLAG ISSUES      │
              └──────────┬──────────┘    └──────────┬──────────┘
                         │                          │
                         ▼                          ▼
              ┌─────────────────────┐    ┌─────────────────────┐
              │  GENERATE PLAN      │    │  GENERATE PLAN      │
              │  Status: READY      │    │  Status: NOT READY  │
              └─────────────────────┘    └─────────────────────┘
```

### Task Definitions

#### Task 1: README Analysis

**Input:** `./prototypes/<name>/README.md`

**Responsibilities:**
- Extract feature purpose and business context
- Identify user stories
- List functional requirements
- Document scope (in-scope vs contextual)
- Extract user journeys with IF/THEN logic
- Note accessibility requirements

**Output Report:**
```
README ANALYSIS REPORT
======================
Feature: <name>
Purpose: <description>

SCOPE:
  In-Scope: [list]
  Contextual: [list]

USER STORIES: [list]

FUNCTIONAL REQUIREMENTS: [list]

USER JOURNEYS: [list with conditions]

ACCESSIBILITY: [requirements]

ISSUES FOUND: [list any missing documentation]
```

---

#### Task 2: Styles Analysis

**Input:**
- `./prototypes/<name>/css/components.css`
- `./prototypes/<name>/css/animations.css` (optional)
- `./prototypes/<name>/css/design-system.css`

**Responsibilities:**
- List all CSS classes and their purposes
- Check for design token usage
- Identify hardcoded values (colors, spacing)
- Document animations
- Note responsive breakpoints

**Output Report:**
```
STYLES ANALYSIS REPORT
======================
Prototype: <name>

CSS CLASSES:
  - .class-name: <purpose>

DESIGN TOKEN USAGE:
  Using tokens: [list of tokens used]
  Hardcoded values found: [list with file:line]

ANIMATIONS:
  - @keyframes <name>: <description>

RESPONSIVE:
  Breakpoints: [list]

ISSUES FOUND:
  - [list hardcoded colors/spacing that should use tokens]
```

---

#### Task 3: Logic/State Analysis

**Input:** `./prototypes/<name>/js/state.js`

**Responsibilities:**
- Document state properties and types
- Identify computed properties (getters)
- List all methods and their purposes
- Map to Svelte 5 patterns ($state, $derived)
- Identify async operations
- Note side effects

**Output Report:**
```
LOGIC/STATE ANALYSIS REPORT
===========================
Prototype: <name>

STATE PROPERTIES:
  - propertyName: <type> → Svelte: $state()

COMPUTED PROPERTIES:
  - computedName → Svelte: $derived()

METHODS:
  - methodName(): <purpose>

ASYNC OPERATIONS:
  - [list async methods]

SIDEEFFECTS:
  - [list any side effects]

MAPPING TO SVELTE:
  [table of Alpine.js → Svelte 5 conversions]

ISSUES FOUND:
  - [list any logic concerns]
```

---

#### Task 4: Structure/HTML Analysis

**Input:** `./prototypes/<name>/index.html`

**Responsibilities:**
- Identify component boundaries
- Document Alpine.js directives used
- List interactive elements
- Map component hierarchy
- Check for inline styles/scripts (violations)
- Identify Lucide Icons usage

**Output Report:**
```
STRUCTURE ANALYSIS REPORT
=========================
Prototype: <name>

COMPONENT HIERARCHY:
  Container
  ├── Header
  │   └── Navigation
  ├── Content
  │   ├── ComponentA
  │   └── ComponentB
  └── Footer

ALPINE.JS DIRECTIVES:
  - x-data: [list]
  - x-show: [list]
  - x-for: [list]
  - @click: [list]

INTERACTIVE ELEMENTS:
  - [list buttons, inputs, etc.]

VIOLATIONS CHECK:
  Inline styles: [PASS/FAIL - details]
  Inline scripts: [PASS/FAIL - details]
  Inline SVGs: [PASS/FAIL - details]

ISSUES FOUND:
  - [list any structural issues]
```

---

## Orchestrator Process

### Step 1: Location Validation

Before launching tasks, verify the prototype location:

```javascript
if (!path.startsWith('./prototypes/')) {
  STOP("Invalid prototype location. Must be at ./prototypes/<name>/");
}
```

### Step 2: Launch Parallel Tasks

Launch all 4 analysis tasks simultaneously:

```
Task 1: README Analysis    → Reading README.md
Task 2: Styles Analysis    → Reading CSS files
Task 3: Logic Analysis     → Reading state.js
Task 4: Structure Analysis → Reading index.html
```

### Step 3: Aggregate Reports

Collect all task reports and cross-validate:

| Check | Source Tasks | Validation |
|-------|--------------|------------|
| Scope alignment | README + Structure | Components match documented scope |
| State usage | Logic + Structure | Alpine.js state matches JS definitions |
| Style consistency | Styles + Structure | CSS classes exist for HTML elements |
| Completeness | All | All documentation sections covered |

### Step 4: Decision Gate

```
IF all checks pass AND no issues found:
  status = "READY TO IMPLEMENT"
  questions = standard_clarification_questions()

ELSE IF issues found:
  status = "NOT READY TO IMPLEMENT"
  questions = clarification_questions + issue_questions

GENERATE PLAN with status and questions
```

---

## Workflow After Parallel Analysis

### Phase 2: Clarification Questions

**MANDATORY:** Before finalizing the plan, ask the user clarifying questions based on analysis reports.

#### Required Questions:

1. **Scope Confirmation:**
   - "The README marks [X, Y, Z] as in-scope and [A, B, C] as contextual. Should I proceed with this scope understanding?"

2. **Component Relevance:**
   - "I've identified these main components: [list]. Are all of these required for the initial implementation, or should some be deferred?"

3. **Backend Requirements:**
   - "Does this feature require backend changes? If so, should I create a separate backend implementation plan?"

4. **Existing Components:**
   - "I found similar existing components: [list]. Should I prioritize extending these over creating new ones?"

5. **Design System Alignment:**
   - "The prototype uses [specific patterns/colors]. Are these aligned with the current design system, or should adjustments be made?"

### Phase 3: Component Analysis

After questions are answered, map to existing Svelte components:

```bash
# Search for existing components
src-ui/src/lib/components/
```

For each prototype component, determine:
- **Exact Match:** Component exists and can be used as-is
- **Extend:** Component exists but needs modification
- **New:** No existing component, create from scratch
- **Reuse Opportunity:** Can be generalized for future reuse

### Phase 4: Plan Generation

Generate the implementation plan at:
```
./docs/<prototype-name>/prototype-implementation-plan.md
```

---

## State Management Mapping Reference

| Alpine.js Pattern | Svelte 5 Equivalent |
|-------------------|---------------------|
| `x-data="{ prop: value }"` | `$state()` runes |
| `x-show="condition"` | `{#if}` blocks |
| `x-for="item in items"` | `{#each}` blocks |
| `x-model="prop"` | `bind:value` |
| `@click="method()"` | `onclick` or `on:click` |
| `x-bind:class="{ active: isActive }"` | `class:` directive |
| `get computedProp()` | `$derived()` |
| Methods | Functions in `<script>` |

---

## Plan Template

```markdown
# Implementation Plan: <Feature Name>

> **Status:** READY TO IMPLEMENT / NOT READY TO IMPLEMENT
> **Generated:** <date>
> **Prototype Source:** prototypes/<prototype-name>/

## Summary

<Brief description of what will be implemented>

---

## Clarification Questions

<ONLY IF status is "NOT READY TO IMPLEMENT" - list required clarifications>

---

## Frontend Implementation

### Component Architecture

#### New Components

| Component | Path | Description | Priority |
|-----------|------|-------------|----------|
| ComponentA | src-ui/src/lib/components/... | ... | P1 |

#### Components to Extend

| Existing Component | Modifications | Priority |
|--------------------|---------------|----------|
| ExistingComponent | Add X, Y | P1 |

### State Management

| State Property | Type | Svelte Pattern | Store Location |
|----------------|------|----------------|----------------|
| isOpen | boolean | $state | Component local |

### CSS Updates

| File | Changes |
|------|---------|
| ComponentA.module.css | New file with styles from prototype |

### Implementation Steps

1. [ ] Create ComponentA with basic structure
2. [ ] Add state management
3. [ ] Implement interactions
4. [ ] Add CSS module styles
5. [ ] Write unit tests
6. [ ] Integrate with parent component

---

## Backend Implementation (Tauri)

> **Status:** REQUIRED / NOT REQUIRED / NEEDS CLARIFICATION

### Tauri Commands

| Command | File | Description |
|---------|------|-------------|
| `get_resources` | commands/module.rs | Fetch all items |

### Data Models (Rust)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub id: String,
    pub name: String,
}
```

---

## File Changes Summary

### New Files
- `src-ui/src/lib/components/...`

### Modified Files
- `src-ui/src/lib/components/...`

---

## Acceptance Criteria

- [ ] All in-scope components implemented
- [ ] State management working correctly
- [ ] CSS matches prototype visual design
- [ ] Accessibility requirements met
- [ ] Unit tests passing
- [ ] No hardcoded design values
```

---

## Golden Rules

### 1. Never Generate a Plan Without Questions
Always ask clarifying questions before generating the implementation plan.

### 2. Flag Uncertainties
If any aspect is unclear, mark the plan as "NOT READY TO IMPLEMENT" and list questions.

### 3. Prioritize Reuse
Always search for existing components before planning new ones.

### 4. Design System Compliance
Ensure all planned CSS uses design tokens. Flag any prototype styles that use hardcoded values.

### 5. Separate Backend Plans
Backend implementation requires its own plan following the same analysis process.

### 6. Scope Discipline
Only plan for components marked as "in-scope" in the prototype README.

### 7. Use Parallel Analysis
Always launch all 4 analysis tasks in parallel for efficiency.

---

## Reference

- Prototype creation skill: `.claude/skills/prototype-interfaces/SKILL.md`
- Design system: `src-ui/src/lib/styles/design-system.css`
- Existing components: `src-ui/src/lib/components/`
- Tauri commands: `src-tauri/src/commands/`
- Tauri storage: `src-tauri/src/storage/`
- Backend rules: `rules/05-backend-planning.md`
