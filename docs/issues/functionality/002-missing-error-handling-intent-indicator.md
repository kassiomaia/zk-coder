# [ISSUE-002] - Missing Error Handling in IntentIndicator

## Priority
High

## Source
- **Review File**: `docs/reviews/code-reviewer/2026-03-04-IntentIndicator.svelte.md`
- **Component/File**: `src-ui/src/lib/components/IntentIndicator.svelte`
- **Line Numbers**: TBD

## Description
The IntentIndicator component doesn't handle errors when determining the user's intent. This causes silent failures and makes the component fail gracefully when API calls or intent analysis fails.

## Impact
Functionality - This affects:
- Reliability of intent detection
- User experience with silent failures
- Debugging capability
- Component stability

## Suggested Fix
Add proper error handling:

```typescript
$: intent = $derived(() => {
  try {
    if (!intentService || !userMessage) return null;

    return intentService.detectIntent(userMessage);
  } catch (error) {
    console.error('Intent detection failed:', error);
    // Fallback to default intent
    return {
      type: 'unknown',
      confidence: 0,
      suggestions: []
    };
  }
});

// Also handle loading and error states
{#if intentLoading}
  <div class="intent-indicator loading">...</div>
{:else if intentError}
  <div class="intent-indicator error">
    <Icon name="alert" />
    <span>Unable to detect intent</span>
  </div>
{:else if intent}
  <IntentDisplay intent={intent} />
{:else}
  <div class="intent-indicator">...</div>
{/if}
```

## Related Issues
- None