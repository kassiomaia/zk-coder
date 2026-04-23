/**
 * Type Exports
 * Central export point for all type definitions.
 */

// Message types
export type {
	Message,
	MessageRole as MessageRoleType,
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
	ConversationSidebarProps
} from './thread.js';

// Command types
export type { Command, CommandPaletteProps } from './command.js';
export {
	CommandCategory,
	filterCommands,
	groupCommandsByCategory,
	getCategoryLabel
} from './command.js';

// NLP types
export type {
	Intent,
	IntentClassification,
	IntentParams,
	MessageRole,
	ContextMessage,
	SymbolKind,
	SymbolReference,
	FileInfo,
	ContextMetadata,
	ConversationContext,
	TextSpan,
	DetectedIntent,
	IntentRelationship,
	MultiIntentResult,
	AmbiguityType,
	AmbiguitySeverity,
	ClarificationOption,
	ClarificationRequest,
	AmbiguitySuggestion,
	Ambiguity,
	ProgrammingLanguage,
	FunctionInfo,
	ClassInfo,
	CodeStructure,
	ComplexityRating,
	ComplexityMetrics,
	CodeUnderstanding
} from './nlp.js';

// Common state types
export type { LoadingState } from './state.js';

// Agent types
export type {
	SessionId,
	AgentId,
	TrustMode,
	TaskStatus,
	PermissionLevel,
	Agent,
	AgentSession,
	SessionState,
	AgentTask,
	TouchedFile,
	TerminalLine,
	PendingAction,
	ActionType,
	ActionPreview,
	AgentEvent,
	ExecutionStep,
	StepAction,
	StepResult,
	ExecutionResult
} from './agent.js';

// NLP constants and utilities
export {
	INTENT_LABELS,
	INTENT_RELATIONSHIP_LABELS,
	AMBIGUITY_TYPE_LABELS,
	PROGRAMMING_LANGUAGE_EXTENSIONS,
	PROGRAMMING_LANGUAGE_NAMES,
	COMPLEXITY_RATING_LABELS,
	getIntentLabel,
	getAmbiguityTypeLabel,
	getComplexityRatingLabel,
	detectLanguageFromExtension
} from './nlp.js';
