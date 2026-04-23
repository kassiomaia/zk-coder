/**
 * Thread Agent Panel Preferences
 *
 * Per-thread persistence of whether the right-side agent panel should be
 * open when the user views that thread. Backed by localStorage — a pure UI
 * preference that does not belong in thread metadata on the backend.
 */

import { createLogger } from '$lib/utils/logger';

const STORAGE_KEY = 'coder.threadAgentPanelPrefs';
const log = createLogger('threadAgentPanelPrefs');

type PrefsMap = Record<string, boolean>;

let cache: PrefsMap | null = null;

function isBrowser(): boolean {
	return typeof window !== 'undefined' && typeof window.localStorage !== 'undefined';
}

function load(): PrefsMap {
	if (cache !== null) return cache;
	if (!isBrowser()) {
		cache = {};
		return cache;
	}
	try {
		const raw = window.localStorage.getItem(STORAGE_KEY);
		if (!raw) {
			cache = {};
			return cache;
		}
		const parsed = JSON.parse(raw) as unknown;
		if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
			const clean: PrefsMap = {};
			for (const [key, value] of Object.entries(parsed as Record<string, unknown>)) {
				if (typeof value === 'boolean') clean[key] = value;
			}
			cache = clean;
		} else {
			cache = {};
		}
	} catch (err) {
		log.warn('Failed to read preferences, resetting:', err);
		cache = {};
	}
	return cache;
}

function persist(prefs: PrefsMap): void {
	if (!isBrowser()) return;
	try {
		window.localStorage.setItem(STORAGE_KEY, JSON.stringify(prefs));
	} catch (err) {
		log.warn('Failed to persist preferences:', err);
	}
}

export const threadAgentPanelPrefs = {
	/**
	 * Get the saved preference for a thread. Default: false (closed).
	 */
	get(threadId: string): boolean {
		if (!threadId) return false;
		const prefs = load();
		return prefs[threadId] === true;
	},

	/**
	 * Persist the preference for a thread.
	 */
	set(threadId: string, open: boolean): void {
		if (!threadId) return;
		const prefs = load();
		if (prefs[threadId] === open) return;
		prefs[threadId] = open;
		persist(prefs);
	},

	/**
	 * Remove the saved preference for a thread (e.g. after deletion).
	 */
	remove(threadId: string): void {
		if (!threadId) return;
		const prefs = load();
		if (!(threadId in prefs)) return;
		delete prefs[threadId];
		persist(prefs);
	}
};
