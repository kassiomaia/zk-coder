# User Preferences

Persisted user preferences and settings.

## Preference Structure

```typescript
interface UserPreferences {
  // Appearance
  theme: 'light' | 'dark' | 'system';
  fontSize: number;
  fontFamily: 'default' | 'compact';
  reduceMotion: boolean;

  // Layout
  sidebarCollapsed: boolean;
  sidebarWidth: number;

  // Editor
  lineNumbers: boolean;
  wordWrap: boolean;
  tabSize: 2 | 4;
  autoSave: boolean;
  autoSaveDelay: number;

  // AI
  defaultModel: string;
  temperature: number;
  autoAcceptEdits: boolean;
  showTokenCount: boolean;

  // Notifications
  notificationsEnabled: boolean;
  soundEnabled: boolean;
}
```

---

## Default Values

```typescript
const defaultPreferences: UserPreferences = {
  // Appearance
  theme: 'dark',
  fontSize: 14,
  fontFamily: 'default',
  reduceMotion: false,

  // Layout
  sidebarCollapsed: false,
  sidebarWidth: 280,

  // Editor
  lineNumbers: true,
  wordWrap: true,
  tabSize: 2,
  autoSave: true,
  autoSaveDelay: 1000,

  // AI
  defaultModel: 'claude-3-opus',
  temperature: 0.7,
  autoAcceptEdits: false,
  showTokenCount: true,

  // Notifications
  notificationsEnabled: true,
  soundEnabled: false
};
```

---

## Store Implementation

```typescript
// $lib/stores/preferences.ts
import { browser } from '$app/environment';
import { writable, derived } from 'svelte/store';
import type { UserPreferences } from './types';

const STORAGE_KEY = 'coder-preferences';

function createPreferencesStore() {
  // Load from localStorage
  const stored = browser
    ? localStorage.getItem(STORAGE_KEY)
    : null;

  const initial: UserPreferences = stored
    ? { ...defaultPreferences, ...JSON.parse(stored) }
    : defaultPreferences;

  const { subscribe, set, update } = writable<UserPreferences>(initial);

  // Persist changes
  function persist(value: UserPreferences) {
    if (browser) {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(value));
    }
  }

  return {
    subscribe,

    set(value: UserPreferences) {
      set(value);
      persist(value);
    },

    update(fn: (value: UserPreferences) => UserPreferences) {
      update((value) => {
        const newValue = fn(value);
        persist(newValue);
        return newValue;
      });
    },

    reset() {
      set(defaultPreferences);
      persist(defaultPreferences);
    },

    setPreference<K extends keyof UserPreferences>(
      key: K,
      value: UserPreferences[K]
    ) {
      update((prefs) => ({ ...prefs, [key]: value }));
    }
  };
}

export const preferences = createPreferencesStore();
```

---

## Derived Stores

```typescript
// Effective theme (resolves 'system')
export const effectiveTheme = derived(
  preferences,
  ($prefs) => {
    if ($prefs.theme !== 'system') {
      return $prefs.theme;
    }

    if (typeof window === 'undefined') {
      return 'dark';
    }

    return window.matchMedia('(prefers-color-scheme: dark)').matches
      ? 'dark'
      : 'light';
  }
);

// Font size in pixels
export const fontSizePixels = derived(
  preferences,
  ($prefs) => $prefs.fontSize
);

// CSS variables for preferences
export const preferenceStyles = derived(
  preferences,
  ($prefs) => ({
    '--user-font-size': `${$prefs.fontSize}px`,
    '--user-tab-size': $prefs.tabSize
  })
);
```

---

## Usage in Components

```svelte
<script lang="ts">
  import { preferences } from '$lib/stores/preferences';

  // Read preference
  $: theme = $preferences.theme;

  // Update preference
  function setTheme(newTheme: 'light' | 'dark' | 'system') {
    preferences.setPreference('theme', newTheme);
  }

  // Reset all preferences
  function resetPreferences() {
    preferences.reset();
  }
</script>

<select value={$preferences.theme} onchange={(e) => setTheme(e.target.value)}>
  <option value="light">Light</option>
  <option value="dark">Dark</option>
  <option value="system">System</option>
</select>
```

---

## Apply Preferences to Document

```typescript
// $lib/utils/applyPreferences.ts
import { preferences, effectiveTheme } from '$lib/stores/preferences';

export function applyPreferences() {
  // Apply theme
  effectiveTheme.subscribe((theme) => {
    document.documentElement.setAttribute('data-theme', theme);
  });

  // Apply font size
  preferences.subscribe(($prefs) => {
    document.documentElement.style.setProperty(
      '--user-font-size',
      `${$prefs.fontSize}px`
    );

    // Apply motion preference
    if ($prefs.reduceMotion) {
      document.documentElement.style.setProperty(
        '--transition-fast', '0ms'
      );
      document.documentElement.style.setProperty(
        '--transition-normal', '0ms'
      );
      document.documentElement.style.setProperty(
        '--transition-slow', '0ms'
      );
    }
  });
}
```

---

## Settings Component

```svelte
<!-- PreferencesPanel.svelte -->
<script lang="ts">
  import { preferences } from '$lib/stores/preferences';

  const themeOptions = [
    { value: 'light', label: 'Light' },
    { value: 'dark', label: 'Dark' },
    { value: 'system', label: 'System' }
  ] as const;
</script>

<div class="preferences-panel">
  <h2>Appearance</h2>

  <label>
    <span>Theme</span>
    <select
      value={$preferences.theme}
      onchange={(e) => preferences.setPreference('theme', e.target.value)}
    >
      {#each themeOptions as option}
        <option value={option.value}>{option.label}</option>
      {/each}
    </select>
  </label>

  <label>
    <span>Font Size</span>
    <input
      type="number"
      min="12"
      max="24"
      value={$preferences.fontSize}
      onchange={(e) => preferences.setPreference('fontSize', Number(e.target.value))}
    />
  </label>

  <label>
    <span>Reduce Motion</span>
    <input
      type="checkbox"
      checked={$preferences.reduceMotion}
      onchange={(e) => preferences.setPreference('reduceMotion', e.target.checked)}
    />
  </label>

  <h2>Editor</h2>

  <label>
    <span>Line Numbers</span>
    <input
      type="checkbox"
      checked={$preferences.lineNumbers}
      onchange={(e) => preferences.setPreference('lineNumbers', e.target.checked)}
    />
  </label>

  <label>
    <span>Word Wrap</span>
    <input
      type="checkbox"
      checked={$preferences.wordWrap}
      onchange={(e) => preferences.setPreference('wordWrap', e.target.checked)}
    />
  </label>

  <button onclick={() => preferences.reset()}>
    Reset to Defaults
  </button>
</div>
```
