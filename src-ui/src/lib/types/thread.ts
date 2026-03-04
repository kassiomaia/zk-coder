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
	tags?: string[];

	/** Priority level of the thread */
	priority?: ThreadPriority;

	/** Detected intent for the thread */
	intent?: string;

	/** Summary of the thread */
	summary?: string;
}

/**
 * Represents a full conversation.
 * Note: Threads are loaded separately from the backend and stored
 * in the chat store, not embedded in the conversation object.
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

	/**
	 * Threads within this conversation.
	 * @deprecated Threads are now stored separately in the chat store.
	 * Use the `threads` prop from ChatLayout or subscribe to `currentThreads` store.
	 */
	threads?: Thread[];

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
 * Matches the actual Props interface in ConversationSidebar.svelte.
 */
export interface ConversationSidebarProps {
	/** List of conversations to display */
	conversations: Conversation[];
	/** Currently selected conversation ID */
	selectedId?: string;
	/** Threads for the selected conversation (for inline rendering) */
	threadsForSelected?: Thread[];
	/** Currently selected thread ID */
	selectedThreadId?: string;
	/** Handler when a conversation is selected */
	onSelect: (conversation: Conversation) => void;
	/** Handler when a conversation is double-clicked */
	onOpenChat?: (conversation: Conversation) => void;
	/** Handler when a thread is selected */
	onSelectThread?: (thread: Thread) => void;
	/** Handler for search input */
	onSearch?: (query: string) => void;
	/** Handler to create new conversation */
	onCreateNew?: () => void;
	/** Handler to create new thread */
	onCreateThread?: () => void;
	/** Handler to delete conversation */
	onDelete?: (conversation: Conversation) => void;
	/** Handler to rename conversation */
	onRename?: (conversation: Conversation, newTitle: string) => void;
	/** Handler to archive conversation */
	onArchive?: (conversation: Conversation) => void;
	/** Handler to resolve thread */
	onResolveThread?: (thread: Thread) => void;
	/** Handler to rename thread */
	onRenameThread?: (thread: Thread, newTitle: string) => void;
	/** Handler to delete thread */
	onDeleteThread?: (thread: Thread) => void;
	/** Handler to archive thread */
	onArchiveThread?: (thread: Thread) => void;
	/** Handler for settings button click */
	onSettings?: () => void;
	/** Loading state */
	loading?: boolean;
	/** Whether any provider is configured */
	hasConfiguredProvider?: boolean;
}
