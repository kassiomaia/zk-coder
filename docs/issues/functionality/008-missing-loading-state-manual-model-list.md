# Missing loading state in ManualModelList

## Metadata
- **Source Review**: docs/reviews/code-reviewer/ManualModelList-2026-03-04.md
- **Location**: ManualModelList.svelte:94-109
- **Priority**: Low
- **Category**: Functionality
- **Status**: Open

## Description
Remove model operation lacks loading state feedback, making the UI appear unresponsive.

## Impact
Users may attempt to remove multiple models simultaneously or think the application is frozen when performing remove operations, leading to confusion and potential duplicate operations.

## Suggested Fix
Add loading state for remove operation.

```typescript
// Example fix for ManualModelList.svelte
let removingModel = null;

const handleRemoveModel = async (modelId: string) => {
  if (removingModel === modelId) return;

  removingModel = modelId;

  try {
    await api.removeModel(modelId);
    // Remove from local state
    models = models.filter(m => m.id !== modelId);
  } catch (error) {
    console.error('Failed to remove model:', error);
    // Show error message
  } finally {
    removingModel = null;
  }
};

// In template:
{#each models as model (model.id)}
  <div class="model-item">
    <span>{model.name}</span>
    <button
      on:click={() => handleRemoveModel(model.id)}
      disabled={removingModel === model.id}
    >
      {#if removingModel === model.id}
        <Spinner size="small" />
        <span>Removing...</span>
      {:else}
        <span>Remove</span>
      {/if}
    </button>
  </div>
{/each}
```

## Related Components
- ManualModelList.svelte
- Spinner component
- API service (removeModel function)

## References
- Original review: docs/reviews/code-reviewer/ManualModelList-2026-03-04.md