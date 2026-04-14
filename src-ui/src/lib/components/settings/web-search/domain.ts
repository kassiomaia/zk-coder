/**
 * Domain-pattern validation for the web-search workspace policy editor.
 *
 * Mirrors the Rust-side `DomainPattern::parse` rules so the UI can reject
 * bad input *before* persisting the list. Accepted shapes:
 *   - "*"                    → any domain (only meaningful in allowlist)
 *   - "github.com"           → exact host
 *   - ".rust-lang.org"       → base host + any subdomain
 *
 * Rejected: schemes ("https://x.com"), paths ("x.com/foo"), ports ("x.com:443"),
 * query strings, empty values, whitespace inside the pattern.
 */

export interface DomainValidationResult {
	/** `true` when the input would be accepted by the Rust-side parser. */
	valid: boolean;
	/** Normalized form (lowercased, trimmed) — only populated when valid. */
	normalized?: string;
	/** Human-readable reason for a rejection, suitable for inline UI. */
	reason?: string;
}

const HOST_LABEL = /^[a-z0-9]([a-z0-9-]*[a-z0-9])?$/;

/**
 * Validates a single domain pattern. Returns the normalized lowercase form
 * or a human-readable reason for rejection.
 */
export function validateDomain(input: string): DomainValidationResult {
	const trimmed = input.trim();
	if (trimmed === '') {
		return { valid: false, reason: 'Cannot be empty.' };
	}
	if (/\s/.test(trimmed)) {
		return { valid: false, reason: 'Whitespace is not allowed.' };
	}
	const lower = trimmed.toLowerCase();
	if (lower === '*') {
		return { valid: true, normalized: '*' };
	}
	if (lower.includes('://')) {
		return { valid: false, reason: "Remove the scheme (e.g. 'https://')." };
	}
	if (lower.includes('/')) {
		return { valid: false, reason: 'Paths are not allowed — enter a host only.' };
	}
	if (lower.includes('?') || lower.includes('#')) {
		return { valid: false, reason: 'Query strings / fragments are not allowed.' };
	}
	if (lower.includes(':')) {
		return { valid: false, reason: 'Ports are not allowed.' };
	}

	const hostPart = lower.startsWith('.') ? lower.slice(1) : lower;
	if (hostPart === '') {
		return {
			valid: false,
			reason: "A leading dot must be followed by a host (e.g. '.github.com')."
		};
	}
	const labels = hostPart.split('.');
	if (labels.length < 2) {
		return {
			valid: false,
			reason: 'Must contain at least one dot (e.g. host.tld).'
		};
	}
	for (const label of labels) {
		if (!HOST_LABEL.test(label)) {
			return {
				valid: false,
				reason: `Invalid host segment: '${label}'.`
			};
		}
	}
	return { valid: true, normalized: lower };
}

/**
 * Returns the set of entries that appear in both lists. The caller uses this
 * to render an inline warning — denylist wins at the enforcement layer, so
 * this is a user-error signal, not a block.
 */
export function findListConflicts(
	allowlist: readonly string[],
	denylist: readonly string[]
): string[] {
	const allowSet = new Set(allowlist.map((d) => d.toLowerCase()));
	return denylist.filter((d) => allowSet.has(d.toLowerCase()));
}

/**
 * Parses a human TTL expression (e.g. `"24h"`, `"30m"`, `"2d"`, `"5400"`)
 * into an integer number of seconds. Returns `null` for unparseable input
 * or values outside `[MIN_TTL_SECS, MAX_TTL_SECS]`.
 */
export function parseTtl(input: string): number | null {
	const trimmed = input.trim().toLowerCase();
	if (trimmed === '') return null;
	const match = trimmed.match(/^(\d+)\s*(s|m|h|d)?$/);
	if (!match) return null;
	const n = Number.parseInt(match[1], 10);
	if (!Number.isFinite(n) || n <= 0) return null;
	const unit = match[2] ?? 's';
	const factor = unit === 's' ? 1 : unit === 'm' ? 60 : unit === 'h' ? 3600 : 86400;
	const seconds = n * factor;
	if (seconds < MIN_TTL_SECS || seconds > MAX_TTL_SECS) return null;
	return seconds;
}

/**
 * Formats a number of seconds as a compact human-readable string. Prefers
 * the largest unit that divides evenly (7200s → "2h", 90s → "90s").
 */
export function formatTtl(seconds: number): string {
	if (seconds % 86400 === 0) return `${seconds / 86400}d`;
	if (seconds % 3600 === 0) return `${seconds / 3600}h`;
	if (seconds % 60 === 0) return `${seconds / 60}m`;
	return `${seconds}s`;
}

/** Minimum accepted TTL — matches the Rust-side constant (5 minutes). */
export const MIN_TTL_SECS = 5 * 60;
/** Maximum accepted TTL — matches the Rust-side constant (7 days). */
export const MAX_TTL_SECS = 7 * 24 * 60 * 60;
