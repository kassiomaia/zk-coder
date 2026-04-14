/**
 * Shell / system URL helpers.
 *
 * Wraps `@tauri-apps/plugin-opener` so the rest of the codebase never has to
 * reach into the plugin directly. The wrapper validates the scheme before
 * invoking the plugin: only `http:` and `https:` URLs are permitted, which
 * matches the capability scope declared in
 * `src-tauri/capabilities/default.json`. Any other scheme is refused with a
 * thrown `Error` so a caller that accidentally receives a `javascript:` or
 * `file:` URL cannot turn it into a live click.
 */

import { openUrl as openUrlPlugin } from '@tauri-apps/plugin-opener';

const SAFE_SCHEMES = new Set(['http:', 'https:']);

/**
 * Opens `url` in the user's default browser. Only `http(s)` URLs are
 * accepted — anything else throws.
 */
export async function openUrl(url: string): Promise<void> {
	let parsed: URL;
	try {
		parsed = new URL(url);
	} catch {
		throw new Error(`Refusing to open URL: not a valid absolute URL: ${url}`);
	}
	if (!SAFE_SCHEMES.has(parsed.protocol)) {
		throw new Error(
			`Refusing to open URL with scheme '${parsed.protocol}' — only http(s) is allowed.`
		);
	}
	await openUrlPlugin(parsed.toString());
}

/**
 * Pure predicate — no IPC, no side effects. Useful for feature guards in
 * the UI (hide the button when the URL would be rejected).
 */
export function isOpenableUrl(url: string): boolean {
	try {
		const parsed = new URL(url);
		return SAFE_SCHEMES.has(parsed.protocol);
	} catch {
		return false;
	}
}
