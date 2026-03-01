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

## Workflow

### Phase 1: Discovery & Validation

#### 1.1 Read Prototype README
- Understand feature purpose and business context
- Identify user stories and functional requirements
- Note the scope definition (in-scope vs contextual elements)
- Review user journeys and conditional logic

#### 1.2 Validate Prototype Quality
Check the prototype against quality criteria:

**Documentation Quality:**
- [ ] README has clear overview with scope note
- [ ] User stories are defined
- [ ] Functional requirements are listed
- [ ] Component hierarchy marks scope for all elements
- [ ] User journeys have IF/THEN conditional logic
- [ ] ASCII flow diagrams are present
- [ ] States and variants are documented
- [ ] Accessibility requirements are specified

**Code Quality:**
- [ ] No inline CSS (all styles in `css/` files)
- [ ] No inline JavaScript logic (all logic in `js/` files)
- [ ] Uses Lucide Icons (no inline SVGs)
- [ ] Design system tokens used (not hardcoded values)
- [ ] Alpine.js state clearly structured in external file

#### 1.3 Analyze HTML Structure
- Identify component boundaries
- Map Alpine.js directives to expected Svelte patterns
- List all interactive elements
- Document component hierarchy

#### 1.4 Analyze CSS
- Review `components.css` for component styles
- Check for design token usage vs hardcoded values
- Identify animations in `animations.css`
- Note any responsive breakpoints

#### 1.5 Analyze JavaScript/State
- Read `js/state.js` for state structure
- Map Alpine.js state properties to Svelte stores/runes
- Identify computed properties ($derived)
- Document methods and their purposes
- Note any side effects or async operations

### Phase 2: Clarification Questions

**MANDATORY:** Before proceeding with the plan, ask the user clarifying questions.

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

#### 3.1 Map to Existing Svelte Components
Search the project for similar components:

```bash
# Search for existing components
src-ui/src/lib/components/
```

For each prototype component, determine:
- **Exact Match:** Component exists and can be used as-is
- **Extend:** Component exists but needs modification
- **New:** No existing component, create from scratch
- **Reuse Opportunity:** Can be generalized for future reuse

#### 3.2 State Management Mapping

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

#### 3.3 CSS Module Mapping
- Convert `.component-class` to CSS module pattern
- Ensure design tokens are preserved
- Create `ComponentName.module.css` for each component

### Phase 4: Plan Generation

Generate the implementation plan at:
```
./docs/<prototype-name>/prototype-implementation-plan.md
```

#### Plan Template:

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
| ComponentB | src-ui/src/lib/components/... | ... | P2 |

#### Components to Extend

| Existing Component | Modifications | Priority |
|--------------------|---------------|----------|
| ExistingComponent | Add X, Y | P1 |

#### Reusable Components to Create

| Component | Path | Can Be Used By |
|-----------|------|----------------|
| SharedComponent | src-ui/src/lib/components/shared/ | Feature A, Feature B |

### State Management

| State Property | Type | Svelte Pattern | Store Location |
|----------------|------|----------------|----------------|
| isOpen | boolean | $state | Component local |
| items | Item[] | Svelte store | src-ui/src/lib/stores/ |

### CSS Updates

| File | Changes |
|------|---------|
| ComponentA.module.css | New file with styles from prototype |
| design-system.css | Add new tokens if needed (justify) |

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

<If required, include Tauri-specific sections below>

### Tauri Commands Examples

| Command | File | Description |
|---------|------|-------------|
| `get_resources` | commands/module.rs | Fetch all items |
| `create_resource` | commands/module.rs | Create new item |

### Data Models (Rust)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub id: String,
    pub name: String,
    pub created_at: i64,
}
```

---

## File Changes Summary

### New Files
- `src-ui/src/lib/components/...`

### Modified Files
- `src-ui/src/lib/components/...`

### Files to Review
- `src-ui/src/lib/styles/design-system.css`

---

## Risks & Considerations

- <List any technical risks or edge cases>

---

## Acceptance Criteria

- [ ] All in-scope components implemented
- [ ] State management working correctly
- [ ] CSS matches prototype visual design
- [ ] Accessibility requirements met
- [ ] Unit tests passing
- [ ] No hardcoded design values
```

## Golden Rules

### 1. Never Generate a Plan Without Questions
Always ask clarifying questions before generating the implementation plan. Understanding is more important than speed.

### 2. Flag Uncertainties
If any aspect is unclear, mark the plan as "NOT READY TO IMPLEMENT" and list questions in a dedicated section.

### 3. Prioritize Reuse
Always search for existing components before planning new ones. Extend existing components when possible.

### 4. Design System Compliance
Ensure all planned CSS uses design tokens. Flag any prototype styles that use hardcoded values.

### 5. Separate Backend Plans
Backend implementation requires its own plan following the same analysis process.

### 6. Scope Discipline
Only plan for components marked as "in-scope" in the prototype README. Ignore contextual backdrop elements.

## Reference

- Prototype creation skill: `.claude/skills/prototype-interfaces/SKILL.md`
- Design system: `src-ui/src/lib/styles/design-system.css`
- Existing components: `src-ui/src/lib/components/`
tant in- Tauri commands: `src-tauri/src/commands/`
- Tauri storage: `src-tauri/src/storage/`
- Backend rules: `rules/05-backend-planning.md`
