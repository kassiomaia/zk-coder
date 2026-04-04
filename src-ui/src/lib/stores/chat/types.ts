/**
 * Chat Store Type Definitions
 */

import type { Conversation, Thread } from '$lib/types/thread';
import type { Message } from '$lib/types/message';
import type { Ambiguity, IntentClassification, ResolvedAmbiguity } from '$lib/types/nlp';
import type { LoadingState } from '$lib/types/state';

// Re-export LoadingState for external consumers
export type { LoadingState } from '$lib/types/state';

// ============================================================================
// NLP State
// ============================================================================

/**
 * NLP state for the current input.
 */
export interface NlpState {
	/** Whether backend is available */
	backendAvailable: boolean;
	/** Currently detected intent */
	detectedIntent: IntentClassification | null;
	/** Detected ambiguities */
	ambiguities: Ambiguity[];
	/** Ambiguities pending user resolution (blocking input) */
	pendingAmbiguities: Ambiguity[];
	/** Map of resolved ambiguity IDs to their resolutions */
	resolvedAmbiguities: Map<string, ResolvedAmbiguity>;
	/** Whether NLP is processing */
	processing: boolean;
	/** Last error */
	error: string | null;
}

// ============================================================================
// LLM Streaming State
// ============================================================================

/**
 * LLM streaming state.
 */
export interface LlmStreamingState {
	/** Whether LLM is currently generating a response */
	isStreaming: boolean;
	/** Current request ID (if streaming) */
	requestId: string | null;
	/** Accumulated content during streaming */
	streamingContent: string;
	/** Accumulated reasoning content from models like GLM-5 */
	streamingReasoning: string;
	/** Model being used for current stream */
	model: string | null;
}

/**
 * Streaming state for a specific thread.
 */
export interface ThreadStreamingState extends LlmStreamingState {
	/** Conversation that owns this thread */
	conversationId: string;
	/** Temporary optimistic assistant message ID for this stream */
	assistantMessageId: string;
}

// ============================================================================
// Main Chat State
// ============================================================================

/**
 * Chat store state.
 */
export interface ChatState {
	/** All conversations */
	conversations: Conversation[];
	/** Loading state for conversations */
	conversationsLoading: LoadingState;
	/** Currently selected conversation ID */
	selectedConversationId: string | null;
	/** Currently selected thread ID */
	selectedThreadId: string | null;
	/** Messages for the current view */
	messages: Message[];
	/** Loading state for messages */
	messagesLoading: LoadingState;
	/** Threads for current conversation */
	threads: Thread[];
	/** Loading state for threads */
	threadsLoading: LoadingState;
	/** NLP state */
	nlp: NlpState;
	/** LLM streaming state */
	llm: LlmStreamingState;
	/** Active LLM streams by thread ID */
	threadStreaming: Record<string, ThreadStreamingState>;
	/** Thread IDs with unseen assistant responses */
	unreadThreadIds: string[];
	/** Global error message */
	error: string | null;
}

// ============================================================================
// LLM Options
// ============================================================================

/**
 * Options for LLM requests.
 */
export interface LlmRequestOptions {
	/** Model ID to use (e.g., "openai-gpt-4"). If not provided, backend uses 'code' operation default */
	modelId?: string;
	/** System prompt override */
	systemPrompt?: string;
	/** Temperature override (0-1) */
	temperature?: number;
	/** Max tokens override */
	maxTokens?: number;
}

/**
 * Context for tracking streaming to prevent cross-contamination.
 */
export interface StreamingContext {
	conversationId: string;
	threadId: string | null;
}
