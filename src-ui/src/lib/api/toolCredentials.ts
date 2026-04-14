/**
 * Tool Credentials API Client
 *
 * Thin IPC wrappers around the `*_tool_credentials` and
 * `test_web_search_credentials` Tauri commands. Stable types so the
 * Settings UI can consume them without reaching into `invoke` directly.
 */

import { invokeCommand } from './core';
import type { ValidationResult } from '$lib/types/config';

export { ConfigApiError } from './errors';

/**
 * Stable tool id used by the web-search credentials surface.
 * Mirrors `WEB_SEARCH_TOOL_ID` on the Rust side.
 */
export const WEB_SEARCH_TOOL_ID = 'search_web_search' as const;

export interface WebSearchCredentials {
	apiKey: string;
	cx: string;
}

/**
 * Persists (or replaces) the credentials for `toolId`.
 * Empty `apiKey` / `cx` are rejected by the backend with a descriptive error.
 */
export function setToolCredentials(
	toolId: string,
	credentials: WebSearchCredentials
): Promise<void> {
	return invokeCommand('config', 'set_tool_credentials', {
		toolId,
		apiKey: credentials.apiKey,
		cx: credentials.cx
	});
}

/**
 * Reads the stored credentials for `toolId`, or `null` when absent.
 */
export function getToolCredentials(
	toolId: string
): Promise<WebSearchCredentials | null> {
	return invokeCommand<WebSearchCredentials | null>(
		'config',
		'get_tool_credentials',
		{ toolId }
	);
}

/**
 * Removes any stored credentials for `toolId`. No-op when absent.
 */
export function deleteToolCredentials(toolId: string): Promise<void> {
	return invokeCommand('config', 'delete_tool_credentials', { toolId });
}

/**
 * Returns `true` when credentials are persisted for `toolId`.
 */
export function hasToolCredentials(toolId: string): Promise<boolean> {
	return invokeCommand<boolean>('config', 'has_tool_credentials', { toolId });
}

/**
 * Runs a canned query against the currently-stored web-search credentials.
 * `valid: false` carries a human-readable message suitable for inline
 * rendering in the Settings form (network, unauthorized, quota, etc.).
 */
export function testWebSearchCredentials(): Promise<ValidationResult> {
	return invokeCommand<ValidationResult>(
		'config',
		'test_web_search_credentials'
	);
}
