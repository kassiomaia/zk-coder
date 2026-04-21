/**
 * API Core Utilities
 * Shared functions for Tauri IPC communication across all API clients.
 */

import { invoke } from '@tauri-apps/api/core';
import {
	AgentApiError,
	ApiError,
	ArtifactApiError,
	ChatApiError,
	ConfigApiError,
	NlpApiError,
	LlmApiError,
	FileApiError
} from './errors';

/**
 * Error constructor types for each API domain.
 */
type ErrorConstructor<T extends ApiError> = new (message: string, command: string, cause?: unknown) => T;

/**
 * Error class map for API domains.
 */
type ApiErrorMap = {
	chat: typeof ChatApiError;
	config: typeof ConfigApiError;
	nlp: typeof NlpApiError;
	llm: typeof LlmApiError;
	file: typeof FileApiError;
	agent: typeof AgentApiError;
	artifact: typeof ArtifactApiError;
};

/**
 * Domain identifier for API errors.
 */
export type ApiDomain = keyof ApiErrorMap;

/**
 * Wraps Tauri invoke with consistent error handling.
 *
 * @param domain - API domain (chat, config, nlp, llm, file)
 * @param command - Tauri command name
 * @param args - Optional command arguments
 * @returns Promise resolving to the command result
 */
export async function invokeCommand<T>(
	domain: ApiDomain,
	command: string,
	args?: Record<string, unknown>
): Promise<T> {
	try {
		return await invoke<T>(command, args);
	} catch (error) {
		const ErrorClass = getErrorConstructor(domain);
		throw new ErrorClass(
			`${domain} command '${command}' failed: ${error instanceof Error ? error.message : String(error)}`,
			command,
			error
		);
	}
}

/**
 * Gets the error constructor for a domain.
 */
function getErrorConstructor(domain: ApiDomain): ApiErrorMap[typeof domain] {
	const constructors: ApiErrorMap = {
		chat: ChatApiError,
		config: ConfigApiError,
		nlp: NlpApiError,
		llm: LlmApiError,
		file: FileApiError,
		agent: AgentApiError,
		artifact: ArtifactApiError
	};
	return constructors[domain];
}

/**
 * Convenience wrapper for single domain usage.
 * Creates a typed invoke function for a specific API domain.
 *
 * @param domain - API domain identifier
 * @returns Function that wraps invoke with domain-specific error handling
 *
 * @example
 * ```typescript
 * const chatInvoke = createInvoker('chat');
 * const conversations = await chatInvoke<ConversationResponse[]>('load_conversations');
 * ```
 */
export function createInvoker<T>(domain: ApiDomain) {
	return (command: string, args?: Record<string, unknown>): Promise<T> =>
		invokeCommand<T>(domain, command, args);
}