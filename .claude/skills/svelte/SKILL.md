---
name: svelte
description: Svelte 5 and SvelteKit frontend development patterns. Use when creating components, managing state with runes, handling routing, or styling with CSS modules.
---

# Svelte 5 Development Skill

Guide for developing the Coder frontend using Svelte 5, SvelteKit, and TypeScript.

## Core Concepts

### Runes (Svelte 5 Reactivity)

Svelte 5 introduces runes for reactivity:

```svelte
<script lang="ts">
  // State - mutable reactive values
  let count = $state(0);
  let projects = $state<Project[]>([]);

  // Derived - computed values
  let filteredProjects = $derived(
    projects.filter(p => p.name.includes(search))
  );

  // Effect - side effects
  $effect(() => {
    console.log('Count changed:', count);
    // Cleanup function (optional)
    return () => console.log('Cleanup');
  });

  // $derived.by for complex computations
  let stats = $derived.by(() => {
    const total = projects.length;
    const active = projects.filter(p => p.active).length;
    return { total, active };
  });
</script>
```

### Props

Use `$props()` for component inputs:

```svelte
<script lang="ts">
  interface Props {
    title: string;
    projects?: Project[];
    onSelect?: (project: Project) => void;
  }

  let { title, projects = [], onSelect }: Props = $props();
</script>
```

### Bindings

Two-way binding with `$bindable`:

```svelte
<!-- ChildComponent.svelte -->
<script lang="ts">
  interface Props {
    value?: string;
  }

  let { value = $bindable('') }: Props = $props();
</script>

<input bind:value />

<!-- Parent.svelte -->
<script lang="ts">
  let searchText = $state('');
</script>

<ChildComponent bind:value={searchText} />
```

## Component Structure

Follow the project conventions:

```
src-ui/src/lib/components/
├── ChatMessage.svelte
├── ChatMessage.module.css
├── ProjectList.svelte
├── ProjectList.module.css
└── Button/
    ├── Button.svelte
    └── Button.module.css
```

### Component Template

```svelte
<script lang="ts">
  // 1. Imports
  import type { Project } from '$lib/types';
  import styles from './ProjectCard.module.css';

  // 2. Props interface
  interface Props {
    project: Project;
    selected?: boolean;
    onSelect?: (project: Project) => void;
  }

  // 3. Props destructuring
  let { project, selected = false, onSelect }: Props = $props();

  // 4. Local state
  let isHovered = $state(false);

  // 5. Derived values
  let cardClasses = $derived(
    [styles.card, selected && styles.selected, isHovered && styles.hovered]
      .filter(Boolean)
      .join(' ')
  );

  // 6. Effects
  $effect(() => {
    if (selected) {
      console.log('Project selected:', project.id);
    }
  });

  // 7. Event handlers
  function handleClick() {
    onSelect?.(project);
  }
</script>

<!-- 8. Template -->
<article
  class={cardClasses}
  onmouseenter={() => isHovered = true}
  onmouseleave={() => isHovered = false}
  onclick={handleClick}
  role="button"
  tabindex="0"
>
  <h3 class={styles.title}>{project.name}</h3>
  <p class={styles.path}>{project.path}</p>
</article>
```

## SvelteKit Routing

### File-based Routing

```
src-ui/src/routes/
├── +layout.svelte      # Shared layout
├── +layout.ts          # Layout data/load
├── +page.svelte        # Home page (/)
├── +page.ts            # Home page data/load
├── projects/
│   ├── +page.svelte    # /projects
│   └── [id]/
│       └── +page.svelte  # /projects/:id
└── settings/
    └── +page.svelte    # /settings
```

### Loading Data

```typescript
// +page.ts
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch, params }) => {
  const response = await fetch(`/api/projects/${params.id}`);
  const project = await response.json();

  return { project };
};
```

```svelte
<!-- +page.svelte -->
<script lang="ts">
  import type { PageData } from './$types';

  let { data }: { data: PageData } = $props();
</script>

<h1>{data.project.name}</h1>
```

### Layouts

```svelte
<!-- +layout.svelte -->
<script lang="ts">
  import Sidebar from '$lib/components/Sidebar.svelte';
  import Header from '$lib/components/Header.svelte';

  let { children } = $props();
</script>

<div class="app-layout">
  <Sidebar />
  <main>
    <Header />
    {@render children()}
  </main>
</div>
```

## CSS Modules

### Scoped Styling

Each component has a matching CSS module:

```css
/* Button.module.css */
.button {
  padding: var(--spacing-sm) var(--spacing-md);
  border-radius: var(--radius-sm);
  font-weight: 500;
  transition: background-color 0.2s;
}

.primary {
  background-color: var(--color-primary);
  color: var(--color-on-primary);
}

.secondary {
  background-color: var(--color-secondary);
  color: var(--color-on-secondary);
}

.disabled {
  opacity: 0.5;
  pointer-events: none;
}
```

```svelte
<!-- Button.svelte -->
<script lang="ts">
  import styles from './Button.module.css';

  interface Props {
    variant?: 'primary' | 'secondary';
    disabled?: boolean;
    onclick?: () => void;
  }

  let { variant = 'primary', disabled = false, onclick, children }: Props = $props();

  let buttonClasses = $derived(
    [styles.button, styles[variant], disabled && styles.disabled]
      .filter(Boolean)
      .join(' ')
  );
</script>

<button class={buttonClasses} {disabled} {onclick}>
  {@render children()}
</button>
```

### CSS Variables (Design System)

Use tokens from `design-system.css`:

```css
/* design-system.css */
:root {
  /* Colors */
  --color-primary: #6366f1;
  --color-on-primary: #ffffff;
  --color-background: #0f172a;
  --color-surface: #1e293b;

  /* Spacing */
  --spacing-xs: 0.25rem;
  --spacing-sm: 0.5rem;
  --spacing-md: 1rem;
  --spacing-lg: 1.5rem;
  --spacing-xl: 2rem;

  /* Typography */
  --font-family: 'Inter', system-ui, sans-serif;
  --font-mono: 'JetBrains Mono', monospace;
  --font-size-sm: 0.875rem;
  --font-size-md: 1rem;
  --font-size-lg: 1.125rem;

  /* Border radius */
  --radius-sm: 0.25rem;
  --radius-md: 0.5rem;
  --radius-lg: 0.75rem;
}
```

## Event Handling

### DOM Events

```svelte
<script lang="ts">
  let inputValue = $state('');

  function handleInput(e: Event) {
    const target = e.target as HTMLInputElement;
    inputValue = target.value;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      submit();
    }
  }
</script>

<input
  value={inputValue}
  oninput={handleInput}
  onkeydown={handleKeydown}
/>
```

### Custom Events

```svelte
<!-- ProjectCard.svelte -->
<script lang="ts">
  import type { Project } from '$lib/types';

  interface Props {
    project: Project;
  }

  let { project }: Props = $props();

  function dispatchSelect() {
    // Custom events in Svelte 5 use callbacks via props
    // See "Props with callbacks" above
  }
</script>
```

## Stores (SvelteKit)

Use SvelteKit's built-in stores for client-side state:

```svelte
<script lang="ts">
  import { persisted } from '$lib/stores/persisted';

  // Persisted state (localStorage)
  const settings = persisted('settings', {
    theme: 'dark',
    fontSize: 14,
  });

  // Update
  settings.update(s => ({ ...s, theme: 'light' }));
</script>
```

## Snippets and Slots

### Snippets (Svelte 5)

```svelte
<script lang="ts">
  interface Props {
    items: Project[];
    renderItem?: (item: Project) => typeof renderDefault;
  }

  let { items, renderItem }: Props = $props();

  function renderDefault(project: Project) {
    return <span>{project.name}</span>;
  }

  const render = renderItem ?? renderDefault;
</script>

<ul>
  {#each items as item}
    <li>{@render render(item)}</li>
  {/each}
</ul>
```

## TypeScript Integration

### Type Definitions

```typescript
// $lib/types.ts
export interface Project {
  id: string;
  name: string;
  path: string;
  createdAt: string;
  updatedAt: string;
}

export interface UserSettings {
  theme: 'light' | 'dark' | 'system';
  autoSave: boolean;
  editorFont: string;
  fontSize: number;
}
```

### Component Types

```typescript
// Component prop types
import type { ComponentType, Snippet } from 'svelte';

interface ListProps<T> {
  items: T[];
  renderItem?: Snippet<[T]>;
  onSelect?: (item: T) => void;
}
```

## Best Practices

1. **Keep components small**: Single responsibility, under 200 lines
2. **Use CSS modules**: Match component name for easy discovery
3. **Type everything**: Strict TypeScript for all components
4. **Prefer runes**: Use `$state`, `$derived`, `$effect` over legacy patterns
5. **Design system first**: Check `design-system.css` before adding new styles
6. **Accessibility**: Include ARIA attributes and keyboard navigation

## Additional Resources

- For component patterns, see [patterns.md](patterns.md)
- For styling guidelines, see [styling.md](styling.md)
- For Svelte 5 docs: https://svelte-5-preview.librejs.org/
