/**
 * NLP API Client
 * TypeScript wrappers for Tauri IPC commands to the NLP backend.
 */

import { invoke } from '@tauri-apps/api/core';
import type {
	IntentClassification,
	ConversationContext,
	MultiIntentResult,
	Ambiguity,
	CodeUnderstanding,
	ProgrammingLanguage
} from '$lib/types/nlp';

// ============================================================================
// Error Handling
// ============================================================================

/**
 * Error class for NLP API errors.
 */
export class NlpApiError extends Error {
	constructor(
		message: string,
		public readonly command: string,
		public readonly cause?: unknown
	) {
		super(message);
		this.name = 'NlpApiError';
	}
}

/**
 * Wraps invoke with error handling.
 */
async function safeInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
	try {
		return await invoke<T>(command, args);
	} catch (error) {
		throw new NlpApiError(
			`NLP command '${command}' failed: ${error instanceof Error ? error.message : String(error)}`,
			command,
			error
		);
	}
}

// ============================================================================
// Intent Classification
// ============================================================================

/**
 * Options for classifying intent.
 */
export interface ClassifyIntentOptions {
	/** User input text */
	input: string;
	/** Conversation ID for context */
	conversationId: string;
}

/**
 * Classifies intent from user input.
 *
 * @param options - Classification options
 * @returns Intent classification result
 */
export async function classifyIntent(
	options: ClassifyIntentOptions
): Promise<IntentClassification> {
	return safeInvoke<IntentClassification>('classify_intent', {
		input: options.input,
		conversation_id: options.conversationId
	});
}

// ============================================================================
// Context Management
// ============================================================================

/**
 * Gets context for a conversation.
 *
 * @param conversationId - The conversation ID
 * @returns The conversation context
 */
export async function getConversationContext(conversationId: string): Promise<ConversationContext> {
	return safeInvoke<ConversationContext>('get_conversation_context', {
		conversation_id: conversationId
	});
}

/**
 * Options for adding a context message.
 */
export interface AddContextMessageOptions {
	/** Conversation ID */
	conversationId: string;
	/** Message role (user, assistant, system) */
	role: 'user' | 'assistant' | 'system';
	/** Message content */
	content: string;
}

/**
 * Adds a message to conversation context.
 *
 * @param options - Message options
 */
export async function addContextMessage(options: AddContextMessageOptions): Promise<void> {
	return safeInvoke<void>('add_context_message', {
		conversation_id: options.conversationId,
		role: options.role,
		content: options.content
	});
}

/**
 * Clears a conversation context.
 *
 * @param conversationId - The conversation ID to clear
 */
export async function clearConversationContext(conversationId: string): Promise<void> {
	return safeInvoke<void>('clear_conversation_context', {
		conversation_id: conversationId
	});
}

// ============================================================================
// Multi-Intent Detection
// ============================================================================

/**
 * Options for detecting multiple intents.
 */
export interface DetectMultiIntentOptions {
	/** User input text */
	input: string;
	/** Conversation ID for context */
	conversationId: string;
}

/**
 * Detects multiple intents in input.
 *
 * @param options - Detection options
 * @returns Multi-intent detection result
 */
export async function detectMultiIntent(
	options: DetectMultiIntentOptions
): Promise<MultiIntentResult> {
	return safeInvoke<MultiIntentResult>('detect_multi_intent', {
		input: options.input,
		conversation_id: options.conversationId
	});
}

// ============================================================================
// Ambiguity Detection
// ============================================================================

/**
 * Options for detecting ambiguities.
 */
export interface DetectAmbiguitiesOptions {
	/** User input text */
	input: string;
	/** Conversation ID for context */
	conversationId: string;
}

/**
 * Detects ambiguities in input.
 *
 * @param options - Detection options
 * @returns Array of detected ambiguities
 */
export async function detectAmbiguities(options: DetectAmbiguitiesOptions): Promise<Ambiguity[]> {
	return safeInvoke<Ambiguity[]>('detect_ambiguities', {
		input: options.input,
		conversation_id: options.conversationId
	});
}

// ============================================================================
// Code Analysis
// ============================================================================

/**
 * Options for analyzing code.
 */
export interface AnalyzeCodeOptions {
	/** Code to analyze */
	code: string;
	/** Programming language */
	language: ProgrammingLanguage;
}

/**
 * Analyzes code for understanding.
 *
 * @param options - Analysis options
 * @returns Code understanding result
 */
export async function analyzeCode(options: AnalyzeCodeOptions): Promise<CodeUnderstanding> {
	return safeInvoke<CodeUnderstanding>('analyze_code', {
		code: options.code,
		language: options.language
	});
}

// ============================================================================
// Project Files
// ============================================================================

/**
 * Updates project files for ambiguity resolution.
 *
 * @param files - Array of file paths
 */
export async function updateProjectFiles(files: string[]): Promise<void> {
	return safeInvoke<void>('update_project_files', { files });
}

// ============================================================================
// Convenience Functions
// ============================================================================

/**
 * Classifies intent with debounce helper.
 * Returns a function that can be called multiple times but only
 * executes after a delay.
 */
export function createDebouncedClassifier(delayMs: number = 300): {
	classify: (options: ClassifyIntentOptions) => Promise<IntentClassification | null>;
	cancel: () => void;
} {
	let timeoutId: ReturnType<typeof setTimeout> | null = null;
	let latestOptions: ClassifyIntentOptions | null = null;
	let resolveLatest: ((value: IntentClassification | null) => void) | null = null;

	return {
		classify: (options: ClassifyIntentOptions) => {
			latestOptions = options;

			// Cancel previous timeout
			if (timeoutId) {
				clearTimeout(timeoutId);
			}

			return new Promise((resolve) => {
				resolveLatest = resolve;
				timeoutId = setTimeout(async () => {
					if (latestOptions) {
						try {
							const result = await classifyIntent(latestOptions);
							resolve(result);
						} catch {
							resolve(null);
						}
					}
					timeoutId = null;
				}, delayMs);
			});
		},
		cancel: () => {
			if (timeoutId) {
				clearTimeout(timeoutId);
				timeoutId = null;
			}
			if (resolveLatest) {
				resolveLatest(null);
				resolveLatest = null;
			}
		}
	};
}

/**
 * Checks if the Tauri backend is available.
 * Useful for development mode when running in browser.
 */
export async function isBackendAvailable(): Promise<boolean> {
	try {
		// Try a simple command to check if backend is available
		await invoke('get_tauri_version');
		return true;
	} catch {
		return false;
	}
}
