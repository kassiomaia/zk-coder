/**
 * Prompt Templates Store
 * Loads and mutates slash-command prompt templates through the config API,
 * exposing them both as raw `PromptTemplate`s (for the settings UI) and as
 * `Command`s (for the chat palette).
 */

import { derived, writable } from 'svelte/store';
import type { Command } from '$lib/types/command';
import { CommandCategory } from '$lib/types/command';
import type { LoadingState } from '$lib/types/state';
import {
	deletePromptTemplate as apiDeletePromptTemplate,
	listPromptTemplates as apiListPromptTemplates,
	resetPromptTemplate as apiResetPromptTemplate,
	savePromptTemplate as apiSavePromptTemplate,
	type PromptTemplate,
	type SavePromptTemplateRequest
} from '$lib/api/config';

export type { PromptTemplate, SavePromptTemplateRequest };
import { createLogger } from '$lib/utils/logger';

const log = createLogger('PromptTemplatesStore');

export interface PromptTemplatesState {
	templates: PromptTemplate[];
	loading: LoadingState;
	error: string | null;
}

const initialState: PromptTemplatesState = {
	templates: [],
	loading: 'idle',
	error: null
};

function createPromptTemplatesStore() {
	const { subscribe, update } = writable<PromptTemplatesState>(initialState);

	async function load(): Promise<void> {
		update((s) => ({ ...s, loading: 'loading', error: null }));
		try {
			const templates = await apiListPromptTemplates();
			update((s) => ({ ...s, templates, loading: 'success' }));
		} catch (error) {
			const message = error instanceof Error ? error.message : String(error);
			log.error('Failed to load prompt templates', error);
			update((s) => ({ ...s, loading: 'error', error: message }));
		}
	}

	async function save(request: SavePromptTemplateRequest): Promise<PromptTemplate> {
		const saved = await apiSavePromptTemplate(request);
		update((s) => {
			const next = s.templates.filter((t) => t.id !== saved.id);
			next.push(saved);
			next.sort(sortTemplates);
			return { ...s, templates: next, error: null };
		});
		return saved;
	}

	async function remove(id: string): Promise<void> {
		await apiDeletePromptTemplate(id);
		update((s) => ({
			...s,
			templates: s.templates.filter((t) => t.id !== id),
			error: null
		}));
	}

	async function reset(slug: string): Promise<PromptTemplate> {
		const restored = await apiResetPromptTemplate(slug);
		update((s) => {
			const next = s.templates.filter((t) => t.slug !== slug);
			next.push(restored);
			next.sort(sortTemplates);
			return { ...s, templates: next, error: null };
		});
		return restored;
	}

	function clearError(): void {
		update((s) => ({ ...s, error: null }));
	}

	return {
		subscribe,
		initialize: load,
		reload: load,
		save,
		remove,
		reset,
		clearError
	};
}

function sortTemplates(a: PromptTemplate, b: PromptTemplate): number {
	if (a.sortOrder !== b.sortOrder) return a.sortOrder - b.sortOrder;
	return a.slug.localeCompare(b.slug);
}

export const promptTemplatesStore = createPromptTemplatesStore();

/** Derived `Command[]` adapter for the chat palette. Disabled templates are hidden. */
export const promptTemplateCommands = derived(promptTemplatesStore, ($state): Command[] =>
	$state.templates
		.filter((t) => t.isEnabled)
		.map(toCommand)
);

/** Map a `PromptTemplate` to the palette's `Command` shape. */
export function toCommand(template: PromptTemplate): Command {
	return {
		id: template.id,
		name: template.slug,
		description: template.description,
		icon: template.icon ?? undefined,
		category: normalizeCategory(template.category)
	};
}

function normalizeCategory(raw: string): CommandCategory {
	const normalized = raw.trim().toLowerCase();
	const known = Object.values(CommandCategory) as string[];
	return (known.includes(normalized) ? normalized : CommandCategory.CODE) as CommandCategory;
}
