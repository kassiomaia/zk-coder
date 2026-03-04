# Insecure ID generation in CustomProviderModal

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-custom-provider-modal.md
- **Location**: src-ui/src/lib/components/config/CustomProviderModal.svelte:32-37
- **Priority**: Medium
- **Category**: Security
- **Status**: Open

## Description
The generateId function uses simple regex replacement to create IDs from names, which could potentially generate non-unique IDs if two different names produce the same ID. Also, the function doesn't sanitize against potentially problematic characters beyond non-alphanumeric.

## Impact
- Potential ID collisions when auto-generating IDs from similar names
- Could lead to accessibility issues with duplicate element IDs
- May cause unexpected behavior in DOM operations that rely on unique IDs
- Limited sanitization could allow problematic characters in IDs

## Suggested Fix
Implement a more robust ID generation algorithm that ensures uniqueness and includes additional sanitization for special characters. Consider adding a uniqueness check when auto-generating IDs.

```typescript
// Enhanced ID generation function
function generateId(name: string): string {
  // Sanitize input more comprehensively
  const sanitized = name
    .toLowerCase()
    .replace(/[^a-z0-9-_]/g, '-')  // Allow hyphens and underscores
    .replace(/-+/g, '-')           // Replace multiple hyphens with single
    .replace(/^-|-$/g, '');       // Remove leading/trailing hyphens

  // Add timestamp to ensure uniqueness
  const timestamp = Date.now().toString(36);
  const random = Math.random().toString(36).substring(2, 6);

  return `${sanized}-${timestamp}-${random}`;
}

// Alternative using crypto API if available
function generateUniqueId(name: string): string {
  const sanitized = name
    .toLowerCase()
    .replace(/[^a-z0-9-_]/g, '-');

  // Use crypto.randomUUID() for better uniqueness guarantees
  const randomId = typeof crypto !== 'undefined' ?
    crypto.randomUUID().substring(0, 8) :
    Math.random().toString(36).substring(2, 10);

  return `${sanitized}-${randomId}`;
}
```

## Related Components
- CustomProviderModal.svelte
- Provider configuration system
- Form validation system

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-custom-provider-modal.md
- HTML ID specification: https://html.spec.whatwg.org/multipage/dom.html#the-id-attribute