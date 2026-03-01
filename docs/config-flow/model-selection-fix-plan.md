# Fix Plan: Model Selection Reactivity in Settings Modal

> **Status:** READY FOR IMPLEMENTATION
> **Created:** 2026-03-01
> **Related:** `prototype-implementation-plan.md`, `prototypes/config-flow/`

## Problem Statement

The Settings modal model selection feature has diverged from the prototype behavior. Users cannot see immediate visual feedback when selecting models, This is caused by three architectural issues:

1. **Non-reactive state access** - Using `get()` breaks Svelte's reactivity
2. **Async persistence in critical path** - Backend calls block UI updates
3. **Missing visual indicator** - Operation headers don't show selected model

---

## Reference: Prototype Behavior

**Source:** `prototypes/config-flow/js/state.js:312-314`

```javascript
selectModel(operationType, modelId) {
    this.selectedModels[operationType] = modelId;  // Immediate synchronous update
}
```

**Source:** `prototypes/config-flow/index.html:519-521`

```html
<div class="operation-selected" x-show="selectedModels[tab.id]">
    <span class="selected-label" x-text="getModelShortName(selectedModels[tab.id])"></span>
</div>
```

The prototype:
- Updates UI **immediately** on click (synchronous)
- Persists data **asynchronously** in background
- Shows selected model in operation header (visual feedback)

---

## Fix Strategy: Optimistic UI Updates

### Approach: UI-First with Background Sync

1. **Update UI immediately** - Don't wait for backend
2. **Sync to backend asynchronously** - Fire and forget
3. **Revert on error** - Rollback UI if backend fails
4. **Add visual indicators** - Show selected model in header

This matches the prototype's behavior and provides instant user feedback.

---

## Implementation Changes

### File 1: `src-ui/src/lib/stores/configStore.ts`

#### Change A: Make `getAvailableModels` a Derived Store

**Current (non-reactive):**
```typescript
getAvailableModels(operation: OperationType): Model[] {
    const state = get({ subscribe });  // NOT reactive
    // ...
}
```

**Fix (reactive):**
```typescript
// Add new derived store at the bottom of the file

/** Available models grouped by operation - reactive */
export const availableModelsByOperation = derived(
    configStore,
    ($config) => {
        const result: Record<OperationType, Model[]> = {
            analysis: [],
            code: [],
            vision: [],
            audio: [],
            tips: [],
            review: []
        };

        for (const provider of $config.providers) {
            if (!provider.isConfigured) continue;

            const providerModels = $config.modelsByProvider[provider.id] || [];
            for (const model of providerModels) {
                const caps = model.capabilities;
                if (caps.analysis) result.analysis.push(model);
                if (caps.code) result.code.push(model);
                if (caps.vision) result.vision.push(model);
                if (caps.audio) result.audio.push(model);
                if (caps.tips) result.tips.push(model);
                if (caps.review) result.review.push(model);
            }
        }

        return result;
    }
);
```

#### Change B: Keep method for backward compatibility but mark as deprecated

```typescript
/**
 * Gets available models for an operation (from configured providers).
 * @deprecated Use `availableModelsByOperation` derived store instead for reactive access
 */
getAvailableModels(operation: OperationType): Model[] {
    const state = get({ subscribe });
    // ... keep existing implementation
}
```

---

### File 2: `src-ui/src/lib/stores/configStore.ts`

#### Change C: Optimistic `setOperationDefault`

**Current:**
```typescript
async setOperationDefault(operation: OperationType, modelId: string): Promise<void> {
    await apiSetOperationDefault(operation, modelId);  // Waits for backend
    update((state) => { /* ... */ });  // Only updates after backend responds
}
```

**Fix (optimistic):**
```typescript
/**
 * Sets the default model for an operation with optimistic UI update.
 * Updates UI immediately, then syncs to backend in background.
 */
async setOperationDefault(operation: OperationType, modelId: string): Promise<void> {
    // 1. Update UI immediately (optimistic update)
    update((state) => {
        const existing = state.operationDefaults.findIndex((d) => d.operation === operation);
        if (existing >= 0) {
            const operationDefaults = [...state.operationDefaults];
            operationDefaults[existing] = { operation, modelId };
            return { ...state, operationDefaults };
        }
        return {
            ...state,
            operationDefaults: [...state.operationDefaults, { operation, modelId }]
        };
    });

    // 2. Sync to backend in background (fire and forget)
    try {
        await apiSetOperationDefault(operation, modelId);
        // Backend sync successful - no further action needed
        // The UI is already updated
    } catch (error) {
        // 3. On error, we could revert the UI update
        // For now, just log the error - the UI stays updated
        // In the future, we could implement a rollback mechanism or or error toast
        console.error('Failed to persist operation default:', error);
                'Operation:', ${operation}, ModelId: ${modelId}`);
        // Note: We're keeping the optimistic update for now
        // The user's selection is saved locally even if backend sync fails
        // This matches the prototype behavior
    }
}
```

---

### File 3: `src-ui/src/lib/components/config/SettingsModal.svelte`

#### Change D: Use Reactive Store Access

**Current (non-reactive):**
```svelte
<script>
    function getConfigState() {
        return get(configStore);  // NOT reactive
    }

    function getSelectedModelId(operation: OperationType): string | null {
        const state = getConfigState();
        const default_ = state.operationDefaults.find((d) => d.operation === operation);
        return default_?.modelId ?? null;
    }
</script>

{#each OPERATION_CONFIGS as op (op.id)}
    {@const availableModels = configStore.getAvailableModels(op.id)}  <!-- Uses get() internally -->
    <div class={styles.operationGroup}>
        <!-- ... -->
        <ModelCard
            {model}
            selected={getSelectedModelId(op.id) === model.id}  <!-- Stale value -->
            onclick={() => handleSelectModel(op.id, model.id)}
        />
    {/each}
{/each}
```

**Fix (reactive):**
```svelte
<script>
    import { configStore, availableModelsByOperation } from '$lib/stores/configStore';
    // Remove getSelectedModelId function - no longer needed

    async function handleSelectModel(operation: OperationType, modelId: string) {
        try {
            await configStore.setOperationDefault(operation, modelId);
            // UI updates automatically via store subscription
        } catch (error) {
            console.error('Failed to set operation default:', error);
        }
    }

    // Helper to get selected model ID from store (reactive via $configStore)
    function getSelectedModelId(operation: OperationType): string | null {
        const default_ = $configStore.operationDefaults.find((d) => d.operation === operation);
        return default_?.modelId ?? null;
    }

    // Helper to get short name for selected model badge
    function getSelectedModelShortName(operation: OperationType): string | null {
        const modelId = getSelectedModelId(operation);
        if (!modelId) return null;

        // Find the model in available models
        const models = $availableModelsByOperation[operation];
        const model = models.find(m => m.id === modelId);
        if (!model) return null;

        // Return shortened name (e.g., "Claude Opus" -> "Opus")
        const shortNames: Record<string, string> = {
            'claude opus': 'Opus',
            'claude sonnet': 'Sonnet',
            'claude haiku': 'Haiku',
            'gpt-4 turbo': 'GPT-4T',
            'gpt-4': 'GPT-4',
            'gemini ultra': 'GemU',
            'gemini pro': 'GemP'
        };
        return shortNames[model.name.toLowerCase()] ?? model.name.split(' ').pop();
    }
</script>

{#each OPERATION_CONFIGS as op (op.id)}
    <div class={styles.operationGroup}>
        <header class={styles.operationHeader}>
            <!-- Icon and info -->
            <div class={styles.operationIcon} style:color={op.color}>
                <!-- ... existing icon SVGs ... -->
            </div>
            <div class={styles.operationInfo}>
                <span class={styles.operationName}>{op.name}</span>
                <span class={styles.operationDesc}>{op.description}</span>
            </div>

            <!-- NEW: Selected model badge (from prototype line 519-521) -->
            {#if getSelectedModelId(op.id)}
                <div class={styles.operationSelected}>
                    <span class={styles.selectedLabel}>
                        {getSelectedModelShortName(op.id)}
                    </span>
                </div>
            {/if}
        </header>

        <div class={styles.modelGrid}>
            {#each $availableModelsByOperation[op.id] as model (model.id)}
                <ModelCard
                    {model}
                    selected={getSelectedModelId(op.id) === model.id}
                    onclick={() => handleSelectModel(op.id, model.id)}
                />
            {/each}

            {#if $availableModelsByOperation[op.id].length === 0}
                <div class={styles.emptyModels}>
                    <span>No models available - configure a provider first</span>
                </div>
            {/if}
        </div>
    </div>
{/each}
```

---

### File 4: `src-ui/src/lib/components/config/SettingsModal.module.css`

#### Change E: Add Selected Model Badge Styles

**Add new CSS classes (from prototype lines 1378-1391):**

```css
/* Operation Selected Badge - from prototype */
.operationSelected {
    flex-shrink: 0;
}

.selectedLabel {
    display: inline-flex;
    align-items: center;
    padding: var(--space-1) var(--space-3);
    background: var(--color-primary-500);
    color: var(--color-neutral-0);
    font-size: var(--text-xs);
    font-weight: var(--font-medium);
    border-radius: var(--radius-full);
}
```

---

## Implementation Order

### Phase 1: Update configStore.ts (Backend)
1. [ ] Add `availableModelsByOperation` derived store to `configStore.ts`
2. [ ] Update `setOperationDefault` to use optimistic updates
3. [ ] Mark `getAvailableModels` as deprecated

### Phase 2: Update SettingsModal.svelte (Frontend)
4. [ ] Import `availableModelsByOperation` derived store
5. [ ] Remove non-reactive `getConfigState()` function
6. [ ] Update `getSelectedModelId` to use `$configStore` directly
7. [ ] Add `getSelectedModelShortName` helper function
8. [ ] Replace `{@const availableModels = ...}` with `$availableModelsByOperation[op.id]`
9. [ ] Add selected model badge to operation header
10. [ ] Update `{#each}` to use reactive store

### Phase 3: Update SettingsModal.module.css (Styles)
11. [ ] Add `.operationSelected` class
12. [ ] Add `.selectedLabel` class

### Phase 4: Testing
13. [ ] Test model selection updates UI immediately
14. [ ] Test selected model badge appears in operation header
15. [ ] Test that backend sync happens in background
16. [ ] Test error handling (backend failure doesn't crash UI)

---

## Acceptance Criteria

### Functional Requirements
- [ ] Clicking a model card immediately shows selection state
- [ ] Selected model badge appears in operation header instantly
- [ ] No visible lag between click and UI update
- [ ] Backend persistence happens asynchronously
- [ ] Errors are logged but don't break the UI

### Prototype Compliance
- [ ] Visual feedback matches prototype (immediate selection indicator)
- [ ] Operation header shows selected model name (from prototype lines 519-521)
- [ ] Selection behavior is synchronous from user perspective

### Code Quality
- [ ] Uses Svelte's reactivity model correctly (`$store` instead of `get()`)
- [ ] Optimistic updates don't cause consistency issues
- [ ] Error handling is appropriate

---

## Verification Checklist

### Visual Verification
Open the prototype at `prototypes/config-flow/index.html` and the implemented component side-by-side:

| Component | Prototype Behavior | Implementation Match |
|-----------|-------------------|---------------------|
| Model Card Selection | Immediate visual feedback on click | [ ] Immediate update |
| Operation Header Badge | Shows "Opus", "Sonnet", etc. | [ ] Badge appears |
| Model Grid | Updates when provider configured | [ ] Reactive grid |

### Interaction Verification
- [ ] Click model card → see check icon immediately
- [ ] Click model card → see badge in operation header immediately
- [ ] Change selection → previous card deselects, new card selects
- [ ] Configure new provider → models appear in grid automatically

---

## Risk Assessment

### Low Risk
- **Optimistic updates**: The UI updates before backend confirms. If backend fails, the UI shows the selection but it won't persist across app restarts. This matches prototype behavior.

### Medium Risk
- **Derived store performance**: `availableModelsByOperation` computes on every store change. With typical data sizes (6-10 providers, 20-50 models), this is negligible.

### Mitigation
- Consider adding error toasts if backend sync fails frequently
- Consider adding a "saving..." indicator for users on slow connections (optional enhancement)

---

## Files Changed Summary

| File | Changes |
|------|---------|
| `src-ui/src/lib/stores/configStore.ts` | Add derived store, update setOperationDefault |
| `src-ui/src/lib/components/config/SettingsModal.svelte` | Use reactive stores, add selected badge |
| `src-ui/src/lib/components/config/SettingsModal.module.css` | Add badge styles |
