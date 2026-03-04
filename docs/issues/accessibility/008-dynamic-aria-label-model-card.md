# [ISSUE-008] - Dynamic ARIA Label for Model Cards

## Priority
High

## Source
- **Review File**: `docs/reviews/code-reviewer/2026-03-04-ModelCard-review.md`
- **Component/File**: `src-ui/src/lib/components/ModelCard.svelte`
- **Line Numbers**: TBD

## Description
The ModelCard component displays dynamic model information but doesn't provide sufficient ARIA labels for screen readers. The cards may not properly communicate the model's status and capabilities to assistive technology users.

## Impact
Accessibility - This fails WCAG guidelines by:
- Not providing sufficient context for dynamic content
- Making it difficult for screen reader users to understand model details
- Reducing the usability of model selection for visually impaired users

## Suggested Fix
Add dynamic ARIA labels that describe the model card:

```typescript
<div
  class="model-card"
  role="article"
  aria-label={`Model: ${model.name}, ${model.capabilities}, Status: ${model.status}`}
>
  <h3>{model.name}</h3>
  <p aria-label="Model description">{model.description}</p>
  <div class="model-status" aria-label={`Model status: ${model.status}`}>
    <StatusBadge variant={model.status} />
  </div>
  <div class="model-capabilities" aria-label="Model capabilities">
    {#each model.capabilities as cap}
      <span class="capability-tag">{cap}</span>
    {/each}
  </div>
</div>
```

## Related Issues
- None

## Resolution Status
**RESOLVED** - Mostly False Positive - 2026-03-04

The ModelCard component already has proper accessibility attributes:
- `aria-pressed={selected}` for toggle state
- `aria-label="Select {model.name}"` for screen readers
- Capability icons have `title` attributes

The capability icons could be improved with additional `aria-label` attributes for better screen reader support, but the current implementation meets WCAG 2.1 basic requirements.