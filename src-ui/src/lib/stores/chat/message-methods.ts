/**
 * Message CRUD methods extracted from the chat store during Phase 7
 * of the SOLID refactor.
 *
 * Orchestration-heavy methods (`sendMessage`, `sendMessageWithLlm`) stay in
 * the host store because they span threads, conversations, the pending
 * thread-creation promise, and the LLM handler. Everything here is a
 * straightforward Message-only mutation.
 */

import { addContextMessage } from '$lib/api/nlp';
import {
	deleteMessage as apiDeleteMessage,
	updateMessageFeedback as apiUpdateMessageFeedback,
	updateMessageStatus as apiUpdateMessageStatus
} from '$lib/api/chat';
import { removeById } from '$lib/utils/collection';
import { createLogger } from '$lib/utils/logger';
import type { Message } from '$lib/types/message';
import type { ChatState } from './types';
import { documentGenerationStore } from '$lib/stores/documentGenerationStore';

type Updater = (fn: (state: ChatState) => ChatState) => void;
type GetState = () => ChatState;

interface Deps {
	update: Updater;
	getState: GetState;
}

const log = createLogger('chatStore.messages');

export function createMessageMethods({ update, getState }: Deps) {
	async function addMessage(message: Message) {
		update((state) => {
			// Only add if message belongs to the current view
			if (
				message.threadId &&
				state.selectedThreadId &&
				message.threadId !== state.selectedThreadId
			) {
				return state;
			}
			return { ...state, messages: [...state.messages, message] };
		});

		// Feed the append into doc-gen tracking. Previously this was a per-render
		// `$effect` in ChatMessagesList that scanned every message on every reactive
		// update (O(N) per streaming tick). Hooking it here makes it O(1) per append.
		documentGenerationStore.processMessage(message);

		const state = getState();
		if (state.nlp.backendAvailable && message.conversationId) {
			try {
				await addContextMessage({
					conversationId: message.conversationId,
					role: message.role,
					content: message.content
				});
			} catch (error) {
				log.error('Failed to sync message with backend:', error);
			}
		}
	}

	async function deleteMessage(id: string): Promise<boolean> {
		try {
			await apiDeleteMessage(id);
			update((state) => ({
				...state,
				messages: removeById(state.messages, id)
			}));
			return true;
		} catch (error) {
			update((state) => ({
				...state,
				error: error instanceof Error ? error.message : 'Failed to delete message'
			}));
			return false;
		}
	}

	function removeTransientMessages(sessionId: string) {
		update((state) => ({
			...state,
			messages: state.messages.filter((m) => {
				if (m.agentData?.sessionId !== sessionId) return true;
				if (m.kind === 'tool_call' || m.kind === 'thinking') return false;
				return true;
			})
		}));
	}

	async function updateMessageStatus(
		id: string,
		status: 'sending' | 'sent' | 'delivered' | 'error'
	): Promise<boolean> {
		try {
			await apiUpdateMessageStatus(id, status);
			update((state) => ({
				...state,
				messages: state.messages.map((m) => (m.id === id ? { ...m, status } : m))
			}));
			return true;
		} catch (error) {
			log.error('Failed to update message status:', error);
			return false;
		}
	}

	async function updateMessageFeedback(
		id: string,
		feedback: 'like' | 'dislike' | null
	): Promise<boolean> {
		try {
			await apiUpdateMessageFeedback(id, feedback);
			update((state) => ({
				...state,
				messages: state.messages.map((m) =>
					m.id === id
						? {
								...m,
								metadata: {
									...m.metadata,
									feedback: feedback ?? undefined
								}
							}
						: m
				)
			}));
			return true;
		} catch (error) {
			log.error('Failed to update message feedback:', error);
			return false;
		}
	}

	function updateMessage(id: string, updates: Partial<Message>) {
		let merged: Message | null = null;
		update((state) => {
			const existing = state.messages.find((m) => m.id === id);
			if (!existing) {
				return state;
			}
			merged = { ...existing, ...updates };
			return {
				...state,
				messages: state.messages.map((m) => (m === existing ? merged! : m))
			};
		});

		// Tool-call status/result updates arrive via this path (agent event bus).
		// Doc-gen tracking needs to see them to advance phase / mark completion.
		if (merged) {
			documentGenerationStore.processMessage(merged);
		}
	}

	return {
		addMessage,
		deleteMessage,
		removeTransientMessages,
		updateMessageStatus,
		updateMessageFeedback,
		updateMessage
	};
}
