import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createInitialState } from '../state';
import type { AgentSession } from '$lib/types/agent';
import type { AgentState } from '../state';
import { handleStreamingContent } from './streamingEvents';

const streamingMocks = vi.hoisted(() => ({
	clearStreamingUpdates: vi.fn(),
	scheduleStreamingUpdate: vi.fn(),
	getOnPersistAssistantMessage: vi.fn(() => null)
}));

vi.mock('../streaming', () => ({
	clearStreamingUpdates: streamingMocks.clearStreamingUpdates,
	scheduleStreamingUpdate: streamingMocks.scheduleStreamingUpdate
}));

vi.mock('../callbacks', () => ({
	getOnAgentActivity: vi.fn(() => null),
	getOnSystemMessage: vi.fn(() => null),
	getOnPersistAssistantMessage: streamingMocks.getOnPersistAssistantMessage
}));

function makeSession(): AgentSession {
	return {
		id: 'session-1',
		agentId: 'code-agent',
		agentName: 'Code Agent',
		conversationId: 'conv-1',
		threadId: 'thread-1',
		state: { type: 'running', currentStep: 1, contextUsage: 0 },
		createdAt: new Date().toISOString(),
		updatedAt: new Date().toISOString()
	};
}

function makeState(): AgentState {
	const state = createInitialState();
	state.sessions.set('session-1', makeSession());
	return state;
}

describe('handleStreamingContent', () => {
	beforeEach(() => {
		streamingMocks.clearStreamingUpdates.mockReset();
		streamingMocks.scheduleStreamingUpdate.mockReset();
		streamingMocks.getOnPersistAssistantMessage.mockReset();
		streamingMocks.getOnPersistAssistantMessage.mockReturnValue(null);
	});

	it('accumulates non-final chunks and schedules throttled updates', () => {
		const state = makeState();

		handleStreamingContent(state, {
			type: 'streaming_content',
			sessionId: 'session-1',
			conversationId: 'conv-1',
			threadId: 'thread-1',
			chunk: 'Hello',
			isFinal: false
		});

		handleStreamingContent(state, {
			type: 'streaming_content',
			sessionId: 'session-1',
			conversationId: 'conv-1',
			threadId: 'thread-1',
			chunk: ' world',
			isFinal: false
		});

		expect(state.sessions.get('session-1')?.streamingMessages).toEqual({
			'streaming-assistant-session-1': 'Hello world'
		});
		expect(streamingMocks.scheduleStreamingUpdate).toHaveBeenLastCalledWith(
			'session-1:streaming-assistant-session-1',
			'conv-1',
			'thread-1',
			'streaming-assistant-session-1',
			'Hello world',
			'sending'
		);
	});

	it('flushes the full accumulated content immediately on the final chunk', () => {
		const state = makeState();

		handleStreamingContent(state, {
			type: 'streaming_content',
			sessionId: 'session-1',
			conversationId: 'conv-1',
			threadId: 'thread-1',
			chunk: 'Hello',
			isFinal: false
		});

		handleStreamingContent(state, {
			type: 'streaming_content',
			sessionId: 'session-1',
			conversationId: 'conv-1',
			threadId: 'thread-1',
			chunk: ' world',
			isFinal: true
		});

		expect(streamingMocks.clearStreamingUpdates).toHaveBeenCalledWith(
			'session-1:streaming-assistant-session-1'
		);
		expect(state.sessions.get('session-1')?.streamingMessages).toBeUndefined();
		expect(streamingMocks.scheduleStreamingUpdate).toHaveBeenLastCalledWith(
			'session-1:streaming-assistant-session-1',
			'conv-1',
			'thread-1',
			'streaming-assistant-session-1',
			'Hello world',
			'sent',
			true
		);
	});

	it('persists dedicated streamed assistant messages when requested', () => {
		const state = makeState();
		const onPersistAssistantMessage = vi.fn();
		streamingMocks.getOnPersistAssistantMessage.mockReturnValue(onPersistAssistantMessage);

		handleStreamingContent(state, {
			type: 'streaming_content',
			sessionId: 'session-1',
			conversationId: 'conv-1',
			threadId: 'thread-1',
			chunk: 'Let me inspect the repository structure.',
			isFinal: true,
			messageId: 'streaming-tool-preamble-session-1-1',
			persistMessage: true
		});

		expect(streamingMocks.scheduleStreamingUpdate).toHaveBeenCalledWith(
			'session-1:streaming-tool-preamble-session-1-1',
			'conv-1',
			'thread-1',
			'streaming-tool-preamble-session-1-1',
			'Let me inspect the repository structure.',
			'sent',
			true
		);
		expect(onPersistAssistantMessage).toHaveBeenCalledWith(
			'session-1',
			'conv-1',
			'thread-1',
			'streaming-tool-preamble-session-1-1',
			'Let me inspect the repository structure.'
		);
	});

	it('includes the session id when updating the thinking message from status phrases', async () => {
		const callbacks = await import('../callbacks');
		const onAgentActivity = vi.fn();
		vi.mocked(callbacks.getOnAgentActivity).mockReturnValue(onAgentActivity);
		const state = makeState();

		const { handleStatusPhrase } = await import('./streamingEvents');
		handleStatusPhrase(state, {
			type: 'status_phrase',
			sessionId: 'session-1',
			message: 'Reviewing the request',
			timestamp: new Date().toISOString()
		});

		expect(onAgentActivity).toHaveBeenCalledWith('conv-1', 'thread-1', {
			id: 'thinking-session-1',
			conversationId: 'conv-1',
			threadId: 'thread-1',
			role: 'assistant',
			kind: 'thinking',
			content: 'Reviewing the request',
			reasoning: 'Reviewing the request',
			timestamp: expect.any(Date),
			agentData: {
				sessionId: 'session-1'
			}
		});
	});
});
