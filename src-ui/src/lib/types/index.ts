/**
 * Type Exports
 * Central export point for all type definitions.
 */

// Message types
export type {
	Message,
	MessageRole,
	MessageStatus,
	MessageMetadata,
	Attachment,
	CodeReference,
	MessageGroup,
	MessageDisplayProps
} from './message.js';

// Thread types
export type {
	Thread,
	ThreadPriority,
	ThreadMetadata,
	Conversation,
	ConversationSettings,
	ConversationGroup,
	ThreadViewProps,
	ThreadItemProps,
	ConversationSidebarProps
} from './thread.js';

// Command types
export type { Command, CommandCategory, CommandPaletteProps } from './command.js';
export {
	BUILT_IN_COMMANDS,
	filterCommands,
	groupCommandsByCategory,
	getCategoryLabel
} from './command.js';
