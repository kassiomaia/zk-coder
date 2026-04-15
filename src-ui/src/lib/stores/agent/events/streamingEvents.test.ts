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

	it('accumulates non-final chunks sharing the same messageId and schedules throttled updates', () => {
		const state = makeState();
		const messageId = 'stream-final-result-session-1-task-1-no-subtask-s4';

		handleStreamingContent(state, {
			type: 'streaming_content',
			sessionId: 'session-1',
			conversationId: 'conv-1',
			threadId: 'thread-1',
			chunk: 'Hello',
			isFinal: false,
			messageId
		});

		handleStreamingContent(state, {
			type: 'streaming_content',
			sessionId: 'session-1',
			conversationId: 'conv-1',
			threadId: 'thread-1',
			chunk: ' world',
			isFinal: false,
			messageId
		});

		expect(state.sessions.get('session-1')?.streamingMessages).toEqual({
			[messageId]: 'Hello world'
		});
		expect(streamingMocks.scheduleStreamingUpdate).toHaveBeenLastCalledWith(
			`session-1:${messageId}`,
			'conv-1',
			'thread-1',
			messageId,
			'Hello world',
			'sending'
		);
	});

	it('flushes the full accumulated content immediately on the final chunk', () => {
		const state = makeState();
		const messageId = 'stream-final-result-session-1-task-1-no-subtask-s4';

		handleStreamingContent(state, {
			type: 'streaming_content',
			sessionId: 'session-1',
			conversationId: 'conv-1',
			threadId: 'thread-1',
			chunk: 'Hello',
			isFinal: false,
			messageId
		});

		handleStreamingContent(state, {
			type: 'streaming_content',
			sessionId: 'session-1',
			conversationId: 'conv-1',
			threadId: 'thread-1',
			chunk: ' world',
			isFinal: true,
			messageId
		});

		expect(streamingMocks.clearStreamingUpdates).toHaveBeenCalledWith(
			`session-1:${messageId}`
		);
		expect(state.sessions.get('session-1')?.streamingMessages).toBeUndefined();
		expect(streamingMocks.scheduleStreamingUpdate).toHaveBeenLastCalledWith(
			`session-1:${messageId}`,
			'conv-1',
			'thread-1',
			messageId,
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

	// Regression guard for the "subtask summary vanishes on reload" bug. The
	// backend emits the subtask-result stream with persistMessage: true once the
	// bug is fixed; this test pins the frontend side of the contract so a future
	// refactor cannot silently drop the callback invocation for that path.
	it('persists subtask-result summaries emitted by the backend', () => {
		const state = makeState();
		const onPersistAssistantMessage = vi.fn();
		streamingMocks.getOnPersistAssistantMessage.mockReturnValue(onPersistAssistantMessage);

		const messageId = 'stream-subtask-result-session-1-task-1-sub-investigate-s3';

		handleStreamingContent(state, {
			type: 'streaming_content',
			sessionId: 'session-1',
			conversationId: 'conv-1',
			threadId: 'thread-1',
			chunk: 'Subtask complete: found the offending module.',
			isFinal: true,
			messageId,
			persistMessage: true
		});

		expect(onPersistAssistantMessage).toHaveBeenCalledWith(
			'session-1',
			'conv-1',
			'thread-1',
			messageId,
			'Subtask complete: found the offending module.'
		);
	});

	// Negative regression guard: when persistMessage is absent the callback must
	// NOT fire. This is what distinguishes transient UI fillers (the "Continuing
	// with next subtask..." message) from real persisted summaries.
	it('does not persist final chunks that lack the persistMessage flag', () => {
		const state = makeState();
		const onPersistAssistantMessage = vi.fn();
		streamingMocks.getOnPersistAssistantMessage.mockReturnValue(onPersistAssistantMessage);

		handleStreamingContent(state, {
			type: 'streaming_content',
			sessionId: 'session-1',
			conversationId: 'conv-1',
			threadId: 'thread-1',
			chunk: '\n\nContinuing with next subtask...\n',
			isFinal: true,
			messageId: 'stream-subtask-continuation-session-1-task-1-sub-A-s5'
		});

		expect(onPersistAssistantMessage).not.toHaveBeenCalled();
	});

	it('keeps two concurrent streams in the same session under distinct keys when messageIds are distinct', () => {
		const state = makeState();

		handleStreamingContent(state, {
			type: 'streaming_content',
			sessionId: 'session-1',
			conversationId: 'conv-1',
			threadId: 'thread-1',
			chunk: 'first subtask output',
			isFinal: false,
			messageId: 'stream-subtask-result-session-1-task-1-sub-A-s3'
		});

		handleStreamingContent(state, {
			type: 'streaming_content',
			sessionId: 'session-1',
			conversationId: 'conv-1',
			threadId: 'thread-1',
			chunk: 'second subtask output',
			isFinal: false,
			messageId: 'stream-subtask-result-session-1-task-1-sub-B-s7'
		});

		const streamingMessages = state.sessions.get('session-1')?.streamingMessages;
		expect(streamingMessages).toEqual({
			'stream-subtask-result-session-1-task-1-sub-A-s3': 'first subtask output',
			'stream-subtask-result-session-1-task-1-sub-B-s7': 'second subtask output'
		});
	});

	it('does not collapse two streams with missing messageId into the same session-wide key', () => {
		const state = makeState();

		handleStreamingContent(state, {
			type: 'streaming_content',
			sessionId: 'session-1',
			conversationId: 'conv-1',
			threadId: 'thread-1',
			chunk: 'subtask one result',
			isFinal: true
		});

		handleStreamingContent(state, {
			type: 'streaming_content',
			sessionId: 'session-1',
			conversationId: 'conv-1',
			threadId: 'thread-1',
			chunk: 'subtask two result',
			isFinal: true
		});

		const flushCalls = streamingMocks.scheduleStreamingUpdate.mock.calls.filter(
			(call: unknown[]) => call[5] === 'sent'
		);
		expect(flushCalls.length).toBe(2);

		const firstKey = flushCalls[0][0] as string;
		const secondKey = flushCalls[1][0] as string;
		expect(firstKey).not.toEqual(secondKey);
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
