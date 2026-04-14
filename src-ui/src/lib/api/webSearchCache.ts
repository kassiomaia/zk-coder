/**
 * Web Search Cache API Client
 *
 * IPC wrappers for the `web_search_cache_stats` and `clear_web_search_cache`
 * Tauri commands.
 */

import { invokeCommand } from './core';

export { ConfigApiError } from './errors';

export interface WebSearchCacheStats {
	entries: number;
	defaultTtlSecs: number;
}

/**
 * Returns the current entry count and default TTL (in seconds). The entry
 * count includes not-yet-swept expired rows.
 */
export function getWebSearchCacheStats(): Promise<WebSearchCacheStats> {
	return invokeCommand<WebSearchCacheStats>(
		'config',
		'web_search_cache_stats'
	);
}

/**
 * Clears cache entries. Pass a `workspacePath` to clear a single partition,
 * or omit it (pass `undefined`) to clear the whole table. Returns the
 * number of rows removed.
 */
export function clearWebSearchCache(
	workspacePath?: string
): Promise<number> {
	return invokeCommand<number>('config', 'clear_web_search_cache', {
		workspacePath: workspacePath ?? null
	});
}
