---
name: prototype-interfaces
description: Allow rapid prototyping to test workflow, requirements and ui/ux and also provide clear overview of the feature and provide capability to prepare the styles
---

# Prototype Interfaces Skill

Rapidly prototype UI/UX interfaces to clarify feature goals, validate workflows, and prepare styles for implementation.

## Purpose

This skill creates isolated HTML/CSS/JS prototypes using Alpine.js for state management. Prototypes are used to:

1. **Validate Workflows** - Test user flows before implementation
2. **Clarify Requirements** - Visualize features for product design review
3. **Prepare Styles** - Develop CSS patterns that align with the design system
4. **Document Decisions** - Capture design rationale for reference

## When to Use

- New feature requiring UI/UX exploration
- Complex user workflows needing validation
- Design system component development
- Proof of concept for interactions/animations
- Communicating design intent to product team

## Prototype Structure

```
prototypes/
└── <prototype-name>/
    ├── index.html          # Main HTML with Alpine.js components
    ├── README.md           # Documentation (purpose, decisions, components)
    ├── css/
    │   ├── design-system.css   # Design tokens (copy from project)
    │   ├── components.css      # Component-specific styles
    │   └── animations.css      # Keyframes and transitions
    └── js/
        ├── state.js            # State management
        └── *.js                # Feature-specific modules
```

## Key Principles

### 1. Isolation
- Prototypes must NOT interfere with the real application
- All code lives in `./prototypes/<prototype-name>/`
- Use standalone HTML/CSS/JS only

### 2. Technology Stack
- **HTML5** - Semantic markup
- **CSS** - CSS modules following project design system
- **Alpine.js** (CDN) - Lightweight reactivity for state management
- **Lucide Icons** (ESM CDN) - Icon library for all icons
- No build tools required - open `index.html` directly in browser

### 3. Design System Alignment
- Copy design tokens from `src-ui/src/lib/styles/design-system.css`
- Use semantic tokens (e.g., `--color-text-primary`) not raw values
- Maintain consistency with project styling conventions
- Reference `src-ui/src/lib/styles/design-system.css` for tokens

### 4. Code Separation (CRITICAL)
**Strictly minimize inline CSS and JavaScript in HTML files.**

**CSS Rules:**
- ALL styles must be in `css/` files, not in `<style>` tags or `style` attributes
- No inline `style="..."` attributes on elements
- No `<style>` blocks in `index.html` except for critical above-the-fold styles (rare)
- Use CSS classes defined in `components.css` for all styling

**JavaScript Rules:**
- ALL JavaScript logic must be in `js/` files, not in `<script>` tags in HTML
- No inline event handlers like `onclick="..."` - use Alpine.js directives instead
- No `<script>` blocks with logic in `index.html` except for:
  - CDN imports (Alpine.js, fonts)
  - Module script imports referencing external files
- Alpine.js inline expressions (`@click="method()"`, `x-show="condition"`) are acceptable as they reference state defined in `js/state.js`

**Icon Rules (CRITICAL):**
- **NO inline SVG elements** in HTML - use Lucide Icons library instead
- Import Lucide Icons via ESM CDN: `<script src="https://unpkg.com/lucide@latest"></script>`
- Use `<i data-lucide="icon-name"></i>` for icons
- Initialize icons with `lucide.createIcons()` in `js/state.js` init function
- Available icons: https://lucide.dev/icons/

**Rationale:** Separation improves maintainability, makes styles reusable, and keeps HTML semantic and readable. Using an icon library ensures consistency and reduces HTML bloat.

### 5. Scope Definition (CRITICAL)
**Every prototype MUST clearly define what components are IN SCOPE vs. CONTEXTUAL BACKDROP.**

Many prototypes include contextual UI elements (app shell, sidebar, navigation) that provide visual context but are NOT part of the feature being prototyped. These MUST be clearly documented.

**Scope Rules:**
- The README Overview MUST include a note distinguishing in-scope components from contextual backdrop
- Component hierarchy MUST mark contextual elements (e.g., `[Contextual - Not in Scope]`)
- Component Responsibilities table SHOULD include a Scope column
- Only document interactions/states for in-scope components
- Do NOT document user journeys for contextual UI elements

**Format for README:**
```markdown
## Overview

> **Note**: This prototype focuses on [feature name]. The [contextual elements, e.g., app shell, sidebar]
> shown in the prototype are purely contextual backdrop and not part of the feature scope.

### Component Hierarchy

[Contextual Elements - Not in Scope]
└── ...

[Feature Components - IN SCOPE]
└── ...
```

**Examples:**
- Config flow prototype: Modals are in scope, app shell is contextual
- Settings page prototype: Settings sections are in scope, navigation is contextual
- Feature modal prototype: Modal content is in scope, parent app is contextual

**Rationale:** Clear scope definition prevents confusion about what needs to be implemented and what's just visual context for the prototype.

### 6. Documentation
Every prototype MUST include a `README.md` with:

**Required Sections:**
- **Overview** - Feature description and business context (MUST include scope note)
- **User Stories** - Who uses this feature and why
- **Functional Requirements** - What the feature must do
- **UI Components** - Component hierarchy with scope markers
- **Interaction Flows** - Step-by-step user journeys
- **User Journeys and Logic** - Conditional logic in natural language with ASCII diagrams
- **Visual Design Specifications** - Colors, typography, spacing, layout
- **States and Variants** - Component states (default, hover, active, disabled, error, loading)
- **Accessibility Requirements** - ARIA labels, keyboard navigation, focus management
- **Implementation Notes** - Technical considerations for developers

**User Journeys and Logic Section:**
This section documents all user journeys using natural language conditionals (IF/THEN) and ASCII flow diagrams. It captures the complete behavioral logic that product design must understand.

Format:
```markdown
## User Journeys and Logic

### Journey 1: <Journey Name>

**Conditional Logic:**
- IF user clicks [element] THEN [action/response]
- IF [condition] is true THEN [behavior]
- IF [condition] is false THEN [alternative behavior]
- IF [property] is not available THEN [fallback behavior]

**Flow Diagram:**
```
    [Start]
       │
       ▼
   ┌───────────┐
   │ Condition │
   └─────┬─────┘
         │
    ┌────┴────┐
    │         │
    ▼         ▼
 [Yes]      [No]
    │         │
    ▼         ▼
[Action A] [Action B]
    │         │
    └────┬────┘
         │
         ▼
      [End]
```
```

**Documentation Tone:**
- Write at a **feature design level**, not implementation level
- Focus on WHAT the feature does and WHY, not HOW it's prototyped
- Use language that product designers and developers can understand
- Avoid prototype-specific implementation details (Alpine.js, file structure)
- Describe behavior and appearance, not the code that produces it
- Use clear IF/THEN statements for all conditional logic
- Include ASCII diagrams for visual clarity of flows

## Workflow

### Step 1: Analyze Requirements
- Understand the feature requirements
- Identify user flows and states
- Review existing design system tokens
- Check for similar patterns in the codebase

### Step 2: Set Up Structure
```bash
mkdir -p prototypes/<prototype-name>/{css,js}
touch prototypes/<prototype-name>/{index.html,README.md}
touch prototypes/<prototype-name>/css/{design-system.css,components.css,animations.css}
touch prototypes/<prototype-name>/js/state.js
```

### Step 3: Copy Design Tokens
Copy relevant design tokens from the project's design system to maintain consistency.

### Step 4: Build HTML Structure
- Use semantic HTML elements
- Add Alpine.js directives (`x-data`, `x-show`, `x-bind`, etc.)
- Structure for accessibility (ARIA labels, roles)

### Step 5: Style Components
- Use CSS custom properties from design system
- Follow BEM-like naming for component classes
- Keep styles scoped to the prototype

### Step 6: Add Interactivity
- Use Alpine.js for state management
- Keep JavaScript minimal and focused
- Document state structure in `state.js`

### Step 7: Document Everything
Complete the `README.md` with all required sections.

## Alpine.js Patterns

### State Management
```javascript
function prototypeState() {
    return {
        // State properties
        isOpen: false,
        items: [],
        selectedItem: null,

        // Computed properties (getters)
        get itemCount() {
            return this.items.length;
        },

        // Methods
        toggle() {
            this.isOpen = !this.isOpen;
        },

        init() {
            // Initialization logic
        }
    }
}
```

### Common Directives
- `x-data` - Initialize component state
- `x-show` - Toggle visibility
- `x-bind:class` - Dynamic classes
- `x-on:click` - Event handlers
- `x-for` - Loop rendering
- `x-if` - Conditional rendering
- `x-model` - Two-way binding
- `x-transition` - Animation helpers

## Example

See `./example/` folder for:
- `folder-structure.md` - Template for prototype directory
- `building-instructions.md` - Detailed guide for creating prototypes

## Reference Prototypes

Existing prototypes for reference:
- `docs/tasks/refs/config-flow/` - LLM provider configuration flow

## Quality Checklist

Before considering a prototype complete:

- [ ] `README.md` fully documented at feature design level
- [ ] Overview includes clear scope note (in-scope vs. contextual)
- [ ] Component hierarchy marks scope for all elements
- [ ] Component Responsibilities table includes Scope column
- [ ] User journeys documented with IF/THEN logic
- [ ] ASCII flow diagrams included for all journeys
- [ ] Design system tokens used (not hardcoded values)
- [ ] Semantic HTML with accessibility attributes
- [ ] Alpine.js state clearly structured in external JS file
- [ ] All user flows documented
- [ ] Works when opening `index.html` directly in browser
- [ ] Responsive considerations noted
- [ ] Implementation notes provided for developers
- [ ] **No inline CSS** - all styles in `css/` files
- [ ] **No inline JavaScript logic** - all logic in `js/` files
- [ ] **No inline SVG** - using Lucide Icons library
- [ ] README avoids prototype implementation details
