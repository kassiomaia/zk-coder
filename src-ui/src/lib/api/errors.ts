/**
 * API Error Classes
 * Consolidated error types for all API clients.
 */

export class ApiError extends Error {
	constructor(
		message: string,
		public readonly command: string,
		public readonly cause?: unknown
	) {
		super(message);
		this.name = 'ApiError';
	}

	toString(): string {
		return `${this.name}: ${this.message}`;
	}
}

export class ChatApiError extends ApiError {
	constructor(message: string, command: string, cause?: unknown) {
		super(message, command, cause);
		this.name = 'ChatApiError';
	}
}

export class ConfigApiError extends ApiError {
	constructor(message: string, command: string, cause?: unknown) {
		super(message, command, cause);
		this.name = 'ConfigApiError';
	}
}

export class NlpApiError extends ApiError {
	constructor(message: string, command: string, cause?: unknown) {
		super(message, command, cause);
		this.name = 'NlpApiError';
	}
}

export class LlmApiError extends ApiError {
	constructor(message: string, command: string, cause?: unknown) {
		super(message, command, cause);
		this.name = 'LlmApiError';
	}
}

export class FileApiError extends ApiError {
	constructor(message: string, command: string, cause?: unknown) {
		super(message, command, cause);
		this.name = 'FileApiError';
	}
}