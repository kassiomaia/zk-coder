/**
 * Content Validation Utilities
 * Provides validation and sanitization for markdown content to prevent XSS and DoS attacks.
 */

import { createLogger } from './logger';

const log = createLogger('contentValidation');

/**
 * Configuration options for content validation
 */
export interface ContentValidationConfig {
	/** Maximum content length in characters (default: 1MB = 1,000,000 chars) */
	maxContentLength?: number;
	/** Whether to allow raw HTML in content (default: false) */
	allowHtml?: boolean;
	/** Allowed URL protocols for links and images (default: ['http:', 'https:']) */
	allowedProtocols?: string[];
	/** Additional dangerous tags to disallow beyond defaults */
	disallowedTags?: string[];
	/** Whether to sanitize links in content (default: true) */
	sanitizeLinks?: boolean;
	/** Whether to show validation errors to users (default: true) */
	showErrors?: boolean;
}

/**
 * Default validation configuration with secure defaults
 */
export const defaultValidationConfig: Required<ContentValidationConfig> = {
	maxContentLength: 1000000, // 1MB
	allowHtml: false,
	allowedProtocols: ['http:', 'https:', 'mailto:'],
	disallowedTags: ['script', 'iframe', 'object', 'embed', 'form', 'input', 'style'],
	sanitizeLinks: true,
	showErrors: true
};

/**
 * Dangerous URL schemes that should always be blocked
 */
const DANGEROUS_PROTOCOLS = ['javascript:', 'data:', 'vbscript:', 'file:', 'blob:'];

/**
 * Result of content validation
 */
export interface ValidationResult {
	/** Whether the content passed validation */
	isValid: boolean;
	/** Sanitized content safe for rendering */
	sanitizedContent: string;
	/** Error message if validation failed */
	error?: string;
	/** Warning messages (content was auto-sanitized) */
	warnings: string[];
}

/**
 * Validate and sanitize markdown content
 * @param content - Raw markdown content to validate
 * @param config - Validation configuration options
 * @returns ValidationResult with sanitized content and any errors/warnings
 */
export function validateContent(
	content: string,
	config: ContentValidationConfig = {}
): ValidationResult {
	const mergedConfig = { ...defaultValidationConfig, ...config };
	const warnings: string[] = [];

	// Handle null/undefined content
	if (content === null || content === undefined) {
		return {
			isValid: true,
			sanitizedContent: '',
			warnings: []
		};
	}

	// Ensure content is a string
	let processedContent = String(content);

	// Check content length
	if (mergedConfig.maxContentLength && processedContent.length > mergedConfig.maxContentLength) {
		return {
			isValid: false,
			sanitizedContent: '',
			error: `Content exceeds maximum length of ${mergedConfig.maxContentLength.toLocaleString()} characters`,
			warnings: []
		};
	}

	// Check for HTML content when not allowed
	if (!mergedConfig.allowHtml) {
		const htmlCheckResult = checkForHtml(processedContent, mergedConfig.disallowedTags);
		if (htmlCheckResult.hasHtml) {
			// Strip dangerous HTML tags but allow the content through
			processedContent = stripDangerousTags(processedContent, mergedConfig.disallowedTags);
			warnings.push('HTML content was stripped from the input');
		}
	}

	// Sanitize links if enabled
	if (mergedConfig.sanitizeLinks) {
		const linkResult = sanitizeLinks(processedContent, mergedConfig.allowedProtocols);
		processedContent = linkResult.content;
		warnings.push(...linkResult.warnings);
	}

	return {
		isValid: true,
		sanitizedContent: processedContent,
		warnings
	};
}

/**
 * Check if content contains HTML tags
 */
function checkForHtml(
	content: string,
	disallowedTags: string[]
): { hasHtml: boolean; foundTags: string[] } {
	const foundTags: string[] = [];

	// Check for any HTML tags
	const htmlTagRegex = /<([a-z][a-z0-9]*)\b[^>]*>/gi;
	let match;
	while ((match = htmlTagRegex.exec(content)) !== null) {
		const tag = match[1].toLowerCase();
		if (disallowedTags.includes(tag)) {
			foundTags.push(tag);
		}
	}

	return {
		hasHtml: foundTags.length > 0,
		foundTags
	};
}

/**
 * Strip dangerous HTML tags from content
 */
function stripDangerousTags(content: string, disallowedTags: string[]): string {
	let result = content;

	for (const tag of disallowedTags) {
		// Remove opening tags with attributes
		const openTagRegex = new RegExp(`<${tag}\\b[^>]*>`, 'gi');
		result = result.replace(openTagRegex, '');

		// Remove closing tags
		const closeTagRegex = new RegExp(`</${tag}>`, 'gi');
		result = result.replace(closeTagRegex, '');

		// Remove self-closing tags
		const selfCloseRegex = new RegExp(`<${tag}\\b[^>]*/>`, 'gi');
		result = result.replace(selfCloseRegex, '');
	}

	return result;
}

/**
 * Sanitize links in markdown content
 * Blocks dangerous protocols and validates URLs
 */
export function sanitizeLinks(
	content: string,
	allowedProtocols: string[] = defaultValidationConfig.allowedProtocols
): { content: string; warnings: string[] } {
	const warnings: string[] = [];

	// Sanitize markdown links: [text](url)
	const linkResult = content.replace(/\[([^\]]*)\]\(([^)]+)\)/g, (match, text, url) => {
		const sanitized = sanitizeUrl(url, allowedProtocols);
		if (sanitized !== url) {
			warnings.push(`Sanitized potentially unsafe URL: ${url.substring(0, 50)}...`);
		}
		return `[${text}](${sanitized})`;
	});

	// Sanitize markdown images: ![alt](url)
	const imageResult = linkResult.replace(/!\[([^\]]*)\]\(([^)]+)\)/g, (match, alt, url) => {
		const sanitized = sanitizeUrl(url, allowedProtocols);
		if (sanitized !== url) {
			warnings.push(`Sanitized potentially unsafe image URL: ${url.substring(0, 50)}...`);
		}
		return `![${alt}](${sanitized})`;
	});

	// Sanitize bare URLs
	const bareUrlResult = imageResult.replace(/(?<!!)\[(?:[^\]]*)\]\[([^\]]+)\]/g, (match, ref) => {
		// Reference-style links - these need separate handling
		return match;
	});

	return { content: bareUrlResult, warnings };
}

/**
 * Sanitize a single URL
 * Returns a safe URL or '#' if the URL is dangerous
 */
export function sanitizeUrl(url: string, allowedProtocols: string[] = []): string {
	const trimmedUrl = url.trim();

	// Handle empty URLs
	if (!trimmedUrl) {
		return '#';
	}

	// Handle anchor links (safe)
	if (trimmedUrl.startsWith('#')) {
		return trimmedUrl;
	}

	// Handle relative URLs (safe)
	if (trimmedUrl.startsWith('/') || trimmedUrl.startsWith('./') || trimmedUrl.startsWith('../')) {
		return trimmedUrl;
	}

	// Try to parse as URL
	try {
		const urlObj = new URL(trimmedUrl);

		// Check for dangerous protocols
		if (DANGEROUS_PROTOCOLS.includes(urlObj.protocol.toLowerCase())) {
			log.warn(`[Security] Blocked dangerous protocol: ${urlObj.protocol}`);
			return '#';
		}

		// Check against allowed protocols
		const protocolsToCheck =
			allowedProtocols.length > 0 ? allowedProtocols : defaultValidationConfig.allowedProtocols!;

		if (!protocolsToCheck.includes(urlObj.protocol.toLowerCase())) {
			log.warn(`[Security] Blocked disallowed protocol: ${urlObj.protocol}`);
			return '#';
		}

		return trimmedUrl;
	} catch {
		// Invalid URL format
		log.warn(`[Security] Invalid URL format: ${trimmedUrl.substring(0, 50)}`);
		return '#';
	}
}

/**
 * Quick check if content might need sanitization
 * Useful for skipping full validation on trusted content
 */
export function mightNeedSanitization(content: string): boolean {
	if (!content) return false;

	// Check for common attack patterns
	const suspiciousPatterns = [
		/javascript:/i,
		/data:/i,
		/vbscript:/i,
		/<script\b/i,
		/<iframe\b/i,
		/on\w+\s*=/i, // Event handlers like onclick=
		/<embed\b/i,
		/<object\b/i
	];

	return suspiciousPatterns.some((pattern) => pattern.test(content));
}
