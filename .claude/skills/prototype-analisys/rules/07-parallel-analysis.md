# Parallel Analysis Rules

These rules govern the parallel task architecture for prototype analysis.

## Mandatory Parallel Execution

**RULE:** All 4 analysis tasks MUST be launched in parallel for every prototype analysis.

### Task Launch Order

```
Location Check (sequential)
    │
    ▼ (if valid)
┌─────────────────────────────────────────────────┐
│           LAUNCH ALL TASKS IN PARALLEL          │
│                                                 │
│  Task 1: README    ──┐                          │
│  Task 2: Styles    ──┼──► Aggregate Reports    │
│  Task 3: Logic     ──┤                          │
│  Task 4: Structure ──┘                          │
└─────────────────────────────────────────────────┘
```

### Why Parallel?

1. **Efficiency:** Each task reads different files, no dependencies
2. **Speed:** 4x faster than sequential analysis
3. **Isolation:** Each task has clear boundaries
4. **Composability:** Reports can be aggregated consistently

## Task Specifications

### Task 1: README Analysis

**Agent Type:** `general-purpose`

**Prompt Template:**
```
You are analyzing a prototype's README.md file.

PROTOTYPE PATH: ./prototypes/{prototype-name}/
FILE TO READ: ./prototypes/{prototype-name}/README.md

Your task:
1. Read the README.md file
2. Extract the following information:
   - Feature purpose and business context
   - User stories (list all)
   - Functional requirements (list all)
   - Scope definition (in-scope vs contextual components)
   - User journeys with IF/THEN conditional logic
   - Accessibility requirements
   - Any missing sections

Output your report in this exact format:

```
README ANALYSIS REPORT
======================
Feature: {prototype-name}
Purpose: {one sentence description}

SCOPE:
  In-Scope:
    - {component 1}
    - {component 2}
  Contextual (do not implement):
    - {component A}
    - {component B}

USER STORIES:
  1. As a {user type}, I want to {action} so that {benefit}
  2. ...

FUNCTIONAL REQUIREMENTS:
  1. The system shall {requirement}
  2. ...

USER JOURNEYS:
  Journey 1: {name}
    - IF {condition} THEN {result}
    - IF {condition} THEN {result}

ACCESSIBILITY:
  - {requirement 1}
  - {requirement 2}

ISSUES FOUND:
  - {issue or "None"}
```

IMPORTANT:
- Do NOT make assumptions about missing content
- List "ISSUES FOUND: None" if documentation is complete
- Be specific about what is missing
```

---

### Task 2: Styles Analysis

**Agent Type:** `general-purpose`

**Prompt Template:**
```
You are analyzing a prototype's CSS files.

PROTOTYPE PATH: ./prototypes/{prototype-name}/
FILES TO READ:
  - ./prototypes/{prototype-name}/css/components.css
  - ./prototypes/{prototype-name}/css/animations.css (if exists)
  - ./prototypes/{prototype-name}/css/design-system.css

Your task:
1. Read all CSS files in the css/ directory
2. Analyze:
   - List all CSS classes with their purposes
   - Check for design token usage (var(--token-name))
   - Find hardcoded values (hex colors, px spacing)
   - Document animations (@keyframes)
   - Note responsive breakpoints

Design tokens are CSS custom properties like:
  - var(--color-primary-500)
  - var(--space-4)
  - var(--radius-lg)

Hardcoded values to flag:
  - Colors: #6366f1, rgb(99, 102, 241), rgba(...)
  - Spacing: 16px, 1rem (when not using tokens)

Output your report in this exact format:

```
STYLES ANALYSIS REPORT
======================
Prototype: {prototype-name}

CSS CLASSES:
  .class-name:
    Purpose: {description}
    Uses tokens: {yes/no}
  ...

DESIGN TOKEN USAGE:
  Tokens used:
    - var(--token-name)
  Hardcoded values found:
    - {file}:{line}: {value} → Suggested: var(--token)

ANIMATIONS:
  - @keyframes {name}: {description}
  - None found (if no animations)

RESPONSIVE:
  Breakpoints used: {list or "None"}

ISSUES FOUND:
  - {issue or "None"}
```

IMPORTANT:
- Flag ALL hardcoded colors and spacing
- Suggest appropriate design tokens for each hardcoded value
- Be thorough in listing all CSS classes
```

---

### Task 3: Logic/State Analysis

**Agent Type:** `general-purpose`

**Prompt Template:**
```
You are analyzing a prototype's JavaScript state management.

PROTOTYPE PATH: ./prototypes/{prototype-name}/
FILE TO READ: ./prototypes/{prototype-name}/js/state.js

Your task:
1. Read the state.js file
2. Analyze the Alpine.js state structure:
   - State properties (data)
   - Computed properties (getters)
   - Methods (functions)
   - Async operations
   - Side effects

3. Map each to Svelte 5 patterns:
   - State properties → $state()
   - Computed properties → $derived()
   - Methods → functions in <script>

Output your report in this exact format:

```
LOGIC/STATE ANALYSIS REPORT
===========================
Prototype: {prototype-name}

STATE PROPERTIES:
  - propertyName: {inferred type}
    Initial: {value}
    Svelte: let propertyName = $state({value})

COMPUTED PROPERTIES:
  - computedName
    Logic: {description}
    Svelte: let computedName = $derived({expression})

METHODS:
  - methodName({params}): {description}
    Svelte: function methodName({params}) {{ ... }}

ASYNC OPERATIONS:
  - {method name}: {description}
  - None found (if no async)

SIDE EFFECTS:
  - {description of side effect}
  - None found (if no side effects)

SVELTE 5 MAPPING:
  {propertyName} → $state()
  {computedName} → $derived()

ISSUES FOUND:
  - {issue or "None"}
```

IMPORTANT:
- Infer types from default values
- Document the purpose of each method
- Note any potential issues with the state structure
```

---

### Task 4: Structure/HTML Analysis

**Agent Type:** `general-purpose`

**Prompt Template:**
```
You are analyzing a prototype's HTML structure.

PROTOTYPE PATH: ./prototypes/{prototype-name}/
FILE TO READ: ./prototypes/{prototype-name}/index.html

Your task:
1. Read the index.html file
2. Analyze:
   - Component boundaries (logical groupings)
   - Alpine.js directives used (x-data, x-show, x-for, etc.)
   - Interactive elements (buttons, inputs, links)
   - Component hierarchy
   - Code quality violations

VIOLATIONS TO CHECK:
  - Inline styles: style="..." attributes (FAIL if found)
  - Inline scripts: <script> blocks with logic (FAIL if found)
  - Inline SVGs: <svg> elements (FAIL if found, should use Lucide)
  - Inline event handlers: onclick="..." (FAIL if found)

Alpine.js directives are OK:
  - x-data, x-show, x-for, x-model, x-bind, @click, etc.

Output your report in this exact format:

```
STRUCTURE ANALYSIS REPORT
=========================
Prototype: {prototype-name}

COMPONENT HIERARCHY:
  {ContainerName}
  ├── {ChildComponent1}
  │   └── {GrandchildComponent}
  ├── {ChildComponent2}
  └── {ChildComponent3}

ALPINE.JS DIRECTIVES:
  x-data:
    - {element}: {state name}
  x-show:
    - {element}: {condition}
  x-for:
    - {element}: {item} in {array}
  @click:
    - {element}: {method}

INTERACTIVE ELEMENTS:
  Buttons: {count}
    - {button text/purpose}
  Inputs: {count}
    - {input type}: {purpose}

VIOLATIONS CHECK:
  Inline styles: PASS/FAIL - {details}
  Inline scripts: PASS/FAIL - {details}
  Inline SVGs: PASS/FAIL - {details}
  Inline handlers: PASS/FAIL - {details}

ISSUES FOUND:
  - {issue or "None"}
```

IMPORTANT:
- Build a clear component hierarchy tree
- Be strict about violations - any inline style/script/SVG is a FAIL
- Note which Alpine.js directives map to which Svelte patterns
```

---

## Orchestrator Responsibilities

After all tasks complete, the orchestrator must:

### 1. Collect Reports

Gather all 4 reports in a structured format.

### 2. Cross-Validation

| Validation | Tasks | Check |
|------------|-------|-------|
| Scope Match | README + Structure | HTML components match README scope |
| State Consistency | Logic + Structure | Alpine.js state used in HTML exists in state.js |
| Style Coverage | Styles + Structure | CSS classes exist for HTML elements |
| Documentation | README | All required sections present |

### 3. Aggregate Issues

Combine all "ISSUES FOUND" sections:
```
AGGREGATED ISSUES:
  From README: [list]
  From Styles: [list]
  From Logic: [list]
  From Structure: [list]
```

### 4. Determine Status

```
IF all_issues is empty AND all_validations pass:
  status = "READY TO IMPLEMENT"
ELSE:
  status = "NOT READY TO IMPLEMENT"
  Add issues to "Clarification Questions" section
```

## Failure Handling

### If a Task Fails

1. Note which task failed
2. Mark status as "NOT READY TO IMPLEMENT"
3. Add to clarification questions: "Analysis of {area} failed: {error}"

### If Files Are Missing

1. README.md missing → Critical failure, cannot proceed
2. CSS files missing → Flag as issue, can proceed
3. state.js missing → Flag as issue, assume no JS logic
4. index.html missing → Critical failure, cannot proceed

## No Sequential Fallback

**RULE:** Do NOT fall back to sequential analysis if parallel tasks fail.

If parallel analysis fails, report the failure and mark the plan as "NOT READY TO IMPLEMENT" with specific questions about the failure.
