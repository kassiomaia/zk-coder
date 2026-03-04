# [ISSUE-012] - Missing ARIA Attributes for Password Input

## Priority
High

## Source
- **Review File**: `docs/reviews/code-reviewer-ConfigModal.svelte-2026-03-04.md`
- **Component/File**: `src-ui/src/lib/components/config/ConfigModal.svelte`
- **Line Numbers**: 212-242

## Description
The password inputs in the ConfigModal lack proper ARIA attributes for screen readers. Without appropriate labels and descriptions, visually impaired users cannot understand what information is being requested.

## Impact
Accessibility - This fails WCAG guidelines by:
- Violating WCAG 4.1.2 (Name, Role, Value)
- Making password fields inaccessible to screen reader users
- Preventing users with visual impairments from completing forms
- Creating barriers for assistive technology users

## Suggested Fix
Add comprehensive ARIA attributes to password inputs:

```typescript
<div class="form-group">
  <label for="password-api-key" class="sr-only">API Key Password</label>
  <div class="input-wrapper">
    <Input
      id="password-api-key"
      type="password"
      aria-label="API Key Password"
      aria-describedby="password-help password-requirements"
      bind:value={apiKey}
      placeholder="Enter your API key"
      autocomplete="off"
    />
    <button
      type="button"
      class="toggle-password"
      aria-label={showApiKey ? "Hide API key" : "Show API key"}
      on:click={toggleApiKeyVisibility}
    >
      {showApiKey ? <Icon name="eye" /> : <Icon name="eye-slash" />}
    </button>
  </div>
  <div id="password-help" class="sr-only">
    Enter your API key from the provider's dashboard
  </div>
  <div id="password-requirements" class="sr-only">
    API keys are sensitive and should not be shared
  </div>
</div>
```

## Related Issues
- None