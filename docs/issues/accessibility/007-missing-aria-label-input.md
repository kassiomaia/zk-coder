# [ISSUE-007] - Missing ARIA Label for Input Fields

## Priority
High

## Source
- **Review File**: `docs/reviews/code-reviewer/2026-03-04-config-modal.md`
- **Component/File**: `src-ui/src/lib/components/config/ConfigModal.svelte`
- **Line Numbers**: 212-242

## Description
The ConfigModal component has input fields without proper ARIA labels. The password and API key inputs lack associated labels or aria-label attributes, making them inaccessible to screen reader users.

## Impact
Accessibility - This fails WCAG guidelines by:
- Violating WCAG 3.3.2 (Labels or Instructions)
- Making form fields inaccessible to screen reader users
- Preventing users with assistive technologies from understanding input purpose

## Suggested Fix
Add proper labels and ARIA attributes to input fields:

```typescript
<div class="input-group">
  <label for="api-key" class="sr-only">API Key</label>
  <Input
    id="api-key"
    aria-label="API Key"
    aria-describedby="apiKeyHelp"
    bind:value={apiKey}
    type="password"
    placeholder="sk-..."
  />
  <span id="apiKeyHelp" class="sr-only">Get your API key from the provider dashboard</span>
  <button
    type="button"
    aria-label="Toggle API key visibility"
    on:click={toggleApiKeyVisibility}
  >
    {showApiKey ? <Icon name="eye" /> : <Icon name="eye-slash" />}
  </button>
</div>
```

## Related Issues
- None