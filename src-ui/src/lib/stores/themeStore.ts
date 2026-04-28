/**
 * Theme Store
 *
 * Single source of truth for the UI theme. Owns three concerns:
 *
 *   1. The user preference — `'light' | 'dark' | 'system'` — persisted to
 *      coder_config.db via Tauri commands so it survives app restarts.
 *   2. The resolved theme — `'light' | 'dark'` — what's actually painted.
 *      When the preference is `'system'` this tracks the OS color scheme.
 *   3. The DOM side-effect — toggling `data-theme` on `<html>` so the
 *      design tokens defined in `design-system.css` cascade correctly.
 *
 * Components that need to *react* to theme changes (e.g. Monaco) should
 * subscribe to the `theme` store. Components that toggle the preference
 * (the Settings UI) call `themeStore.setPreference()`.
 */

import { derived, writable, get } from 'svelte/store';
import { getUiTheme, setUiTheme, type UiThemePreference } from '$lib/api/config';
import { createLogger } from '$lib/utils/logger';

const log = createLogger('themeStore');

const STORAGE_KEY = 'theme';
const SYSTEM_QUERY = '(prefers-color-scheme: dark)';
const THEME_ATTR = 'data-theme';

export type ResolvedTheme = 'light' | 'dark';
export type ThemePreference = UiThemePreference;

/** What the user picked. `'system'` means follow the OS. */
const preference = writable<ThemePreference>('system');

/** The OS-reported preferred color scheme; updates when the OS changes. */
const systemTheme = writable<ResolvedTheme>('light');

/** The theme actually applied to the DOM. */
export const theme = derived(
	[preference, systemTheme],
	([$preference, $systemTheme]): ResolvedTheme =>
		$preference === 'system' ? $systemTheme : $preference
);

/** Read-only view of the user preference for the Settings UI. */
export const themePreference = {
	subscribe: preference.subscribe
};

/** Currently resolved theme — useful for non-reactive consumers (e.g. snapshots). */
export function getResolvedTheme(): ResolvedTheme {
	return get(theme);
}

function readSystemTheme(): ResolvedTheme {
	if (typeof window === 'undefined' || !window.matchMedia) return 'light';
	return window.matchMedia(SYSTEM_QUERY).matches ? 'dark' : 'light';
}

function applyTheme(resolved: ResolvedTheme): void {
	if (typeof document === 'undefined') return;
	document.documentElement.setAttribute(THEME_ATTR, resolved);
}

function readLegacyLocalStoragePreference(): ThemePreference | null {
	if (typeof localStorage === 'undefined') return null;
	const raw = localStorage.getItem(STORAGE_KEY);
	if (raw === 'light' || raw === 'dark' || raw === 'system') return raw;
	return null;
}

/**
 * Hydrates the store from the backend (or, on first run, from the legacy
 * localStorage key written by the previous theme implementation), wires up the
 * OS color-scheme listener, and applies `data-theme` to the document.
 *
 * Returns a teardown function for the OS-listener.
 */
export async function initTheme(): Promise<() => void> {
	systemTheme.set(readSystemTheme());

	let initial: ThemePreference = 'system';
	try {
		const persisted = await getUiTheme();
		if (persisted) {
			initial = persisted;
		} else {
			// Migrate the previous localStorage-backed preference once.
			const legacy = readLegacyLocalStoragePreference();
			if (legacy) {
				initial = legacy;
				try {
					await setUiTheme(legacy);
				} catch (err) {
					log.warn('Failed to migrate legacy theme to backend', err);
				}
			}
		}
	} catch (err) {
		log.warn('Failed to load persisted theme; using system default', err);
	}
	preference.set(initial);

	// Apply once synchronously so the first paint sees the correct theme.
	applyTheme(initial === 'system' ? readSystemTheme() : initial);

	// Keep the DOM in sync with future store changes.
	const unsubscribeTheme = theme.subscribe(applyTheme);

	let detachMedia: (() => void) | null = null;
	if (typeof window !== 'undefined' && window.matchMedia) {
		const mql = window.matchMedia(SYSTEM_QUERY);
		const handler = (event: MediaQueryListEvent) => {
			systemTheme.set(event.matches ? 'dark' : 'light');
		};
		mql.addEventListener('change', handler);
		detachMedia = () => mql.removeEventListener('change', handler);
	}

	return () => {
		unsubscribeTheme();
		detachMedia?.();
	};
}

/** Persists and applies a new user preference. */
async function setPreference(next: ThemePreference): Promise<void> {
	preference.set(next);
	try {
		await setUiTheme(next);
		// Keep the legacy key in sync so a downgrade doesn't lose the choice.
		if (typeof localStorage !== 'undefined') {
			localStorage.setItem(STORAGE_KEY, next);
		}
	} catch (err) {
		log.error('Failed to persist theme preference', err);
		throw err;
	}
}

export const themeStore = {
	subscribe: preference.subscribe,
	setPreference
};
