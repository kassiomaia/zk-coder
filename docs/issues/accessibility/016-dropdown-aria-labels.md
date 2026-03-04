# No ARIA labels for dropdown menus in state.js

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-state.js-review.md
- **Location**: docs/prototypes/agent-coding-input/js/state.js:244-295
- **Priority**: Low
- **Category**: Accessibility
- **Status**: Open

## Description
No ARIA labels or keyboard navigation support for dropdown menus and interactive elements. The dropdown components in the prototype lack proper accessibility attributes, making them difficult to navigate for screen reader users.

## Impact
Users with visual impairments or those relying on keyboard navigation cannot effectively interact with dropdown menus, limiting their ability to use the full functionality of the application.

## Suggested Fix
Add proper ARIA attributes and keyboard support for accessibility.

```javascript
// Fix example - dropdown component with accessibility
class Dropdown {
  constructor(element) {
    this.element = element;
    this.button = element.querySelector('.dropdown-button');
    this.menu = element.querySelector('.dropdown-menu');

    // Set ARIA attributes
    this.button.setAttribute('aria-expanded', 'false');
    this.button.setAttribute('aria-haspopup', 'true');
    this.button.setAttribute('aria-controls', this.menu.id);

    // Add keyboard navigation
    this.button.addEventListener('keydown', (e) => {
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        this.toggle();
      } else if (e.key === 'ArrowDown') {
        e.preventDefault();
        this.open();
      }
    });

    // Add aria-labels to menu items
    this.menu.querySelectorAll('.dropdown-item').forEach((item, index) => {
      item.setAttribute('role', 'option');
      item.setAttribute('tabindex', '-1');
      item.setAttribute('aria-selected', 'false');
    });
  }
}
```

## Related Components
- Dropdown.js
- AgentCodingInput prototype
- state.js

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-state.js-review.md
- WAI-ARIA Combobox Pattern: https://www.w3.org/WAI/ARIA/apg/patterns/combobox/