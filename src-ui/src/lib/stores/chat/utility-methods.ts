/**
 * Utility methods extracted from the chat store during Phase 7 of the SOLID
 * refactor. These are thin helpers that touch a single state field and do
 * not orchestrate across domains.
 */

import { clearUnreadForThread } from '$lib/utils/state';
import { createLogger } from '$lib/utils/logger';
import type { ChatState } from './types';

type Updater = (fn: (state: ChatState) => ChatState) => void;

interface Deps {
	update: Updater;
}

const log = createLogger('chatStore.util');

export function createUtilityMethods({ update }: Deps) {
	function clearError() {
		update((state) => ({ ...state, error: null }));
	}

	function markThreadRead(threadId: string) {
		update((state) => {
			if (!state.unreadThreadIds.includes(threadId)) {
				return state;
			}
			return {
				...state,
				unreadThreadIds: clearUnreadForThread(state.unreadThreadIds, threadId)
			};
		});
	}

	async function setThreadWorkspace(id: string, workspaceRoot: string): Promise<boolean> {
		try {
			update((state) => ({
				...state,
				threads: state.threads.map((t) =>
					t.id === id ? { ...t, metadata: { ...t.metadata, workspaceRoot } } : t
				)
			}));
			return true;
		} catch (error) {
			log.error('Failed to set thread workspace:', error);
			return false;
		}
	}

	return {
		clearError,
		markThreadRead,
		setThreadWorkspace
	};
}
