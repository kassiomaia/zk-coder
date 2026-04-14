/**
 * Workspace Search Policy API Client
 *
 * Thin IPC wrappers around the `*_workspace_search_policy` Tauri commands.
 * The policy is scoped by canonicalized workspace path; use the empty
 * string as a sentinel for "no workspace" (global fallback).
 */

import { invokeCommand } from './core';

export { ConfigApiError } from './errors';

/**
 * Allow/deny domain lists for one workspace. Missing rows surface as empty
 * arrays (the backend returns an empty policy, never `null`).
 */
export interface WorkspaceSearchPolicy {
	allowlist: string[];
	denylist: string[];
}

/**
 * Reads the policy for `workspacePath`. Returns empty lists when no row
 * is stored.
 */
export function getWorkspaceSearchPolicy(
	workspacePath: string
): Promise<WorkspaceSearchPolicy> {
	return invokeCommand<WorkspaceSearchPolicy>(
		'config',
		'get_workspace_search_policy',
		{ workspacePath }
	);
}

/**
 * Upserts the policy for `workspacePath`.
 */
export function setWorkspaceSearchPolicy(
	workspacePath: string,
	allowlist: string[],
	denylist: string[]
): Promise<void> {
	return invokeCommand('config', 'set_workspace_search_policy', {
		workspacePath,
		allowlist,
		denylist
	});
}

/**
 * Deletes any stored policy for `workspacePath`. No-op when absent.
 */
export function deleteWorkspaceSearchPolicy(workspacePath: string): Promise<void> {
	return invokeCommand('config', 'delete_workspace_search_policy', {
		workspacePath
	});
}
