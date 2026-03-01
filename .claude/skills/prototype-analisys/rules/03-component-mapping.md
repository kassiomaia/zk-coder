# Component Mapping Rules

These rules govern how prototype elements are mapped to Svelte components.

## Component Identification

### Component Boundaries
Identify component boundaries by looking for:
1. Elements with `x-data` attributes (Alpine.js components)
2. Logically grouped UI elements
3. Repeated patterns (lists, cards, items)
4. Interactive elements with their related state

### Scope Classification
For each identified component, classify as:
- **IN SCOPE:** Component implements feature functionality
- **CONTEXTUAL:** Component provides visual context only (do not implement)
- **UNCLEAR:** Needs clarification from user

## Existing Component Matching

### Search Process
1. Search `src-ui/src/lib/components/` for similar components
2. Check component names, props, and functionality
3. Review component CSS modules for styling patterns

### Match Categories

| Category | Criteria | Action |
|----------|----------|--------|
| **Exact Match** | Same UI pattern, same props | Use as-is |
| **Extend** | Similar pattern, needs additions | Plan modifications |
| **Partial Match** | Some shared functionality | Extract shared logic |
| **No Match** | No similar component | Create new |

## Alpine.js to Svelte 5 Mapping

### State Mapping

| Alpine.js | Svelte 5 | Notes |
|-----------|----------|-------|
| `x-data="{ prop: val }"` | `let prop = $state(val)` | Use runes for reactivity |
| `get computed()` | `$derived()` | Computed values |
| `method() {}` | `function method() {}` | Standard functions |

### Directive Mapping

| Alpine.js | Svelte 5 | Example |
|-----------|----------|---------|
| `x-show="cond"` | `{#if cond}` | Conditional display |
| `x-if="cond"` | `{#if cond}...{/if}` | Conditional render |
| `x-for="item in items"` | `{#each items as item}` | Loop rendering |
| `x-model="value"` | `bind:value` | Two-way binding |
| `@click="fn()"` | `onclick={fn}` or `on:click={fn}` | Event handling |
| `@keydown.enter="fn()"` | `onkeydown={fn}` with modifier | Key events |
| `x-bind:class="{active: on}"` | `class:active={on}` | Class binding |
| `x-bind:disabled="cond"` | `disabled={cond}` | Attribute binding |
| `x-transition` | Svelte `transition:` directive | Animations |
| `x-cloak` | Not needed | Svelte handles this |

### Event Modifiers

| Alpine.js | Svelte 5 |
|-----------|----------|
| `@click.away` | Custom click-outside action |
| `@keydown.escape` | `onkeydown` with key check |
| `@click.prevent` | `on:click|preventDefault` |

## CSS Module Conversion

### Class Naming
- Convert kebab-case to camelCase for module imports
- Example: `.provider-card` → `styles.providerCard`

### Module Structure
```
ComponentName.svelte
ComponentName.module.css
```

### Import Pattern
```svelte
<script>
  import styles from './ComponentName.module.css';
</script>

<div class={styles.container}>
  ...
</div>
```

## Component Sizing

### File Size Limits
Following project guidelines:
- Soft limit: 500 lines of code
- Hard limit: 800 lines of code

### When to Split
Split a component if:
1. It exceeds 500 lines
2. It contains multiple distinct UI concerns
3. Part of the component is reusable elsewhere
4. State management becomes complex

## Reusable Component Identification

Mark a component as reusable if:
1. It appears in multiple places in the prototype
2. It represents a common UI pattern (button, input, card)
3. It has generic functionality applicable to other features
4. User explicitly requests it

Place reusable components in:
```
src-ui/src/lib/components/shared/
```
