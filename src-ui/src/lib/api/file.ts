/**
 * File viewing API for the UI.
 *
 * These functions provide file content access for the frontend UI,
 * separate from agent tool execution. They don't consume LLM tokens.
 */

import { invokeCommand } from './core';

// Re-export error for backwards compatibility
export { FileApiError } from './errors';

/**
 * Response containing file content and diff information.
 */
export interface FileContentResponse {
	/** File path relative to workspace */
	path: string;
	/** Whether the file currently exists on disk */
	exists: boolean;
	/** Current file content (if file exists) */
	current_content: string | null;
	/** Original content from rollback store (before agent modifications) */
	original_content: string | null;
	/** Generated diff (if both contents exist or file is new/deleted) */
	diff: string | null;
	/** Whether this is a new file (no original content) */
	is_new: boolean;
}

/**
 * Get file content for UI viewing.
 *
 * This is separate from agent tools and doesn't consume LLM tokens.
 * It returns current content, original content (from rollback store),
 * and a generated diff.
 *
 * @param workspaceRoot - The workspace root directory path
 * @param filePath - The file path relative to workspace
 * @param sessionId - The agent session ID
 * @returns File content response with diff information
 */
export async function getFileContent(
	workspaceRoot: string,
	filePath: string,
	sessionId: string
): Promise<FileContentResponse> {
	return invokeCommand<FileContentResponse>('file', 'get_file_content', {
		workspaceRoot,
		filePath,
		sessionId
	});
}

/**
 * Open the dedicated file-diff window for a path within a session's
 * workspace. The backend spawns the window; errors surface as
 * `FileApiError`.
 */
export async function openFileDiffWindow(
	filePath: string,
	workspaceRoot: string,
	sessionId: string
): Promise<void> {
	await invokeCommand<void>('file', 'open_file_diff_window', {
		filePath,
		workspaceRoot,
		sessionId
	});
}
