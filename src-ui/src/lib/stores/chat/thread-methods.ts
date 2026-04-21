/**
 * Thread CRUD methods extracted from the chat store during Phase 7
 * of the SOLID refactor.
 *
 * Orchestration-heavy methods (`selectThread`, `createThread`) stay in the
 * host store because they reach across to messages, agent sessions, and
 * streaming state. Everything here is a straightforward Thread-only mutation.
 */

import {
	archiveThread as apiArchiveThread,
	deleteThread as apiDeleteThread,
	renameThread as apiRenameThread,
	resolveThread as apiResolveThread,
	setYoloMode as apiSetYoloMode
} from '$lib/api/chat';
import { removeById, removeNestedById, updateById, updateNestedById } from '$lib/utils/collection';
import { agentStore } from '../agentStore';
import type { ChatState } from './types';

type Updater = (fn: (state: ChatState) => ChatState) => void;
type GetState = () => ChatState;

interface Deps {
	update: Updater;
	getState: GetState;
}

export function createThreadMethods({ update, getState }: Deps) {
	async function resolveThread(id: string, resolved: boolean): Promise<boolean> {
		try {
			await apiResolveThread(id, resolved);
			update((state) => ({
				...state,
				threads: state.threads.map((t) =>
					t.id === id ? { ...t, metadata: { ...t.metadata, resolved }, updatedAt: new Date() } : t
				),
				conversations: state.conversations.map((c) =>
					c.id === state.selectedConversationId
						? {
								...c,
								threads: (c.threads || []).map((t) =>
									t.id === id
										? {
												...t,
												metadata: { ...t.metadata, resolved },
												updatedAt: new Date()
											}
										: t
								)
							}
						: c
				)
			}));
			return true;
		} catch (error) {
			update((state) => ({
				...state,
				error: error instanceof Error ? error.message : 'Failed to update thread'
			}));
			return false;
		}
	}

	async function archiveThread(id: string, archived: boolean): Promise<boolean> {
		try {
			await apiArchiveThread(id, archived);
			if (archived && getState().selectedThreadId === id) {
				agentStore.setActiveSession(null);
			}
			update((state) => {
				const shouldClearSelection = archived && state.selectedThreadId === id;
				return {
					...state,
					threads: archived ? removeById(state.threads, id) : state.threads,
					conversations: state.selectedConversationId
						? removeNestedById(state.conversations, state.selectedConversationId, 'threads', id)
						: state.conversations,
					selectedThreadId: shouldClearSelection ? null : state.selectedThreadId,
					messages: shouldClearSelection ? [] : state.messages,
					unreadThreadIds: state.unreadThreadIds.filter((threadId) => threadId !== id)
				};
			});
			return true;
		} catch (error) {
			update((state) => ({
				...state,
				error: error instanceof Error ? error.message : 'Failed to archive thread'
			}));
			return false;
		}
	}

	async function setYoloMode(id: string, yoloMode: boolean): Promise<boolean> {
		try {
			await apiSetYoloMode(id, yoloMode);
			update((state) => ({
				...state,
				threads: state.threads.map((t) =>
					t.id === id ? { ...t, metadata: { ...t.metadata, yoloMode }, updatedAt: new Date() } : t
				),
				conversations: state.conversations.map((c) =>
					c.id === state.selectedConversationId
						? {
								...c,
								threads: (c.threads || []).map((t) =>
									t.id === id
										? {
												...t,
												metadata: { ...t.metadata, yoloMode },
												updatedAt: new Date()
											}
										: t
								)
							}
						: c
				)
			}));
			return true;
		} catch (error) {
			update((state) => ({
				...state,
				error: error instanceof Error ? error.message : 'Failed to update thread'
			}));
			return false;
		}
	}

	async function deleteThread(id: string): Promise<boolean> {
		try {
			await apiDeleteThread(id);
			update((state) => ({
				...state,
				threads: removeById(state.threads, id),
				threadStreaming: Object.fromEntries(
					Object.entries(state.threadStreaming).filter(([threadId]) => threadId !== id)
				),
				unreadThreadIds: state.unreadThreadIds.filter((threadId) => threadId !== id),
				selectedThreadId: state.selectedThreadId === id ? null : state.selectedThreadId,
				messages: state.selectedThreadId === id ? [] : state.messages,
				conversations: state.selectedConversationId
					? removeNestedById(state.conversations, state.selectedConversationId, 'threads', id)
					: state.conversations
			}));
			return true;
		} catch (error) {
			update((state) => ({
				...state,
				error: error instanceof Error ? error.message : 'Failed to delete thread'
			}));
			return false;
		}
	}

	async function renameThread(id: string, title: string): Promise<boolean> {
		try {
			const updated = await apiRenameThread(id, title);
			update((state) => ({
				...state,
				threads: updateById(state.threads, id, updated),
				conversations: state.selectedConversationId
					? updateNestedById(
							state.conversations,
							state.selectedConversationId,
							'threads',
							id,
							updated
						)
					: state.conversations
			}));
			return true;
		} catch (error) {
			update((state) => ({
				...state,
				error: error instanceof Error ? error.message : 'Failed to rename thread'
			}));
			return false;
		}
	}

	return {
		resolveThread,
		archiveThread,
		setYoloMode,
		deleteThread,
		renameThread
	};
}
