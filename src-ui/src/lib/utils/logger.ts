/**
 * Secure Logger Utility
 *
 * A logging utility that automatically redacts sensitive information
 * to prevent API keys, tokens, and other secrets from appearing in logs.
 *
 * In production mode, all logging is disabled to prevent:
 * - Performance overhead from string formatting
 * - Accidental exposure of sensitive data
 * - Console clutter in production
 */

import { dev } from '$app/environment';

/**
 * Whether debug logging is enabled.
 * Only true in development mode.
 */
const isDebugEnabled = dev;

/**
 * Field names that should be redacted from logs.
 * These are common names used for sensitive data.
 */
const SENSITIVE_FIELDS = new Set([
	'apiKey',
	'api_key',
	'key',
	'token',
	'accessToken',
	'access_token',
	'refreshToken',
	'refresh_token',
	'secret',
	'secretKey',
	'secret_key',
	'password',
	'passwd',
	'pass',
	'authorization',
	'auth',
	'credential',
	'credentials',
	'privateKey',
	'private_key',
	'private-key',
	'sessionId',
	'session_id',
	'cookie',
	'csrf',
	'xsrf'
]);

/**
 * Patterns that indicate sensitive data in strings.
 * Used to detect and redact API key formats and auth headers.
 */
const SENSITIVE_PATTERNS = [
	// API key prefixes
	/\bsk-[a-zA-Z0-9]{20,}\b/gi,
	/\bsk-ant-[a-zA-Z0-9-]{20,}\b/gi,
	/\bAIza[a-zA-Z0-9_-]{35}\b/g,
	// Bearer tokens
	/\bBearer\s+[a-zA-Z0-9._-]+/gi,
	// Basic auth
	/\bBasic\s+[a-zA-Z0-9._-]+/gi,
	// JWT-like tokens
	/\beyJ[a-zA-Z0-9_-]*\.eyJ[a-zA-Z0-9_-]*\.[a-zA-Z0-9_-]*/g
];

/**
 * Redacts a string value if it matches sensitive patterns.
 */
function redactStringValue(value: string): string {
	let result = value;
	for (const pattern of SENSITIVE_PATTERNS) {
		result = result.replace(pattern, '[REDACTED]');
	}
	return result;
}

/**
 * Checks if a key name indicates sensitive data.
 */
function isSensitiveKey(key: string): boolean {
	const normalizedKey = key.toLowerCase().replace(/[-_]/g, '');
	for (const field of SENSITIVE_FIELDS) {
		const normalizedField = field.toLowerCase().replace(/[-_]/g, '');
		if (normalizedKey.includes(normalizedField)) {
			return true;
		}
	}
	return false;
}

/**
 * Recursively redacts sensitive data from an object.
 *
 * @param obj - The object to redact
 * @param depth - Current recursion depth (to prevent infinite loops)
 * @returns A new object with sensitive data redacted
 */
function redactSensitiveData(obj: unknown, depth = 0): unknown {
	// Prevent infinite recursion
	if (depth > 10) {
		return '[MAX_DEPTH_REACHED]';
	}

	// Handle null and undefined
	if (obj === null || obj === undefined) {
		return obj;
	}

	// Handle primitives
	if (typeof obj === 'string') {
		return redactStringValue(obj);
	}

	if (typeof obj !== 'object') {
		return obj;
	}

	// Handle Error objects
	if (obj instanceof Error) {
		return {
			name: obj.name,
			message: redactStringValue(obj.message),
			stack: obj.stack ? redactStringValue(obj.stack) : undefined
		};
	}

	// Handle arrays
	if (Array.isArray(obj)) {
		return obj.map((item) => redactSensitiveData(item, depth + 1));
	}

	// Handle regular objects
	const redacted: Record<string, unknown> = {};
	for (const [key, value] of Object.entries(obj as Record<string, unknown>)) {
		if (isSensitiveKey(key)) {
			redacted[key] = '[REDACTED]';
		} else {
			redacted[key] = redactSensitiveData(value, depth + 1);
		}
	}
	return redacted;
}

/**
 * Formats arguments for logging, redacting any sensitive data.
 */
function formatArgs(args: unknown[]): unknown[] {
	return args.map((arg) => redactSensitiveData(arg));
}

/**
 * Determines if logging should be enabled.
 * Uses SvelteKit's built-in dev mode detection.
 */
function shouldLog(): boolean {
	return isDebugEnabled;
}

/**
 * Secure logger that redacts sensitive information before logging.
 *
 * Usage:
 * ```typescript
 * import { logger } from '$lib/utils/logger';
 *
 * logger.info('[Component] Message', { apiKey: 'secret' });
 * // Output: [Component] Message { apiKey: '[REDACTED]' }
 * ```
 */
export const logger = {
	/**
	 * Log an informational message.
	 * Sensitive data in arguments will be automatically redacted.
	 */
	info(...args: unknown[]): void {
		if (shouldLog()) {
			console.log(...formatArgs(args));
		}
	},

	/**
	 * Log a warning message.
	 * Sensitive data in arguments will be automatically redacted.
	 */
	warn(...args: unknown[]): void {
		if (shouldLog()) {
			console.warn(...formatArgs(args));
		}
	},

	/**
	 * Log an error message.
	 * Sensitive data in arguments will be automatically redacted.
	 * Note: Errors are logged even in production for debugging purposes,
	 * but sensitive data is still redacted.
	 */
	error(...args: unknown[]): void {
		// Always log errors, but redact sensitive data
		console.error(...formatArgs(args));
	},

	/**
	 * Log a debug message (only in development).
	 * Sensitive data in arguments will be automatically redacted.
	 */
	debug(...args: unknown[]): void {
		if (shouldLog()) {
			console.debug(...formatArgs(args));
		}
	},

	/**
	 * Create a child logger with a fixed prefix.
	 * Useful for component-specific logging.
	 *
	 * @param prefix - The prefix to prepend to all log messages
	 * @returns A logger instance with the prefix applied
	 */
	withPrefix(prefix: string): PrefixedLogger {
		return new PrefixedLoggerImpl(prefix);
	}
};

/**
 * Interface for a logger with a fixed prefix.
 */
export interface PrefixedLogger {
	info(...args: unknown[]): void;
	warn(...args: unknown[]): void;
	error(...args: unknown[]): void;
	debug(...args: unknown[]): void;
}

/**
 * Implementation of a prefixed logger.
 */
class PrefixedLoggerImpl implements PrefixedLogger {
	constructor(private readonly prefix: string) {}

	info(...args: unknown[]): void {
		logger.info(this.prefix, ...args);
	}

	warn(...args: unknown[]): void {
		logger.warn(this.prefix, ...args);
	}

	error(...args: unknown[]): void {
		logger.error(this.prefix, ...args);
	}

	debug(...args: unknown[]): void {
		logger.debug(this.prefix, ...args);
	}
}

/**
 * Helper function to create a component-specific logger.
 *
 * @param componentName - The name of the component
 * @returns A logger instance with the component name as prefix
 *
 * @example
 * ```typescript
 * const log = createLogger('ConfigModal');
 * log.info('Provider selected', { id: 'openai' });
 * // Output: [ConfigModal] Provider selected { id: 'openai' }
 * ```
 */
export function createLogger(componentName: string): PrefixedLogger {
	return logger.withPrefix(`[${componentName}]`);
}

// Export a default instance for convenience
export default logger;
