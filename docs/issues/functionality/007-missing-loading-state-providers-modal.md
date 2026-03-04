# Missing loading state in ProvidersModal

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04_ProvidersModal.svelte.md
- **Location**: src-ui/src/lib/components/config/ProvidersModal.svelte:51
- **Priority**: Low
- **Category**: Functionality
- **Status**: Open

## Description
No loading state is shown when providers are being fetched or configured.

## Impact
Users may perceive the application as unresponsive when loading providers, especially on slow network connections, leading to poor user experience.

## Suggested Fix
Add loading indicator when appropriate.

```typescript
// Example fix for ProvidersModal.svelte
let isLoading = false;
let providers = [];

// Load providers with loading state
const loadProviders = async () => {
  isLoading = true;
  try {
    providers = await api.getProviders();
  } catch (error) {
    console.error('Failed to load providers:', error);
    // Show error message
  } finally {
    isLoading = false;
  }
};

// On component mount
onMount(() => {
  loadProviders();
});

// In template:
{#if isLoading}
  <div class="loading-state">
    <Spinner />
    <span>Loading providers...</span>
  </div>
{:else}
  <!-- Existing providers list -->
{/if}
```

## Related Components
- ProvidersModal.svelte
- Spinner component
- API service (getProviders function)

## References
- Original review: docs/reviews/code-reviewer/2026-03-04_ProvidersModal.svelte.md