/**
 * Conversation CRUD methods extracted from the chat store during Phase 7
 * of the SOLID refactor.
 *
 * Orchestration-heavy methods (`selectConversation`) that cross into thread
 * lifecycle stay in the host store so that `this.selectThread` resolves
 * without additional dependency wiring. Everything here is a straightforward
 * Conversation-only mutation.
 */

import {
	archiveConversation as apiArchiveConversation,
	createConversation as apiCreateConversation,
	deleteConversation as apiDeleteConversation,
	renameConversation as apiRenameConversation,
	setConversationWorkspace as apiSetConversationWorkspace,
	updateConversationSettings as apiUpdateConversationSettings
} from '$lib/api/chat';
import { removeById, updateById } from '$lib/utils/collection';
import type { Conversation, ConversationSettings } from '$lib/types/thread';
import { agentStore } from '../agentStore';
import type { ChatState } from './types';

type Updater = (fn: (state: ChatState) => ChatState) => void;
type GetState = () => ChatState;

interface Deps {
	update: Updater;
	getState: GetState;
}

export function createConversationMethods({ update, getState }: Deps) {
	function setConversations(conversations: Conversation[]) {
		update((state) => ({ ...state, conversations }));
	}

	async function createConversation(
		title: string = 'New Conversation'
	): Promise<Conversation | null> {
		try {
			const conversation = await apiCreateConversation(title);
			update((state) => ({
				...state,
				conversations: [conversation, ...state.conversations]
			}));
			return conversation;
		} catch (error) {
			update((state) => ({
				...state,
				error: error instanceof Error ? error.message : 'Failed to create conversation'
			}));
			return null;
		}
	}

	function addConversation(conversation: Conversation) {
		update((state) => ({
			...state,
			conversations: [conversation, ...state.conversations]
		}));
	}

	async function renameConversation(id: string, title: string): Promise<boolean> {
		try {
			const updated = await apiRenameConversation(id, title);
			update((state) => ({
				...state,
				conversations: state.conversations.map((c) =>
					c.id === id ? { ...c, title: updated.title, updatedAt: updated.updatedAt } : c
				)
			}));
			return true;
		} catch (error) {
			update((state) => ({
				...state,
				error: error instanceof Error ? error.message : 'Failed to rename conversation'
			}));
			return false;
		}
	}

	function updateConversation(id: string, updates: Partial<Conversation>) {
		update((state) => ({
			...state,
			conversations: updateById(state.conversations, id, updates)
		}));
	}

	async function deleteConversation(id: string): Promise<boolean> {
		try {
			await apiDeleteConversation(id);
			update((state) => ({
				...state,
				conversations: removeById(state.conversations, id),
				selectedConversationId:
					state.selectedConversationId === id ? null : state.selectedConversationId,
				messages: state.selectedConversationId === id ? [] : state.messages,
				threads: state.selectedConversationId === id ? [] : state.threads
			}));
			return true;
		} catch (error) {
			update((state) => ({
				...state,
				error: error instanceof Error ? error.message : 'Failed to delete conversation'
			}));
			return false;
		}
	}

	async function archiveConversation(id: string, archived: boolean): Promise<boolean> {
		try {
			await apiArchiveConversation(id, archived);
			if (archived && getState().selectedConversationId === id) {
				agentStore.setActiveSession(null);
			}
			update((state) => {
				const nextConversationId =
					archived && state.selectedConversationId === id ? null : state.selectedConversationId;
				return {
					...state,
					conversations: state.conversations.map((c) =>
						c.id === id ? { ...c, archived, updatedAt: new Date() } : c
					),
					selectedConversationId: nextConversationId,
					selectedThreadId:
						archived && state.selectedConversationId === id ? null : state.selectedThreadId,
					messages: archived && state.selectedConversationId === id ? [] : state.messages,
					threads: archived && state.selectedConversationId === id ? [] : state.threads
				};
			});
			return true;
		} catch (error) {
			update((state) => ({
				...state,
				error: error instanceof Error ? error.message : 'Failed to archive conversation'
			}));
			return false;
		}
	}

	async function updateConversationSettings(
		id: string,
		settings: Partial<ConversationSettings>
	): Promise<boolean> {
		try {
			const state = getState();
			const conversation = state.conversations.find((c) => c.id === id);
			if (!conversation) {
				update((s) => ({ ...s, error: 'Conversation not found' }));
				return false;
			}

			const newSettings: ConversationSettings = {
				...conversation.settings,
				...settings
			};

			const updated = await apiUpdateConversationSettings(id, {
				model: newSettings.model,
				temperature: newSettings.temperature,
				maxTokens: newSettings.maxTokens,
				systemPrompt: newSettings.systemPrompt,
				workspaceRoot: newSettings.workspaceRoot
			});

			update((s) => ({
				...s,
				conversations: s.conversations.map((c) =>
					c.id === id ? { ...c, settings: updated.settings } : c
				)
			}));
			return true;
		} catch (error) {
			update((s) => ({
				...s,
				error: error instanceof Error ? error.message : 'Failed to update settings'
			}));
			return false;
		}
	}

	async function setConversationWorkspace(id: string, workspaceRoot: string): Promise<boolean> {
		try {
			const updated = await apiSetConversationWorkspace(id, workspaceRoot);
			update((state) => ({
				...state,
				conversations: state.conversations.map((c) =>
					c.id === id
						? {
								...c,
								settings: { ...c.settings, workspaceRoot: updated.settings.workspaceRoot }
							}
						: c
				)
			}));
			return true;
		} catch (error) {
			update((state) => ({
				...state,
				error: error instanceof Error ? error.message : 'Failed to set workspace'
			}));
			return false;
		}
	}

	return {
		setConversations,
		createConversation,
		addConversation,
		renameConversation,
		updateConversation,
		deleteConversation,
		archiveConversation,
		updateConversationSettings,
		setConversationWorkspace
	};
}
