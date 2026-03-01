/**
 * Thread Types
 * Type definitions for conversation threads in the Coder AI application.
 */

/**
 * Priority level for a thread.
 */
export type ThreadPriority = 'low' | 'medium' | 'high';

/**
 * Represents a thread within a conversation.
 * Threads allow organizing related messages together.
 */
export interface Thread {
	/** Unique identifier for the thread */
	id: string;

	/** ID of the parent conversation */
	conversationId: string;

	/** ID of the parent thread (for nested threads) */
	parentId: string | null;

	/** Title/topic of the thread */
	title: string;

	/** When the thread was created */
	createdAt: Date;

	/** When the thread was last updated */
	updatedAt: Date;

	/** Number of messages in the thread */
	messageCount: number;

	/** Additional metadata about the thread */
	metadata: ThreadMetadata;
}

/**
 * Metadata for a thread.
 */
export interface ThreadMetadata {
	/** Whether the thread has been resolved */
	resolved: boolean;

	/** Tags associated with the thread */
	tags: string[];

	/** Priority level of the thread */
	priority: ThreadPriority;

	/** Detected intent for the thread */
	intent?: string;
}

/**
 * Represents a full conversation with its threads.
 */
export interface Conversation {
	/** Unique identifier for the conversation */
	id: string;

	/** Title of the conversation */
	title: string;

	/** When the conversation was created */
	createdAt: Date;

	/** When the conversation was last updated */
	updatedAt: Date;

	/** Threads within this conversation */
	threads: Thread[];

	/** Settings for this conversation */
	settings: ConversationSettings;
}

/**
 * Settings for a conversation.
 */
export interface ConversationSettings {
	/** AI model to use */
	model: string;

	/** Temperature for response generation */
	temperature: number;

	/** Maximum tokens for responses */
	maxTokens: number;

	/** Custom system prompt */
	systemPrompt?: string;
}

/**
 * Group of conversations for display purposes.
 * Conversations are grouped by time (Today, Yesterday, etc.).
 */
export interface ConversationGroup {
	/** Label for the group */
	label: string;

	/** Conversations in this group */
	conversations: Conversation[];
}

/**
 * Props for the ThreadView component.
 */
export interface ThreadViewProps {
	/** ID of the current conversation */
	conversationId: string;

	/** All threads in the conversation */
	threads: Thread[];

	/** ID of the currently active/selected thread */
	activeThreadId: string | null;

	/** Handler when a thread is selected */
	onThreadSelect: (id: string) => void;

	/** Handler to create a new thread */
	onNewThread: () => void;
}

/**
 * Props for the ThreadItem component.
 */
export interface ThreadItemProps {
	/** The thread to display */
	thread: Thread;

	/** Whether this thread is currently active */
	isActive: boolean;

	/** Click handler */
	onclick: () => void;
}

/**
 * Props for the ConversationSidebar component.
 */
export interface ConversationSidebarProps {
	/** All conversations */
	conversations: Conversation[];

	/** ID of the currently active conversation */
	activeConversationId: string | null;

	/** Handler when a conversation is selected */
	onConversationSelect: (id: string) => void;

	/** Handler to create a new conversation */
	onNewConversation: () => void;

	/** Handler to delete a conversation */
	onDeleteConversation: (id: string) => void;

	/** Handler to rename a conversation */
	onRenameConversation: (id: string, name: string) => void;
}
