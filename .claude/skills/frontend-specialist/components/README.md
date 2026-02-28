# Component Specifications

This directory contains detailed specifications for all UI components in the Coder application.

## How to Use These Specs

Each component specification includes:

1. **TypeScript Interface** - Type definitions for props and data
2. **Design Tokens** - Which CSS variables to use
3. **States** - All possible visual states
4. **Accessibility** - Required ARIA attributes and keyboard support
5. **Implementation** - Reference Svelte code

## Component List

| Component | Description |
|-----------|-------------|
| [sidebar.md](./sidebar.md) | Navigation sidebar |
| [header.md](./header.md) | Top header bar |
| [greeting-section.md](./greeting-section.md) | Personalized welcome |
| [suggestion-cards.md](./suggestion-cards.md) | Quick action cards |
| [input-box.md](./input-box.md) | Message input |
| [theme-toggle.md](./theme-toggle.md) | Theme switch |
| [chat-message.md](./chat-message.md) | Message bubbles |
| [code-block.md](./code-block.md) | Syntax-highlighted code |
| [file-attachment.md](./file-attachment.md) | File previews |
| [buttons.md](./buttons.md) | Button variants |

## Implementation Workflow

1. Read the component spec
2. Check `design-system.css` for exact token values
3. Use the template from `examples/`
4. Implement following the spec patterns
5. Verify all states work correctly
6. Test accessibility (keyboard, screen reader)
7. Test both themes (light/dark)
8. Test responsive breakpoints

## Component Checklist

Before submitting any component:

- [ ] TypeScript `Props` interface defined
- [ ] All design tokens referenced (no hardcoded values)
- [ ] CSS module created with matching name
- [ ] All states implemented (hover, active, disabled, etc.)
- [ ] ARIA attributes included
- [ ] Keyboard navigation works
- [ ] Focus-visible styles present
- [ ] Works in both themes
- [ ] Responsive breakpoints handled
- [ ] Component under 200 lines
