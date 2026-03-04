# [ISSUE-006] - Unsafe Error Handling in ManualModelList

## Priority
High

## Source
- **Review File**: `docs/reviews/code-reviewer/ManualModelList-2026-03-04.md`
- **Component/File**: `src-ui/src/lib/components/ManualModelList.svelte`
- **Line Numbers**: TBD

## Description
The ManualModelList component has unsafe error handling that doesn't properly validate model data. This could lead to runtime errors when malformed model data is loaded.

## Impact
Functionality - This affects:
- Application stability
- Data integrity
- User experience with crashes
- Debugging capability

## Suggested Fix
Implement safe error handling:

```typescript
interface Model {
  id: string;
  name: string;
  capabilities: string[];
  status: 'online' | 'offline' | 'busy';
  maxTokens?: number;
}

function validateModel(model: any): model is Model {
  return (
    model &&
    typeof model.id === 'string' &&
    typeof model.name === 'string' &&
    Array.isArray(model.capabilities) &&
    ['online', 'offline', 'busy'].includes(model.status)
  );
}

$: safeModels = $derived(() => {
  if (!models) return [];

  return models
    .filter(model => validateModel(model))
    .map(model => ({
      ...model,
      maxTokens: model.maxTokens || 4096 // Provide default
    }));
});

// In template
{#if modelsLoading}
  <div class="loading">Loading models...</div>
{:else if modelsError}
  <div class="error">
    <Icon name="alert" />
    <p>Failed to load models: {modelsError}</p>
    <Button on:click={retryLoad}>Retry</Button>
  </div>
{:else if safeModels.length === 0}
  <div class="empty-state">
    <p>No models available</p>
  </div>
{:else}
  {#each safeModels as model}
    <ModelCard model={model} />
  {/each}
{/if}
```

## Related Issues
- None