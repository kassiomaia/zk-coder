# Prototype Folder Structure Template

This template shows the standard structure for all prototypes.

## Directory Layout

```
prototypes/
└── <prototype-name>/           # Use kebab-case naming
    ├── index.html              # Main entry point - open in browser
    ├── README.md               # Required documentation
    ├── css/
    │   ├── design-system.css   # Design tokens from project
    │   ├── components.css      # Component styles
    │   └── animations.css      # Keyframes and transitions (optional)
    └── js/
        ├── state.js            # Central state management
        └── <feature>.js        # Feature-specific modules (optional)
```

## File Descriptions

### `index.html`

Main HTML file containing:
- Document structure with proper `<head>` and `<body>`
- Font imports (Google Fonts or system fonts)
- Alpine.js CDN script (deferred)
- CSS file imports in correct order
- HTML structure with Alpine.js directives

**Template:**
```html
<!DOCTYPE html>
<html lang="en" data-theme="dark">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title><Feature Name> - Coder Prototype</title>

    <!-- Fonts -->
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=JetBrains+Mono:wght@400;500&display=swap" rel="stylesheet">

    <!-- Alpine.js -->
    <script defer src="https://cdn.unpkg.com/alpinejs@3.x.x/dist/cdn.min.js"></script>

    <!-- Styles (order matters) -->
    <link rel="stylesheet" href="css/design-system.css">
    <link rel="stylesheet" href="css/components.css">
    <link rel="stylesheet" href="css/animations.css">
</head>
<body x-data="prototypeState()">
    <!-- Prototype content here -->

    <script src="js/state.js"></script>
</body>
</html>
```

### `README.md`

Feature design specification document. This is NOT implementation documentation - it describes the feature at a product/design level for stakeholders and developers.

**Required sections:**

```markdown
# <Feature Name>

## Overview
Business context and feature description. What problem does this solve? Who is it for?

## User Stories
- As a [user type], I want to [action] so that [benefit]
- As a [user type], I want to [action] so that [benefit]

## Functional Requirements
1. The system shall [functional requirement]
2. The system shall [functional requirement]
3. The system shall [functional requirement]

## UI Components

### <Component Name>
**Purpose:** What this component does
**Responsibilities:**
- Responsibility 1
- Responsibility 2

**Variants:**
- Default: Description
- Active: Description
- Disabled: Description

### <Component Name>
...

## Interaction Flows

### Flow 1: <Flow Name>
1. User initiates action by [trigger]
2. System responds with [response]
3. User can then [next action]
4. Flow completes when [end condition]

### Flow 2: <Flow Name>
...

## User Journeys and Logic

This section documents all user journeys using natural language conditionals and ASCII flow diagrams. It captures the complete behavioral logic that product design must understand.

### Journey 1: <Journey Name>

**Description:** Brief description of this user journey

**Conditional Logic:**
- IF user clicks [element] THEN [action/response occurs]
- IF [condition] is true THEN [behavior is triggered]
- IF [condition] is false THEN [alternative behavior]
- IF [property] is not available THEN [fallback behavior]
- IF [validation] fails THEN [error state is shown]
- IF user cancels THEN [return to previous state]

**Flow Diagram:**
```
    ┌─────────────┐
    │   [Start]   │
    └──────┬──────┘
           │
           ▼
    ┌──────────────┐
    │ User Action  │
    └──────┬───────┘
           │
           ▼
    ┌──────────────┐
    │  Condition?  │
    └──────┬───────┘
           │
      ┌────┴────┐
      │         │
     Yes        No
      │         │
      ▼         ▼
┌──────────┐ ┌──────────┐
│ Action A │ │ Action B │
└────┬─────┘ └────┬─────┘
     │            │
     └─────┬──────┘
           │
           ▼
    ┌─────────────┐
    │    [End]    │
    └─────────────┘
```

### Journey 2: <Journey Name>

**Description:** Brief description of this user journey

**Conditional Logic:**
- IF user selects [option] THEN [consequence]
- IF [data] is loading THEN [show loading state]
- IF [data] load fails THEN [show error message]
- IF user retries THEN [reload data]

**Flow Diagram:**
```
         [Start]
            │
            ▼
     ┌──────────────┐
     │ Fetch Data   │
     └──────┬───────┘
            │
       ┌────┴────┐
       │         │
    Success   Failure
       │         │
       ▼         ▼
 ┌──────────┐ ┌──────────┐
 │ Show     │ │ Show     │
 │ Content  │ │ Error    │
 └────┬─────┘ └────┬─────┘
      │            │
      │     ┌──────┴──────┐
      │     │             │
      │   Retry        Cancel
      │     │             │
      │     └──────┬──────┘
      │            │
      └─────┬──────┘
            │
            ▼
         [End]
```

### Journey 3: <Journey Name>

...additional journeys as needed...

## Visual Design Specifications

### Layout
- Component arrangement and hierarchy
- Spacing and alignment rules
- Responsive breakpoints

### Typography
- Font families, sizes, weights per element
- Line heights and letter spacing

### Colors
- Background colors per element
- Text colors per context
- Border and divider colors
- State colors (hover, active, error, success)

### Spacing
- Padding and margins per component
- Gap between elements

## States and Variants

### <Component> States
| State | Appearance | Trigger |
|-------|------------|---------|
| Default | Description | Initial render |
| Hover | Description | Mouse enter |
| Active | Description | Mouse down |
| Focus | Description | Tab navigation |
| Disabled | Description | Condition |
| Loading | Description | Async operation |
| Error | Description | Validation failure |

## Accessibility Requirements
- Keyboard navigation: [specifics]
- Screen reader announcements: [specifics]
- Focus management: [specifics]
- Color contrast: [WCAG level]

## Implementation Notes
Technical considerations for development:
- API requirements
- State management approach
- Performance considerations
- Edge cases to handle

## Out of Scope
- Feature/behavior explicitly not included
- Future considerations deferred
```

### `css/design-system.css`

Copy from `src-ui/src/lib/styles/design-system.css`. Contains:
- CSS custom properties (design tokens)
- Color palette
- Typography scale
- Spacing scale
- Border radius
- Shadows
- Transitions
- Z-index scale
- Base reset styles

**Do NOT modify design tokens** - keep them consistent with the project.

### `css/components.css`

Component-specific styles using design tokens:
- Use semantic tokens (e.g., `var(--color-text-primary)`)
- Follow naming conventions from project
- Keep styles modular and reusable

### `css/animations.css` (Optional)

Animation definitions:
- `@keyframes` rules
- Animation utility classes
- Transition utilities

### `js/state.js`

Central state management:
- State object with all reactive properties
- Computed properties as getters
- Methods for state mutations
- Initialization logic

## Naming Conventions

| Type | Convention | Example |
|------|------------|---------|
| Prototype folder | kebab-case | `config-flow`, `thread-sidebar` |
| CSS classes | kebab-case | `.provider-card`, `.model-grid` |
| JS functions | camelCase | `prototypeState()`, `handleClick()` |
| State properties | camelCase | `isOpen`, `selectedItem` |

## File Size Guidelines

Keep files focused and maintainable:
- `index.html`: < 300 lines (consider splitting if larger)
- `components.css`: < 400 lines per component file
- `state.js`: < 150 lines (split into modules if larger)

## Checklist Before Starting

- [ ] Create folder structure
- [ ] Copy `design-system.css` from project
- [ ] Set up `index.html` with template
- [ ] Create empty `components.css`
- [ ] Create `state.js` with initial state
- [ ] Create `README.md` with required sections
