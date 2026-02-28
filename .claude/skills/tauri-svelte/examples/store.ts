// Tauri-Svelte Integration Examples

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { onMount } from 'svelte';
import type { Project, Settings } from '$lib/types';

// ============ Command Wrappers ============

export const api = {
  // Project operations
  projects: {
    async list(): Promise<Project[]> {
      return invoke('fetch_projects');
    },

    async create(name: string, path: string): Promise<Project> {
      return invoke('create_project', { name, path });
    },

    async delete(id: string): Promise<void> {
      return invoke('delete_project', { id });
    },

    async open(): Promise<string | null> {
      return invoke('open_project_dialog');
    },
  },

  // Settings operations
  settings: {
    async get(): Promise<Settings> {
      return invoke('get_settings');
    },

    async update(settings: Partial<Settings>): Promise<Settings> {
      return invoke('update_settings', { settings });
    },
  },

  // File operations
  files: {
    async read(path: string): Promise<string> {
      return invoke('read_file', { path });
    },

    async write(path: string, content: string): Promise<void> {
      return invoke('write_file', { path, content });
    },

    async exists(path: string): Promise<boolean> {
      return invoke('file_exists', { path });
    },
  },
};

// ============ Event Helpers ============

export function useEventListener<T>(
  eventName: string,
  callback: (payload: T) => void
): () => void {
  let unlisten: UnlistenFn | null = null;

  onMount(async () => {
    unlisten = await listen<T>(eventName, (event) => {
      callback(event.payload);
    });
  });

  return () => {
    unlisten?.();
  };
}

// ============ Project Store Example ============

export function createProjectStore() {
  let projects = $state<Project[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);

  // Event cleanup function
  let cleanup: (() => void) | null = null;

  async function load() {
    loading = true;
    error = null;
    try {
      projects = await api.projects.list();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  async function create(name: string, path: string) {
    try {
      const project = await api.projects.create(name, path);
      projects = [...projects, project];
      return project;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      throw e;
    }
  }

  async function remove(id: string) {
    try {
      await api.projects.delete(id);
      projects = projects.filter((p) => p.id !== id);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      throw e;
    }
  }

  function subscribe() {
    onMount(async () => {
      await load();

      // Listen for project updates from backend
      cleanup = useEventListener<Project[]>('projects-changed', (updated) => {
        projects = updated;
      });
    });
  }

  function destroy() {
    cleanup?.();
  }

  return {
    get projects() { return projects; },
    get loading() { return loading; },
    get error() { return error; },
    load,
    create,
    remove,
    subscribe,
    destroy,
  };
}

// ============ Settings Store Example ============

export function createSettingsStore() {
  let settings = $state<Settings>({
    theme: 'dark',
    autoSave: true,
    editorFont: 'JetBrains Mono',
  });
  let loading = $state(false);

  async function load() {
    loading = true;
    try {
      settings = await api.settings.get();
    } finally {
      loading = false;
    }
  }

  async function update(updates: Partial<Settings>) {
    const previous = { ...settings };
    try {
      // Optimistic update
      settings = { ...settings, ...updates };
      await api.settings.update(settings);
    } catch (e) {
      // Revert on error
      settings = previous;
      throw e;
    }
  }

  onMount(load);

  return {
    get settings() { return settings; },
    get loading() { return loading; },
    update,
    reload: load,
  };
}

// ============ File Watcher Example ============

export function useFileWatcher(
  path: string,
  onChange: (content: string) => void
) {
  let watching = $state(false);
  let cleanup: (() => void) | null = null;

  async function start() {
    await invoke('watch_file', { path });
    watching = true;

    cleanup = useEventListener<{ path: string; content: string }>(
      'file-changed',
      (event) => {
        if (event.path === path) {
          onChange(event.content);
        }
      }
    );
  }

  async function stop() {
    await invoke('unwatch_file', { path });
    watching = false;
    cleanup?.();
  }

  onMount(start);

  return {
    get watching() { return watching; },
    start,
    stop,
  };
}
