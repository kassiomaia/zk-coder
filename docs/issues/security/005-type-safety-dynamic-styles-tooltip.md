# Type safety for dynamic styles in Tooltip

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04_Tooltip.svelte.md
- **Location**: src-ui/src/lib/components/shared/Tooltip.svelte:260
- **Priority**: Low
- **Category**: Security
- **Status**: Open

## Description
The positionStyle calculation uses direct string interpolation which could lead to XSS vulnerabilities if content isn't properly sanitized.

## Impact
- Direct string interpolation without sanitization could allow CSS injection
- Malicious content in position properties could execute arbitrary code
- Type safety issues could lead to runtime errors
- Potential for prototype pollution through crafted style strings

## Suggested Fix
Use template literals or a safer interpolation method for style properties.

```typescript
// Safer style object approach
interface PositionStyles {
  top?: string | number;
  left?: string | number;
  right?: string | number;
  bottom?: string | number;
  transform?: string;
}

function createSafeStyleObject(styles: PositionStyles): Record<string, string> {
  const safeStyles: Record<string, string> = {};

  Object.entries(styles).forEach(([key, value]) => {
    // Only allow safe CSS properties
    const allowedProperties = ['top', 'left', 'right', 'bottom', 'transform'];

    if (allowedProperties.includes(key) && value !== undefined) {
      // Sanitize numeric values
      const sanitizedValue = typeof value === 'number' ? `${value}px` : String(value);

      // Validate value doesn't contain dangerous characters
      if (!/[<>{}();'"\\]/.test(sanitizedValue)) {
        safeStyles[key] = sanitizedValue;
      }
    }
  });

  return safeStyles;
}

// Usage in Tooltip component
const safePositionStyle = createSafeStyleObject({
  top: `${position.y}px`,
  left: `${position.x}px`,
  transform: 'translate(-50%, -100%)'
});

// Alternative: CSS variable approach
function createSafeCSSVariables(styles: PositionStyles): Record<string, string> {
  return {
    '--tooltip-top': `${styles.top}px`,
    '--tooltip-left': `${styles.left}px`,
    '--tooltip-transform': styles.transform || ''
  };
}
```

## Related Components
- Tooltip.svelte
- Position tracking system
- CSS utilities

## References
- Original review: docs/reviews/code-reviewer/2026-03-04_Tooltip.svelte.md
- CSS Injection Prevention: https://cheatsheetseries.owasp.org/cheatsheets/CSS_Injection_Prevention_Cheat_Sheet.html
- TypeScript Style Safety: https://github.com/stylelint/stylelint/tree/main/lib/rules/no-unsafe-rule