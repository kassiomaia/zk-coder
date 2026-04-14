import { describe, expect, it } from 'vitest';
import {
	findListConflicts,
	formatTtl,
	MAX_TTL_SECS,
	MIN_TTL_SECS,
	parseTtl,
	validateDomain
} from './domain';

describe('validateDomain', () => {
	it('accepts a bare host with at least one dot', () => {
		expect(validateDomain('github.com')).toEqual({
			valid: true,
			normalized: 'github.com'
		});
	});

	it('accepts a subdomain pattern with leading dot', () => {
		expect(validateDomain('.rust-lang.org')).toEqual({
			valid: true,
			normalized: '.rust-lang.org'
		});
	});

	it('accepts the universal wildcard', () => {
		expect(validateDomain('*')).toEqual({ valid: true, normalized: '*' });
	});

	it('lowercases the normalized form', () => {
		expect(validateDomain('GitHub.COM').normalized).toBe('github.com');
	});

	it('rejects empty input', () => {
		expect(validateDomain('')).toMatchObject({ valid: false });
		expect(validateDomain('   ')).toMatchObject({ valid: false });
	});

	it('rejects schemes', () => {
		expect(validateDomain('https://x.com')).toMatchObject({ valid: false });
	});

	it('rejects paths', () => {
		expect(validateDomain('x.com/foo')).toMatchObject({ valid: false });
	});

	it('rejects ports', () => {
		expect(validateDomain('x.com:443')).toMatchObject({ valid: false });
	});

	it('rejects whitespace inside a pattern', () => {
		expect(validateDomain('a b.com')).toMatchObject({ valid: false });
	});

	it('rejects single-label hosts', () => {
		expect(validateDomain('localhost')).toMatchObject({ valid: false });
	});

	it('rejects lone leading dot', () => {
		expect(validateDomain('.')).toMatchObject({ valid: false });
	});

	it('rejects invalid characters', () => {
		expect(validateDomain('hello_world.com')).toMatchObject({ valid: false });
	});
});

describe('findListConflicts', () => {
	it('returns entries present in both lists', () => {
		const conflicts = findListConflicts(
			['github.com', 'rust-lang.org'],
			['github.com', 'pinterest.com']
		);
		expect(conflicts).toEqual(['github.com']);
	});

	it('is case-insensitive', () => {
		const conflicts = findListConflicts(['GitHub.com'], ['github.com']);
		expect(conflicts).toEqual(['github.com']);
	});

	it('returns empty when there is no overlap', () => {
		expect(findListConflicts(['github.com'], ['pinterest.com'])).toEqual([]);
	});
});

describe('parseTtl', () => {
	it('parses compact units', () => {
		expect(parseTtl('24h')).toBe(86_400);
		expect(parseTtl('30m')).toBe(1800);
		expect(parseTtl('2d')).toBe(172_800);
	});

	it('accepts bare seconds', () => {
		expect(parseTtl('1800')).toBe(1800);
	});

	it('trims whitespace', () => {
		expect(parseTtl('  24h  ')).toBe(86_400);
	});

	it('rejects out-of-range values', () => {
		expect(parseTtl('1s')).toBeNull();
		expect(parseTtl('30d')).toBeNull();
	});

	it('rejects malformed input', () => {
		expect(parseTtl('abc')).toBeNull();
		expect(parseTtl('')).toBeNull();
		expect(parseTtl('-5h')).toBeNull();
		expect(parseTtl('5x')).toBeNull();
	});

	it('boundaries are inclusive', () => {
		expect(parseTtl(`${MIN_TTL_SECS}s`)).toBe(MIN_TTL_SECS);
		expect(parseTtl(`${MAX_TTL_SECS}s`)).toBe(MAX_TTL_SECS);
	});
});

describe('formatTtl', () => {
	it('prefers the largest evenly-dividing unit', () => {
		expect(formatTtl(86_400)).toBe('1d');
		expect(formatTtl(7200)).toBe('2h');
		expect(formatTtl(300)).toBe('5m');
	});

	it('falls back to seconds for non-round values', () => {
		expect(formatTtl(90)).toBe('90s');
	});
});
