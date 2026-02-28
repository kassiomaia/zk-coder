# HTML References

This directory contains HTML reference files for learning component patterns.

## Purpose

HTML references serve as learning materials to understand:
- Real-world component structures
- Layout patterns and techniques
- Styling approaches
- Accessibility implementations

## How to Add References

### 1. Create a new HTML file

Use descriptive names like:
- `button-primary.html`
- `modal-dialog.html`
- `navigation-sidebar.html`
- `chat-message-bubble.html`
- `code-block-highlighted.html`

### 2. Include complete, self-contained HTML

The file should work standalone with inline styles:

```html
<!--
  Reference: Button with Loading State
  Pattern: Shows loading spinner while maintaining button width
  Technique: CSS Grid overlay for spinner positioning

  Source: [URL or "internal reference"]
  Date: YYYY-MM-DD
-->
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Button with Loading State</title>
  <style>
    /* Reset */
    *, *::before, *::after {
      box-sizing: border-box;
    }

    /* Component styles */
    .button {
      /* ... */
    }

    .button--loading {
      /* ... */
    }
  </style>
</head>
<body>
  <button class="button">
    <span class="button__text">Submit</span>
    <span class="button__spinner"></span>
  </button>
</body>
</html>
```

### 3. Required Comment Header

Every reference file must include:
- **Title**: What component/pattern it demonstrates
- **Pattern**: Brief description of the pattern
- **Technique**: Notable techniques used
- **Source**: Where the reference came from (optional)
- **Date**: When it was added

## File Naming Convention

```
{component}-{variant/feature}.html

Examples:
- button-primary.html
- button-loading-state.html
- card-with-actions.html
- modal-centered.html
- sidebar-collapsible.html
- input-with-validation.html
```

## Using References

When implementing components:

1. **Browse references** for similar patterns
2. **Extract structural ideas**, not exact styles
3. **Adapt to use design system tokens**
4. **Ensure accessibility compliance**
5. **Test in both themes**

## Reference Categories

### Buttons
- `button-primary.html` - Primary button styles
- `button-secondary.html` - Secondary/outline buttons
- `button-loading.html` - Loading state buttons
- `button-icon.html` - Icon-only buttons

### Forms
- `input-text.html` - Text input fields
- `input-textarea.html` - Multi-line inputs
- `select-dropdown.html` - Select menus
- `checkbox-toggle.html` - Checkbox and toggle switches

### Layout
- `sidebar-collapsible.html` - Collapsible sidebar
- `header-with-actions.html` - Header layouts
- `grid-responsive.html` - Responsive grid patterns
- `split-view.html` - Split pane layouts

### Overlays
- `modal-dialog.html` - Modal dialogs
- `dropdown-menu.html` - Dropdown menus
- `tooltip.html` - Tooltip patterns
- `toast-notification.html` - Toast messages

### Data Display
- `card-container.html` - Card components
- `list-item.html` - List items with actions
- `table-responsive.html` - Responsive tables
- `code-block.html` - Syntax-highlighted code

### Chat
- `chat-message-user.html` - User message bubbles
- `chat-message-assistant.html` - AI message bubbles
- `chat-input.html` - Message input with actions
- `typing-indicator.html` - Typing animation

## Contributing

When adding references:

1. Keep files small and focused
2. Use semantic HTML
3. Include accessibility attributes
4. Test in isolation
5. Update this README if adding new categories
