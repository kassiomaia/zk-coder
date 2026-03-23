/**
 * Message Types
 * Type definitions for chat messages in the Coder AI application.
 */

/** Role of the message sender */
export type MessageRole = 'user' | 'assistant' | 'system';

/** Status of a message */
export type MessageStatus = 'sending' | 'sent' | 'delivered' | 'error';

/** Feedback type for a message (like/dislike) */
export type MessageFeedback = 'like' | 'dislike';

/** Kind of message for specialized rendering */
export type MessageKind = 'normal' | 'thinking' | 'tool_call' | 'terminal';

/**
 * Agent-specific data for messages that represent agent activity.
 */
export interface AgentMessageData {
	/** Tool name (for tool_call kind) */
	toolName?: string;
	/** Tool arguments (for tool_call kind) */
	toolArgs?: Record<string, unknown>;
	/** Tool result (for tool_call kind) */
	toolResult?: unknown;
	/** Whether the tool call resulted in an error */
	toolError?: boolean;
	/** Streaming content (for terminal/tool_call kinds) */
	streamingContent?: string;
	/** Stream type (for terminal kind) */
	streamType?: 'stdout' | 'stderr' | 'command';
	/** Session ID for agent messages */
	sessionId?: string;
	/** Whether the message is expanded (for tool_call) */
	expanded?: boolean;
	/** Exit code for finished commands */
	exitCode?: number | null;
	/** Tool call status: 'pending' | 'success' | 'error' */
	toolCallStatus?: 'pending' | 'success' | 'error';
}

/**
 * Represents a single chat message.
 */
export interface Message {
	/** Unique identifier for the message */
	id: string;

	/** ID of the conversation this message belongs to */
	conversationId: string;

	/** ID of the thread this message belongs to (required - every message must be in a thread) */
	threadId: string;

	/** Role of the message sender */
	role: MessageRole;

	/** Message content (can be plain text or HTML from markdown) */
	content: string;

	/** When the message was sent */
	timestamp: Date;

	/** Current status of the message */
	status?: MessageStatus;

	/** Kind of message for specialized rendering (default: 'normal') */
	kind?: MessageKind;

	/** Agent-specific data for messages that represent agent activity */
	agentData?: AgentMessageData;

	/** Whether this message should be auto-removed (e.g., transient status updates) */
	isTransient?: boolean;

	/** Optional metadata about the message */
	metadata?: MessageMetadata;
}

/**
 * Additional metadata for a message.
 */
export interface MessageMetadata {
	/** Detected intent from NLP */
	intent?: string;

	/** Confidence score of intent detection (0-1) */
	confidence?: number;

	/** Token count for the message */
	tokens?: number;

	/** AI model used for generation */
	model?: string;

	/** User feedback on the message (like/dislike) */
	feedback?: MessageFeedback;

	/** Files attached to the message */
	attachments?: Attachment[];

	/** Code references mentioned in the message */
	codeReferences?: CodeReference[];
}

/**
 * File attachment for a message.
 */
export interface Attachment {
	/** Unique identifier for the attachment */
	id: string;

	/** Original filename */
	name: string;

	/** MIME type of the file */
	type: string;

	/** File size in bytes */
	size: number;

	/** URL to access the file (if stored) */
	url?: string;

	/** Preview image URL (for images) */
	preview?: string;
}

/**
 * Reference to a code section.
 */
export interface CodeReference {
	/** Path to the file containing the code */
	filePath: string;

	/** Starting line number (1-indexed) */
	lineStart: number;

	/** Ending line number (optional, for multi-line references) */
	lineEnd?: number;

	/** Programming language of the code */
	language: string;
}

/**
 * Group of messages for display purposes.
 * Messages are grouped by time and role.
 */
export interface MessageGroup {
	/** Messages in this group */
	messages: Message[];

	/** Optional label for the group (e.g., "Today", "Yesterday") */
	label?: string;
}

/**
 * Props for components that display messages.
 */
export interface MessageDisplayProps {
	/** The message to display */
	message: Message;

	/** Whether the message is being hovered */
	isHovered?: boolean;

	/** Handler for editing the message */
	onEdit?: () => void;

	/** Handler for deleting the message */
	onDelete?: () => void;

	/** Handler for retrying a failed message */
	onRetry?: () => void;
}
