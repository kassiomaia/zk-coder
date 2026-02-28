# Application State

Core state management architecture for the Coder application.

## State Structure

```typescript
interface AppState {
  // UI State
  ui: {
    sidebar: {
      collapsed: boolean;
      activeSection: string;
    };
    theme: 'light' | 'dark' | 'system';
    modals: {
      activeModal: string | null;
      modalData: Record<string, unknown>;
    };
  };

  // Chat State
  chat: {
    conversations: Conversation[];
    activeConversationId: string | null;
    messages: Message[];
    currentInput: string;
    isLoading: boolean;
    error: string | null;
  };

  // Repository State
  repository: {
    selectedRepo: string | null;
    branch: string;
    files: FileNode[];
    status: 'idle' | 'loading' | 'error' | 'ready';
    lastSync: Date | null;
  };

  // AI Configuration
  ai: {
    model: string;
    temperature: number;
    maxTokens: number;
    autoAcceptEdits: boolean;
    contextFiles: string[];
  };

  // User Preferences
  preferences: {
    fontSize: number;
    lineNumbers: boolean;
    wordWrap: boolean;
    autoSave: boolean;
    sidebarCollapsed: boolean;
  };
}

interface Conversation {
  id: string;
  title: string;
  createdAt: Date;
  updatedAt: Date;
  messageCount: number;
}

interface Message {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  timestamp: Date;
  status: 'sending' | 'sent' | 'error';
}

interface FileNode {
  name: string;
  path: string;
  type: 'file' | 'directory';
  children?: FileNode[];
}
```

---

## Store Implementation

### Persisted Store

```typescript
// $lib/stores/persisted.ts
import { browser } from '$app/environment';
import { writable, type Writable } from 'svelte/store';

export function persisted<T>(key: string, initialValue: T): Writable<T> {
  const stored = browser ? localStorage.getItem(key) : null;
  const value = stored ? JSON.parse(stored) : initialValue;

  const store = writable<T>(value);

  store.subscribe((val) => {
    if (browser) {
      localStorage.setItem(key, JSON.stringify(val));
    }
  });

  return store;
}
```

### UI Store

```typescript
// $lib/stores/ui.ts
import { writable, derived } from 'svelte/store';
import { persisted } from './persisted';

export type Theme = 'light' | 'dark' | 'system';

// Persisted preferences
export const themePreference = persisted<Theme>('theme', 'dark');
export const sidebarCollapsed = persisted<boolean>('sidebar-collapsed', false);

// Runtime state
export const activeSection = writable<string>('chat');
export const activeModal = writable<string | null>(null);
export const modalData = writable<Record<string, unknown>>({});

// Derived: effective theme
export const theme = derived(
  themePreference,
  ($pref) => {
    if ($pref === 'system' && typeof window !== 'undefined') {
      return window.matchMedia('(prefers-color-scheme: dark)').matches
        ? 'dark'
        : 'light';
    }
    return $pref === 'system' ? 'dark' : $pref;
  }
);

// Actions
export function toggleSidebar() {
  sidebarCollapsed.update((v) => !v);
}

export function setTheme(newTheme: Theme) {
  themePreference.set(newTheme);
}

export function openModal(name: string, data: Record<string, unknown> = {}) {
  activeModal.set(name);
  modalData.set(data);
}

export function closeModal() {
  activeModal.set(null);
  modalData.set({});
}
```

### Chat Store

```typescript
// $lib/stores/chat.ts
import { writable, derived } from 'svelte/store';
import type { Message, Conversation } from './types';

// State
export const conversations = writable<Conversation[]>([]);
export const activeConversationId = writable<string | null>(null);
export const messages = writable<Message[]>([]);
export const currentInput = writable<string>('');
export const isLoading = writable<boolean>(false);
export const error = writable<string | null>(null);

// Derived: active conversation
export const activeConversation = derived(
  [conversations, activeConversationId],
  ([$convs, $id]) => $convs.find((c) => c.id === $id) || null
);

// Actions
export function setInput(value: string) {
  currentInput.set(value);
}

export async function sendMessage(content: string) {
  if (!content.trim()) return;

  const message: Message = {
    id: crypto.randomUUID(),
    role: 'user',
    content,
    timestamp: new Date(),
    status: 'sending'
  };

  messages.update((msgs) => [...msgs, message]);
  currentInput.set('');
  isLoading.set(true);
  error.set(null);

  try {
    // Call API here
    const response = await fetch('/api/chat', {
      method: 'POST',
      body: JSON.stringify({ message: content })
    });

    if (!response.ok) throw new Error('Failed to send message');

    const data = await response.json();

    // Update message status
    messages.update((msgs) =>
      msgs.map((m) => (m.id === message.id ? { ...m, status: 'sent' } : m))
    );

    // Add assistant response
    const assistantMessage: Message = {
      id: crypto.randomUUID(),
      role: 'assistant',
      content: data.response,
      timestamp: new Date(),
      status: 'sent'
    };

    messages.update((msgs) => [...msgs, assistantMessage]);
  } catch (e) {
    error.set(e instanceof Error ? e.message : 'Unknown error');
    messages.update((msgs) =>
      msgs.map((m) =>
        m.id === message.id ? { ...m, status: 'error' } : m
      )
    );
  } finally {
    isLoading.set(false);
  }
}

export function clearMessages() {
  messages.set([]);
}
```

---

## Usage in Components

### Reading State

```svelte
<script lang="ts">
  import { theme, sidebarCollapsed } from '$lib/stores/ui';
  import { messages, isLoading } from '$lib/stores/chat';
</script>

<!-- Using $ prefix for auto-subscription -->
<div class="app" data-theme={$theme}">
  {#if !$sidebarCollapsed}
    <Sidebar />
  {/if}

  <main>
    {#each $messages as message}
      <ChatMessage {message} />
    {/each}

    {#if $isLoading}
      <TypingIndicator />
    {/if}
  </main>
</div>
```

### Updating State

```svelte
<script lang="ts">
  import { toggleSidebar, setTheme } from '$lib/stores/ui';
  import { sendMessage, setInput, currentInput } from '$lib/stores/chat';

  function handleSubmit() {
    sendMessage($currentInput);
  }
</script>

<button onclick={toggleSidebar}>Toggle Sidebar</button>
<button onclick={() => setTheme('dark')}>Dark Mode</button>

<input
  value={$currentInput}
  oninput={(e) => setInput(e.target.value)}
  onkeydown={(e) => e.key === 'Enter' && handleSubmit()}
/>
```

---

## Best Practices

1. **Separate concerns**: Keep UI, chat, and repository state separate
2. **Persist user preferences**: Use `persisted()` for user settings
3. **Derive when possible**: Use `derived()` for computed values
4. **Action creators**: Encapsulate state changes in functions
5. **Type everything**: Define interfaces for all state shapes
