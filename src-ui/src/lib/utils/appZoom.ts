/**
 * App Zoom Store
 *
 * Persists the scale factor applied to the app shell content (everything
 * below the title bar). Exposed as a `--app-zoom` CSS variable on the
 * shell-scaler wrapper and consumed by a `transform: scale()` on the
 * main shell, so the entire UI — sidebars, chat, right panel — zooms
 * together while the AppHeader stays at native size.
 *
 * Driven by CTRL + wheel on the app shell (see AppShell.svelte).
 */

import { writable, get } from 'svelte/store';
import { browser } from '$app/environment';

export const MIN_ZOOM = 0.8;
export const MAX_ZOOM = 1.6;
export const ZOOM_STEP = 0.05;
export const DEFAULT_ZOOM = 1.0;

const STORAGE_KEY = 'app-zoom-level';

function clamp(value: number): number {
	if (!Number.isFinite(value)) return DEFAULT_ZOOM;
	if (value < MIN_ZOOM) return MIN_ZOOM;
	if (value > MAX_ZOOM) return MAX_ZOOM;
	return Math.round(value * 100) / 100;
}

function loadInitial(): number {
	if (!browser) return DEFAULT_ZOOM;
	try {
		const raw = localStorage.getItem(STORAGE_KEY);
		if (raw == null) return DEFAULT_ZOOM;
		const parsed = parseFloat(raw);
		return clamp(parsed);
	} catch {
		return DEFAULT_ZOOM;
	}
}

export const appZoom = writable<number>(loadInitial());

let persistTimer: ReturnType<typeof setTimeout> | null = null;

if (browser) {
	appZoom.subscribe((value) => {
		if (persistTimer) clearTimeout(persistTimer);
		persistTimer = setTimeout(() => {
			try {
				localStorage.setItem(STORAGE_KEY, String(value));
			} catch {
				// localStorage may be unavailable (private mode, quota) — ignore
			}
		}, 200);
	});
}

export function setZoom(value: number): void {
	appZoom.set(clamp(value));
}

export function zoomIn(): void {
	appZoom.set(clamp(get(appZoom) + ZOOM_STEP));
}

export function zoomOut(): void {
	appZoom.set(clamp(get(appZoom) - ZOOM_STEP));
}

export function resetZoom(): void {
	appZoom.set(DEFAULT_ZOOM);
}
