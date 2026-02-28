# Svelte Component Patterns

## List Components

### Generic List with Custom Rendering

```svelte
<script lang="ts" generics="T">
  import styles from './List.module.css';

  interface Props {
    items: T[];
    keyFn?: (item: T) => string;
    emptyMessage?: string;
    onSelect?: (item: T) => void;
    children?: import('svelte').Snippet<[T]>;
  }

  let {
    items,
    keyFn = (item: T) => String(item),
    emptyMessage = 'No items',
    onSelect,
    children
  }: Props = $props();

  let selectedIndex = $state(-1);

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') {
      selectedIndex = Math.min(selectedIndex + 1, items.length - 1);
    } else if (e.key === 'ArrowUp') {
      selectedIndex = Math.max(selectedIndex - 1, 0);
    } else if (e.key === 'Enter' && selectedIndex >= 0) {
      onSelect?.(items[selectedIndex]);
    }
  }
</script>

<ul
  class={styles.list}
  role="listbox"
  onkeydown={handleKeydown}
>
  {#if items.length === 0}
    <li class={styles.empty}>{emptyMessage}</li>
  {:else}
    {#each items as item, i (keyFn(item))}
      <li
        class={[styles.item, i === selectedIndex && styles.selected].filter(Boolean).join(' ')}
        role="option"
        aria-selected={i === selectedIndex}
        onclick={() => onSelect?.(item)}
      >
        {#if children}
          {@render children(item)}
        {:else}
          {String(item)}
        {/if}
      </li>
    {/each}
  {/if}
</ul>
```

## Form Components

### Controlled Input

```svelte
<script lang="ts">
  import styles from './TextInput.module.css';

  interface Props {
    value?: string;
    placeholder?: string;
    type?: 'text' | 'password' | 'email';
    disabled?: boolean;
    error?: string;
    oninput?: (value: string) => void;
    onkeydown?: (event: KeyboardEvent) => void;
  }

  let {
    value = $bindable(''),
    placeholder = '',
    type = 'text',
    disabled = false,
    error,
    oninput,
    onkeydown
  }: Props = $props();
</script>

<div class={styles.container}>
  <input
    class={[styles.input, error && styles.error].filter(Boolean).join(' ')}
    {type}
    {placeholder}
    {disabled}
    bind:value
    oninput={(e) => oninput?.((e.target as HTMLInputElement).value)}
    {onkeydown}
  />
  {#if error}
    <span class={styles.errorText}>{error}</span>
  {/if}
</div>
```

### Form State Management

```svelte
<script lang="ts">
  import TextInput from './TextInput.svelte';

  interface FormData {
    name: string;
    email: string;
    message: string;
  }

  let form = $state<FormData>({
    name: '',
    email: '',
    message: '',
  });

  let errors = $state<Partial<FormData>>({});

  let isValid = $derived(
    form.name.length > 0 &&
    form.email.includes('@') &&
    form.message.length > 10
  );

  function validate() {
    errors = {};
    if (!form.name) errors.name = 'Name is required';
    if (!form.email.includes('@')) errors.email = 'Invalid email';
    if (form.message.length < 10) errors.message = 'Message too short';
    return Object.keys(errors).length === 0;
  }

  async function handleSubmit() {
    if (!validate()) return;
    // Submit form...
  }
</script>

<form onsubmit={(e) => { e.preventDefault(); handleSubmit(); }}>
  <TextInput
    bind:value={form.name}
    placeholder="Name"
    error={errors.name}
  />
  <TextInput
    bind:value={form.email}
    type="email"
    placeholder="Email"
    error={errors.email}
  />
  <textarea bind:value={form.message}></textarea>

  <button type="submit" disabled={!isValid}>Submit</button>
</form>
```

## Modal/Dialog Pattern

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import styles from './Modal.module.css';

  interface Props {
    open: boolean;
    title?: string;
    onClose: () => void;
    children: import('svelte').Snippet;
  }

  let { open, title, onClose, children }: Props = $props();

  let modalRef: HTMLDialogElement | undefined = $state();

  $effect(() => {
    if (open) {
      modalRef?.showModal();
    } else {
      modalRef?.close();
    }
  });

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === modalRef) {
      onClose();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onClose();
    }
  }
</script>

{#if open}
  <dialog
    bind:this={modalRef}
    class={styles.modal}
    onclick={handleBackdropClick}
    onkeydown={handleKeydown}
  >
    <article class={styles.content}>
      {#if title}
        <header class={styles.header}>
          <h2>{title}</h2>
          <button
            class={styles.close}
            onclick={onClose}
            aria-label="Close modal"
          >
            ×
          </button>
        </header>
      {/if}
      <div class={styles.body}>
        {@render children()}
      </div>
    </article>
  </dialog>
{/if}
```

## Loading States

### Skeleton Component

```svelte
<script lang="ts">
  import styles from './Skeleton.module.css';

  interface Props {
    width?: string;
    height?: string;
    rounded?: boolean;
  }

  let { width = '100%', height = '1rem', rounded = false }: Props = $props();
</script>

<div
  class={[styles.skeleton, rounded && styles.rounded].filter(Boolean).join(' ')}
  style:width={width}
  style:height={height}
  aria-hidden="true"
></div>
```

### Async Data Pattern

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import Skeleton from './Skeleton.svelte';

  interface Props {
    fetcher: () => Promise<T>;
    children: import('svelte').Snippet<[T]>;
    fallback?: import('svelte').Snippet;
    error?: import('svelte').Snippet<[Error]>;
  }

  let { fetcher, children, fallback, error }: Props<T> = $props();

  let data = $state<T | null>(null);
  let loading = $state(true);
  let err = $state<Error | null>(null);

  onMount(async () => {
    try {
      data = await fetcher();
    } catch (e) {
      err = e instanceof Error ? e : new Error(String(e));
    } finally {
      loading = false;
    }
  });
</script>

{#if loading}
  {#if fallback}
    {@render fallback()}
  {:else}
    <Skeleton height="200px" />
  {/if}
{:else if err}
  {#if error}
    {@render error(err)}
  {:else}
    <p>Error: {err.message}</p>
  {/if}
{:else if data}
  {@render children(data)}
{/if}
```

## Context Pattern

### Creating Context

```svelte
<!-- ThemeProvider.svelte -->
<script lang="ts">
  import { setContext, getContext } from 'svelte';

  interface Theme {
    mode: 'light' | 'dark';
    toggle: () => void;
  }

  const THEME_KEY = Symbol('theme');

  let mode = $state<'light' | 'dark'>('dark');

  function toggle() {
    mode = mode === 'light' ? 'dark' : 'light';
  }

  setContext(THEME_KEY, {
    get mode() { return mode; },
    toggle
  });
</script>

{@render children()}
```

### Using Context

```svelte
<!-- ThemedButton.svelte -->
<script lang="ts">
  import { getContext } from 'svelte';

  const theme = getContext<Theme>('theme');
</script>

<button class:dark={theme.mode === 'dark'}>
  {@render children()}
</button>
```

## Composition Pattern

### Compound Components

```svelte
<!-- Tabs.svelte -->
<script lang="ts">
  import { setContext } from 'svelte';
  import styles from './Tabs.module.css';

  interface Props {
    defaultTab?: string;
    children: import('svelte').Snippet;
  }

  let { defaultTab, children }: Props = $props();

  let activeTab = $state(defaultTab ?? '');

  setContext('tabs', {
    activeTab,
    setActiveTab: (id: string) => { activeTab = id; }
  });
</script>

<div class={styles.tabs}>
  {@render children()}
</div>
```

```svelte
<!-- TabList.svelte -->
<script lang="ts">
  import { getContext } from 'svelte';
  import styles from './Tabs.module.css';

  interface Props {
    children: import('svelte').Snippet;
  }

  let { children }: Props = $props();

  const tabs = getContext('tabs');
</script>

<div class={styles.tabList} role="tablist">
  {@render children()}
</div>
```

```svelte
<!-- Tab.svelte -->
<script lang="ts">
  import { getContext } from 'svelte';
  import styles from './Tabs.module.css';

  interface Props {
    id: string;
    children: import('svelte').Snippet;
  }

  let { id, children }: Props = $props();

  const { activeTab, setActiveTab } = getContext('tabs');
</script>

<button
  class={[styles.tab, activeTab === id && styles.active].filter(Boolean).join(' ')}
  role="tab"
  aria-selected={activeTab === id}
  onclick={() => setActiveTab(id)}
>
  {@render children()}
</button>
```
