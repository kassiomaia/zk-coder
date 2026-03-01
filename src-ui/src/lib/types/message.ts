/**
 * Message Types
 * Type definitions for chat messages in the Coder AI application.
 */

/** Role of the message sender */
export type MessageRole = 'user' | 'assistant' | 'system';

/** Status of a message */
export type MessageStatus = 'sending' | 'sent' | 'delivered' | 'error';

/**
 * Represents a single chat message.
 */
export interface Message {
	/** Unique identifier for the message */
	id: string;

	/** ID of the conversation this message belongs to */
	conversationId: string;

	/** ID of the thread this message belongs to (if part of a thread) */
	threadId?: string;

	/** Role of the message sender */
	role: MessageRole;

	/** Message content (can be plain text or HTML from markdown) */
	content: string;

	/** When the message was sent */
	timestamp: Date;

	/** Current status of the message */
	status?: MessageStatus;

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
