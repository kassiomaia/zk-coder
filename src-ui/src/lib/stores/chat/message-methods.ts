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
		update((state) => {
			const messageExists = state.messages.some((m) => m.id === id);
			if (!messageExists) {
				return state;
			}
			return {
				...state,
				messages: state.messages.map((m) => (m.id === id ? { ...m, ...updates } : m))
			};
		});
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
