/**
 * Agent Event Bus
 *
 * Typed pub/sub that decouples the agent store from the chat store.
 * Replaces the previous `setXxxCallback` registration pattern:
 * subscribers call `agentEventBus.on(event, handler)` and receive
 * an unsubscribe function; emitters call `agentEventBus.emit(event, payload)`.
 *
 * Multiple handlers per event are supported — new subscribers no longer
 * require surgery on the agent store.
 */

import type { Message } from '$lib/types/message';
import type { ArtifactReference } from '$lib/types/agent';
import { createLogger } from '$lib/utils/logger';

const log = createLogger('agentEventBus');

export interface SystemMessagePayload {
	role: 'system';
	content: string;
	metadata?: Message['metadata'];
}

export interface AgentEventMap {
	'execution-complete': {
		sessionId: string;
		conversationId: string;
		threadId: string;
		content: string;
		artifacts: ArtifactReference[];
	};
	'system-message': {
		sessionId: string;
		conversationId: string;
		threadId: string;
		message: SystemMessagePayload;
	};
	'agent-activity': {
		conversationId: string;
		threadId: string;
		message: Partial<Message>;
	};
	'cleanup-transients': {
		conversationId: string;
		threadId: string;
		sessionId: string;
	};
	'persist-assistant-message': {
		sessionId: string;
		conversationId: string;
		threadId: string;
		messageId: string;
		content: string;
	};
}

export type AgentEventName = keyof AgentEventMap;
export type AgentEventHandler<E extends AgentEventName> = (payload: AgentEventMap[E]) => void;

type AnyHandler = (payload: unknown) => void;

export interface AgentEventBus {
	on<E extends AgentEventName>(event: E, handler: AgentEventHandler<E>): () => void;
	emit<E extends AgentEventName>(event: E, payload: AgentEventMap[E]): void;
	clear(): void;
}

export function createAgentEventBus(): AgentEventBus {
	const handlers = new Map<AgentEventName, Set<AnyHandler>>();

	return {
		on(event, handler) {
			let set = handlers.get(event);
			if (!set) {
				set = new Set();
				handlers.set(event, set);
			}
			const wrapped = handler as AnyHandler;
			set.add(wrapped);
			return () => {
				set?.delete(wrapped);
			};
		},
		emit(event, payload) {
			const set = handlers.get(event);
			if (!set || set.size === 0) return;
			for (const handler of set) {
				try {
					handler(payload);
				} catch (error) {
					log.error(`Handler for "${String(event)}" threw:`, error);
				}
			}
		},
		clear() {
			handlers.clear();
		}
	};
}

export const agentEventBus: AgentEventBus = createAgentEventBus();
