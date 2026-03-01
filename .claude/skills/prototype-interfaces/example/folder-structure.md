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

**Required sections:**

```markdown
# <Feature Name> - Prototype

## Overview
Brief description of what this prototype demonstrates.

## Goals
1. Goal 1
2. Goal 2
3. Goal 3

## Features
- Feature 1: Description
- Feature 2: Description

## Components Included
- Component A
- Component B

## Components Excluded
- Component C (reason: not relevant to this prototype)
- Component D (reason: exists elsewhere, using placeholder)

## State Structure
\`\`\`javascript
const state = {
    propertyName: defaultValue,
    // ... other properties
};
\`\`\`

## Interactions
1. User does X → System responds with Y
2. User does A → System responds with B

## Design Decisions
- Decision 1: Rationale
- Decision 2: Rationale

## Notes for Implementation
- Consideration 1
- Consideration 2

## Future Enhancements (Optional)
- [ ] Enhancement 1
- [ ] Enhancement 2
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
