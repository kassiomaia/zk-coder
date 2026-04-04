/**
 * File Events
 * Handles file touch events.
 */

import type { AgentEvent } from '$lib/types/agent';
import type { AgentState } from '../state';

/**
 * Handle file_touched event.
 */
export function handleFileTouched(
	state: AgentState,
	event: Extract<AgentEvent, { type: 'file_touched' }>
) {
	const session = state.sessions.get(event.sessionId);
	if (session) {
		const existingFiles = session.filesTouched ?? [];
		// Check if this file is already tracked
		const fileIndex = existingFiles.findIndex((f) => f.path === event.filePath);
		const newFile = {
			path: event.filePath,
			modified: event.action !== 'read',
			action: event.action,
			// Include diff if provided by the event
			...(event.diff !== undefined ? { diff: event.diff } : {})
		} as import('$lib/types/agent').TouchedFile;

		let updatedFiles;
		if (fileIndex >= 0) {
			// Update existing file entry, preserving existing diff if new one not provided
			updatedFiles = [...existingFiles];
			updatedFiles[fileIndex] = {
				...newFile,
				// Keep existing diff if the new event doesn't have one
				diff: event.diff ?? existingFiles[fileIndex].diff
			};
		} else {
			// Add new file
			updatedFiles = [...existingFiles, newFile];
		}

		state.sessions.set(event.sessionId, {
			...session,
			filesTouched: updatedFiles
		});
	}
}
