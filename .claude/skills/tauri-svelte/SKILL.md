---
name: tauri-svelte
description: Integration patterns between Tauri Rust backend and Svelte frontend. Use when connecting frontend to backend, handling IPC, managing state across the bridge, or setting up event listeners.
---

# Tauri-Svelte Integration Skill

Guide for connecting the Tauri Rust backend with the Svelte frontend in the Coder application.

## Core Integration

### Frontend API Setup

Install Tauri API package:
```bash
cd src-ui
npm install @tauri-apps/api
```

### Invoking Commands

Basic command invocation from Svelte:

```svelte
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import type { Project } from '$lib/types';

  let projects = $state<Project[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  async function loadProjects() {
    try {
      loading = true;
      error = null;
      projects = await invoke<Project[]>('fetch_projects');
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  onMount(loadProjects);
</script>
```

### Type-Safe Commands

Create typed command wrappers:

```typescript
// $lib/api/commands.ts
import { invoke } from '@tauri-apps/api/core';
import type { Project, Settings } from '$lib/types';

export async function fetchProjects(): Promise<Project[]> {
  return invoke('fetch_projects');
}

export async function createProject(name: string, path: string): Promise<Project> {
  return invoke('create_project', { name, path });
}

export async function deleteProject(id: string): Promise<void> {
  return invoke('delete_project', { id });
}

export async function getSettings(): Promise<Settings> {
  return invoke('get_settings');
}

export async function updateSettings(settings: Settings): Promise<Settings> {
  return invoke('update_settings', { settings });
}
```

## Event System

### Listening to Events

Frontend event listeners:

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';

  interface FileProcessedPayload {
    path: string;
    success: boolean;
  }

  let unlisten: UnlistenFn | null = null;

  onMount(async () => {
    unlisten = await listen<FileProcessedPayload>('file-processed', (event) => {
      console.log('File processed:', event.payload);
      // Update UI based on event
    });
  });

  // Cleanup on unmount
  $effect(() => {
    return () => {
      unlisten?.();
    };
  });
</script>
```

### Event Composable Pattern

```typescript
// $lib/api/events.ts
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { onMount } from 'svelte';

export function useEvent<T>(eventName: string, handler: (payload: T) => void) {
  let unlisten: UnlistenFn | null = null;

  onMount(async () => {
    unlisten = await listen<T>(eventName, (event) => {
      handler(event.payload);
    });
  });

  return {
    destroy: () => unlisten?.()
  };
}
```

```svelte
<script lang="ts">
  import { useEvent } from '$lib/api/events';

  let notifications = $state<Notification[]>([]);

  useEvent<Notification>('notification', (payload) => {
    notifications = [...notifications, payload];
  });
</script>
```

## State Synchronization

### Bidirectional State Pattern

Keep Rust and Svelte state in sync:

```svelte
<!-- ProjectStore.svelte -->
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import type { Project } from '$lib/types';

  // Local reactive state
  let projects = $state<Project[]>([]);
  let loading = $state(false);

  // Load initial state from backend
  async function load() {
    loading = true;
    try {
      projects = await invoke<Project[]>('fetch_projects');
    } finally {
      loading = false;
    }
  }

  // CRUD operations that sync both sides
  async function create(name: string, path: string) {
    const project = await invoke<Project>('create_project', { name, path });
    projects = [...projects, project];
    return project;
  }

  async function remove(id: string) {
    await invoke('delete_project', { id });
    projects = projects.filter(p => p.id !== id);
  }

  // Listen for external changes
  onMount(async () => {
    await load();

    const unlisten = await listen<Project[]>('projects-updated', (event) => {
      projects = event.payload;
    });

    return unlisten;
  });
</script>

<!-- Export as context or store -->
```

### Context-Based State

```svelte
<!-- ProjectContext.svelte -->
<script lang="ts">
  import { setContext } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import type { Project } from '$lib/types';

  let projects = $state<Project[]>([]);
  let selected = $state<Project | null>(null);

  async function load() {
    projects = await invoke<Project[]>('fetch_projects');
  }

  async function create(name: string, path: string) {
    const project = await invoke<Project>('create_project', { name, path });
    projects = [...projects, project];
    return project;
  }

  function select(project: Project | null) {
    selected = project;
  }

  // Expose reactive state and methods
  setContext('projects', {
    get projects() { return projects; },
    get selected() { return selected; },
    load,
    create,
    select
  });
</script>

{@render children()}
```

## Error Handling

### Unified Error Type

Define matching error types in both Rust and TypeScript:

```typescript
// $lib/types/errors.ts
export interface CommandError {
  code: string;
  message: string;
  details?: Record<string, unknown>;
}

export function isCommandError(error: unknown): error is CommandError {
  return (
    typeof error === 'object' &&
    error !== null &&
    'code' in error &&
    'message' in error
  );
}
```

### Error Boundary Pattern

```svelte
<!-- ErrorBoundary.svelte -->
<script lang="ts">
  import { isCommandError } from '$lib/types/errors';

  interface Props {
    fallback?: import('svelte').Snippet<[Error]>;
    children: import('svelte').Snippet;
  }

  let { fallback, children }: Props = $props();

  let error = $state<Error | null>(null);

  function handleError(e: Error) {
    error = e;
    console.error('Command error:', e);
  }

  function reset() {
    error = null;
  }
</script>

{#if error}
  {#if fallback}
    {@render fallback(error)}
  {:else}
    <div class="error">
      <p>Something went wrong: {error.message}</p>
      <button onclick={reset}>Try again</button>
    </div>
  {/if}
{:else}
  {@render children({ handleError })}
{/if}
```

### Safe Invoke Wrapper

```typescript
// $lib/api/safe-invoke.ts
import { invoke } from '@tauri-apps/api/core';
import { isCommandError, type CommandError } from '$lib/types/errors';

export async function safeInvoke<T>(
  cmd: string,
  args?: Record<string, unknown>
): Promise<{ data: T; error: null } | { data: null; error: CommandError }> {
  try {
    const data = await invoke<T>(cmd, args);
    return { data, error: null };
  } catch (e) {
    if (isCommandError(e)) {
      return { data: null, error: e };
    }
    return {
      data: null,
      error: {
        code: 'UNKNOWN',
        message: e instanceof Error ? e.message : String(e),
      },
    };
  }
}
```

## Loading States

### Async Operation Pattern

```svelte
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

  let loading = $state(false);
  let error = $state<string | null>(null);
  let projects = $state<Project[]>([]);

  async function loadProjects() {
    loading = true;
    error = null;

    try {
      projects = await invoke<Project[]>('fetch_projects');
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load projects';
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    loadProjects();
  });
</script>

{#if loading}
  <div class="loading">Loading...</div>
{:else if error}
  <div class="error">{error}</div>
{:else}
  <ul>
    {#each projects as project}
      <li>{project.name}</li>
    {/each}
  </ul>
{/if}
```

## File Operations

### Read/Write Pattern

```svelte
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

  let content = $state('');
  let filePath = $state('');

  async function openFile() {
    // Use Tauri dialog to pick file
    const path = await invoke<string>('open_file_dialog');
    if (path) {
      filePath = path;
      content = await invoke<string>('read_file', { path });
    }
  }

  async function saveFile() {
    if (filePath) {
      await invoke('write_file', { path: filePath, content });
    }
  }
</script>

<button onclick={openFile}>Open</button>
<button onclick={saveFile} disabled={!filePath}>Save</button>

<textarea bind:value={content}></textarea>
```

## Development vs Production

### Environment Detection

```typescript
// $lib/api/env.ts
import { isTauri } from '@tauri-apps/api/core';

export async function getEnvironment() {
  const isDesktop = await isTauri();
  return {
    isDesktop,
    isWeb: !isDesktop,
  };
}
```

### Mock Commands for Web

```typescript
// $lib/api/mock.ts
import type { Project } from '$lib/types';

// Mock implementations for web development
export const mockCommands = {
  fetch_projects: async (): Promise<Project[]> => {
    return [
      { id: '1', name: 'Project 1', path: '/path/1' },
      { id: '2', name: 'Project 2', path: '/path/2' },
    ];
  },
  create_project: async (name: string, path: string): Promise<Project> => {
    return { id: Date.now().toString(), name, path };
  },
};

// Conditional command wrapper
export async function command<T>(name: string, args?: Record<string, unknown>): Promise<T> {
  const { isTauri } = await import('@tauri-apps/api/core');

  if (await isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<T>(name, args);
  }

  // Use mock in web mode
  const mock = mockCommands[name as keyof typeof mockCommands];
  if (mock) {
    return mock(args as never) as Promise<T>;
  }

  throw new Error(`Unknown command: ${name}`);
}
```

## Best Practices

1. **Type everything**: Share type definitions between Rust and TypeScript
2. **Handle errors gracefully**: Never let backend errors crash the UI
3. **Use loading states**: Show feedback during async operations
4. **Clean up listeners**: Always unsubscribe from events on unmount
5. **Mock for development**: Enable web-mode development with mock commands
6. **Batch operations**: Minimize command invocations for better performance

## Additional Resources

- For command examples, see [examples/](examples/)
- For Tauri API docs: https://v2.tauri.app/reference/javascript/api/
- For Svelte docs: https://svelte-5-preview.librejs.org/
