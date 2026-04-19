/**
 * Workspace file listing API.
 *
 * Thin wrapper around the `list_workspace_dir` Tauri command, which
 * returns the immediate children of one directory under the workspace
 * root. The FileTree component calls this on expand to stay lazy.
 */

import { invoke } from '@tauri-apps/api/core';
import type { WorkspaceEntry } from '$lib/types/agent';

export interface WorkspaceDirResponse {
	entries: WorkspaceEntry[];
	/** `true` when the folder contained more entries than the backend cap. */
	truncated: boolean;
}

/**
 * List the immediate children of `path` under `root`.
 *
 * Pass an empty `path` to list the workspace root itself. Paths use
 * forward-slash separators and must stay inside the workspace (leading
 * `/`, `..`, and absolute paths are rejected by the backend).
 */
export async function listWorkspaceDir(
	root: string,
	path: string
): Promise<WorkspaceDirResponse> {
	return invoke<WorkspaceDirResponse>('list_workspace_dir', { root, path });
}
