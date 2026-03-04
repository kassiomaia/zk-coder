# Hard-coded Animation Delay Style Prop in ConversationItem

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04_ConversationItem.svelte.md
- **Location**: src-ui/src/lib/components/chat/sidebar/ConversationItem.svelte:81
- **Priority**: Medium
- **Category**: Design & Architecture
- **Status**: Open

## Description
The style prop is directly assigned to the li element without validation or sanitization. The component directly applies a style prop to the list element, which could potentially allow unsafe CSS to be injected if the prop value comes from untrusted sources. While this might be safe in the current implementation, it's a security risk that should be addressed by validating and sanitizing the style string before applying it.

## Impact
- Potential XSS vulnerability if style values come from untrusted sources
- Risk of invalid CSS breaking component styling
- Difficult to debug CSS issues caused by malformed style strings
- Inconsistent with the application's security best practices

## Suggested Fix
Create a helper function to validate and sanitize style strings before applying them.

```javascript
// Helper function to sanitize style strings
function sanitizeStyles(styleString: string): string {
  if (!styleString) return '';

  // Basic validation - only allow specific CSS properties
  const allowedProperties = [
    'animation-delay', 'animation-duration', 'animation-name',
    'opacity', 'transform', 'transition'
  ];

  const stylePairs = styleString.split(';');
  const validStyles = stylePairs
    .map(pair => pair.trim())
    .filter(pair => {
      if (!pair) return false;
      const property = pair.split(':')[0].trim();
      return allowedProperties.includes(property);
    })
    .filter(Boolean);

  return validStyles.join(';');
}

// Usage in component
const sanitizedStyle = sanitizeStyles(animationDelayStyle);
```

## Related Components
- src-ui/src/lib/components/chat/sidebar/ConversationItem.svelte
- src-ui/src/lib/utils/styleSanitizer.ts (would need to be created)

## References
- Original review: docs/reviews/code-reviewer/2026-03-04_ConversationItem.svelte.md