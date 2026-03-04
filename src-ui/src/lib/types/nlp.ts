/**
 * NLP Types
 * Type definitions for the NLP backend integration.
 * These types match the Rust backend types in src-tauri/src/nlp/
 */

// ============================================================================
// Intent Types (from intent.rs)
// ============================================================================

/**
 * Developer command intents supported by the system.
 */
export type Intent =
	| 'generate_code'
	| 'generate_function'
	| 'generate_class'
	| 'generate_test'
	| 'refactor_code'
	| 'optimize_code'
	| 'explain_code'
	| 'analyze_code'
	| 'find_bugs'
	| 'document_code'
	| 'find_file'
	| 'find_symbol'
	| 'goto_definition'
	| 'find_references'
	| 'run_tests'
	| 'build_project'
	| 'format_code'
	| 'lint_code'
	| 'git_status'
	| 'git_commit'
	| 'git_diff'
	| 'git_log'
	| 'start_new_thread'
	| 'switch_context'
	| 'clear_history'
	| 'unknown';

/**
 * Human-readable labels for intents.
 */
export const INTENT_LABELS: Record<Intent, string> = {
	generate_code: 'Generate Code',
	generate_function: 'Generate Function',
	generate_class: 'Generate Class',
	generate_test: 'Generate Test',
	refactor_code: 'Refactor',
	optimize_code: 'Optimize',
	explain_code: 'Explain',
	analyze_code: 'Analyze',
	find_bugs: 'Find Bugs',
	document_code: 'Document',
	find_file: 'Find File',
	find_symbol: 'Find Symbol',
	goto_definition: 'Go to Definition',
	find_references: 'Find References',
	run_tests: 'Run Tests',
	build_project: 'Build',
	format_code: 'Format',
	lint_code: 'Lint',
	git_status: 'Git Status',
	git_commit: 'Git Commit',
	git_diff: 'Git Diff',
	git_log: 'Git Log',
	start_new_thread: 'New Thread',
	switch_context: 'Switch Context',
	clear_history: 'Clear History',
	unknown: 'Unknown'
};

/**
 * Result of intent classification with confidence score.
 */
export interface IntentClassification {
	/** The primary detected intent */
	intent: Intent;
	/** Confidence score (0.0 to 1.0) */
	confidence: number;
	/** Alternative intents with their confidence scores */
	alternative_intents: Array<[Intent, number]>;
}

/**
 * Extracted parameters from user input.
 */
export interface IntentParams {
	/** Programming language mentioned */
	language?: string;
	/** File path mentioned */
	file_path?: string;
	/** Symbol name mentioned */
	symbol_name?: string;
	/** Code snippet provided */
	code_snippet?: string;
	/** Natural language description */
	description?: string;
}

// ============================================================================
// Context Types (from context.rs)
// ============================================================================

/**
 * Message role in a conversation.
 */
export type MessageRole = 'user' | 'assistant' | 'system';

/**
 * A single message in the context.
 */
export interface ContextMessage {
	/** Unique identifier */
	id: string;
	/** Role (user, assistant, system) */
	role: MessageRole;
	/** Message content */
	content: string;
	/** ISO timestamp */
	timestamp: string;
	/** Token count (if known) */
	tokens?: number;
}

/**
 * Kind of code symbol.
 */
export type SymbolKind =
	| 'function'
	| 'class'
	| 'variable'
	| 'interface'
	| 'constant'
	| 'type_alias';

/**
 * Reference to a code symbol.
 */
export interface SymbolReference {
	/** Symbol name */
	name: string;
	/** Symbol kind */
	kind: SymbolKind;
	/** File path containing the symbol */
	file_path: string;
	/** Line number */
	line: number;
}

/**
 * Information about a file in context.
 */
export interface FileInfo {
	/** File path */
	path: string;
	/** Programming language */
	language: string;
}

/**
 * Metadata tracking current context state.
 */
export interface ContextMetadata {
	/** Currently active file */
	current_file?: FileInfo;
	/** Active symbol references */
	active_symbols: SymbolReference[];
	/** Compressed context summary for older messages */
	summary?: string;
}

/**
 * Simplified context for AI prompts.
 */
export interface ConversationContext {
	/** Recent messages (last N) */
	recent_messages: ContextMessage[];
	/** Context metadata */
	metadata: ContextMetadata;
	/** Summary of older context */
	summary?: string;
}

// ============================================================================
// Multi-Intent Types (from multi_intent.rs)
// ============================================================================

/**
 * A span of text in the original input.
 */
export interface TextSpan {
	/** Start character position */
	start: number;
	/** End character position */
	end: number;
	/** The text at this span */
	text: string;
}

/**
 * A detected intent with its position in the input.
 */
export interface DetectedIntent {
	/** The detected intent */
	intent: Intent;
	/** Extracted parameters */
	params: IntentParams;
	/** Position in the original text */
	span: TextSpan;
	/** Confidence score (0.0 to 1.0) */
	confidence: number;
}

/**
 * Relationships between detected intents.
 */
export type IntentRelationship = 'sequential' | 'parallel' | 'dependent' | 'exclusive';

/**
 * Labels for intent relationships.
 */
export const INTENT_RELATIONSHIP_LABELS: Record<IntentRelationship, string> = {
	sequential: 'then',
	parallel: 'and',
	dependent: 'using',
	exclusive: 'or'
};

/**
 * Result of multi-intent detection.
 */
export interface MultiIntentResult {
	/** Detected intents */
	intents: DetectedIntent[];
	/** Relationships between intents */
	relationships: IntentRelationship[];
	/** Suggested execution order (indices into intents) */
	execution_order: number[];
}

// ============================================================================
// Ambiguity Types (from ambiguity.rs)
// ============================================================================

/**
 * Types of ambiguities that can be detected.
 */
export type AmbiguityType =
	| 'missing_file'
	| 'missing_symbol'
	| 'ambiguous_reference'
	| 'multiple_matches'
	| 'unclear_scope'
	| 'missing_parameter';

/**
 * Labels for ambiguity types.
 */
export const AMBIGUITY_TYPE_LABELS: Record<AmbiguityType, string> = {
	missing_file: 'Missing File',
	missing_symbol: 'Missing Symbol',
	ambiguous_reference: 'Ambiguous Reference',
	multiple_matches: 'Multiple Matches',
	unclear_scope: 'Unclear Scope',
	missing_parameter: 'Missing Parameter'
};

/**
 * Severity of an ambiguity.
 */
export type AmbiguitySeverity = 'low' | 'medium' | 'high';

/**
 * An option in a clarification request.
 */
export interface ClarificationOption {
	/** Value to return if selected */
	value: string;
	/** Display label */
	label: string;
	/** Optional description */
	description?: string;
}

/**
 * Request for clarification from the user.
 */
export interface ClarificationRequest {
	/** Question to ask the user */
	question: string;
	/** Predefined options */
	options: ClarificationOption[];
	/** Whether freeform input is allowed */
	allows_freeform: boolean;
}

/**
 * A suggested resolution for an ambiguity.
 */
export interface AmbiguitySuggestion {
	/** Suggested value */
	value: string;
	/** Confidence in this suggestion */
	confidence: number;
	/** Reason for the suggestion */
	reason: string;
}

/**
 * A detected ambiguity in user input.
 */
export interface Ambiguity {
	/** Unique identifier for this ambiguity */
	id: string;
	/** Type of ambiguity */
	ambiguity_type: AmbiguityType;
	/** Location in the input */
	span: TextSpan;
	/** Severity level */
	severity: AmbiguitySeverity;
	/** Request for clarification */
	clarification: ClarificationRequest;
	/** Suggested resolutions */
	suggestions: AmbiguitySuggestion[];
}

// ============================================================================
// Resolution Types (for ambiguity resolution)
// ============================================================================

/**
 * Type of resolution applied to an ambiguity.
 */
export type ResolutionType = 'selected' | 'custom' | 'dismissed';

/**
 * Labels for resolution types.
 */
export const RESOLUTION_TYPE_LABELS: Record<ResolutionType, string> = {
	selected: 'Selected',
	custom: 'Custom',
	dismissed: 'Dismissed'
};

/**
 * A resolved ambiguity with the chosen resolution.
 */
export interface ResolvedAmbiguity {
	/** ID of the original ambiguity */
	id: string;
	/** The original ambiguous text */
	original_text: string;
	/** The resolution value (selected option or custom input) */
	resolution: string;
	/** How the resolution was made */
	resolution_type: ResolutionType;
}

// ============================================================================
// Code Understanding Types (from code_understanding.rs)
// ============================================================================

/**
 * Supported programming languages.
 */
export type ProgrammingLanguage = 'rust' | 'typescript' | 'javascript' | 'python' | 'unknown';

/**
 * File extensions for programming languages.
 */
export const PROGRAMMING_LANGUAGE_EXTENSIONS: Record<ProgrammingLanguage, string> = {
	rust: 'rs',
	typescript: 'ts',
	javascript: 'js',
	python: 'py',
	unknown: ''
};

/**
 * Human-readable names for programming languages.
 */
export const PROGRAMMING_LANGUAGE_NAMES: Record<ProgrammingLanguage, string> = {
	rust: 'Rust',
	typescript: 'TypeScript',
	javascript: 'JavaScript',
	python: 'Python',
	unknown: 'Unknown'
};

/**
 * Information about a function.
 */
export interface FunctionInfo {
	/** Function name */
	name: string;
	/** Parameter names (simplified) */
	parameters: string[];
	/** Return type if known */
	return_type?: string;
	/** Start line (0-indexed) */
	line_start: number;
	/** End line (0-indexed) */
	line_end: number;
}

/**
 * Information about a class/struct.
 */
export interface ClassInfo {
	/** Class name */
	name: string;
	/** Methods */
	methods: FunctionInfo[];
	/** Start line (0-indexed) */
	line_start: number;
	/** End line (0-indexed) */
	line_end: number;
}

/**
 * Structure of code.
 */
export interface CodeStructure {
	/** Functions found */
	functions: FunctionInfo[];
	/** Classes/structs found */
	classes: ClassInfo[];
	/** Import statements */
	imports: string[];
}

/**
 * Complexity rating levels.
 */
export type ComplexityRating = 'low' | 'medium' | 'high' | 'very_high';

/**
 * Labels for complexity ratings.
 */
export const COMPLEXITY_RATING_LABELS: Record<ComplexityRating, string> = {
	low: 'Low',
	medium: 'Medium',
	high: 'High',
	very_high: 'Very High'
};

/**
 * Complexity metrics for code.
 */
export interface ComplexityMetrics {
	/** Cyclomatic complexity */
	cyclomatic: number;
	/** Lines of code */
	lines: number;
	/** Maximum nesting depth */
	nesting: number;
}

/**
 * Information extracted from code analysis.
 */
export interface CodeUnderstanding {
	/** Detected language */
	language: ProgrammingLanguage;
	/** Code structure */
	structure: CodeStructure;
	/** Complexity metrics */
	complexity: ComplexityMetrics;
}

// ============================================================================
// Utility Functions
// ============================================================================

/**
 * Gets the label for an intent.
 */
export function getIntentLabel(intent: Intent): string {
	return INTENT_LABELS[intent] || intent;
}

/**
 * Gets the label for an ambiguity type.
 */
export function getAmbiguityTypeLabel(type: AmbiguityType): string {
	return AMBIGUITY_TYPE_LABELS[type] || type;
}

/**
 * Gets the label for a complexity rating.
 */
export function getComplexityRatingLabel(rating: ComplexityRating): string {
	return COMPLEXITY_RATING_LABELS[rating] || rating;
}

/**
 * Detects programming language from file extension.
 */
export function detectLanguageFromExtension(ext: string): ProgrammingLanguage {
	const normalized = ext.toLowerCase().replace(/^\./, '');
	switch (normalized) {
		case 'rs':
			return 'rust';
		case 'ts':
		case 'tsx':
			return 'typescript';
		case 'js':
		case 'jsx':
		case 'mjs':
		case 'cjs':
			return 'javascript';
		case 'py':
		case 'pyw':
			return 'python';
		default:
			return 'unknown';
	}
}
