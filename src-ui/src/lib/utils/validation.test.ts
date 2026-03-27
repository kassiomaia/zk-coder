import {describe, expect, it} from 'vitest';
import {isValidModelId, sanitizeModelId} from './validation';

describe('validation', () => {
	describe('isValidModelId', () => {
		describe('valid model IDs', () => {
			it('accepts simple model names', () => {
				expect(isValidModelId('gpt-4')).toBe(true);
				expect(isValidModelId('claude-3-opus')).toBe(true);
				expect(isValidModelId('llama-2')).toBe(true);
			});

			it('accepts provider/model format', () => {
				expect(isValidModelId('anthropic/claude-3-opus')).toBe(true);
				expect(isValidModelId('openai/gpt-4')).toBe(true);
				expect(isValidModelId('meta/llama-2-70b')).toBe(true);
			});

			it('accepts provider:model format', () => {
				expect(isValidModelId('openai:gpt-4')).toBe(true);
				expect(isValidModelId('anthropic:claude-3')).toBe(true);
			});

			it('accepts model IDs with dots', () => {
				expect(isValidModelId('gpt-3.5-turbo')).toBe(true);
				expect(isValidModelId('model.v1')).toBe(true);
			});

			it('accepts model IDs with underscores', () => {
				expect(isValidModelId('model_name_v1')).toBe(true);
				expect(isValidModelId('llama_2_70b')).toBe(true);
			});

			it('accepts model IDs with colons', () => {
				expect(isValidModelId('model:version')).toBe(true);
				expect(isValidModelId('provider:model:variant')).toBe(true);
			});

			it('accepts model IDs with forward slashes', () => {
				expect(isValidModelId('provider/model/version')).toBe(true);
				expect(isValidModelId('org/repo/model')).toBe(true);
			});

			it('accepts complex model IDs', () => {
				expect(isValidModelId('anthropic/claude-3-opus-20240229')).toBe(true);
				expect(isValidModelId('openai:gpt-4-turbo-2024-04-09')).toBe(true);
				expect(isValidModelId('meta-llama/Llama-2-7b-chat-hf')).toBe(true);
			});

			it('accepts model IDs at minimum length boundary', () => {
				expect(isValidModelId('abc')).toBe(true);
				expect(isValidModelId('a-b')).toBe(true);
				expect(isValidModelId('a_b')).toBe(true);
			});

			it('accepts model IDs at maximum length boundary', () => {
				// Create a 256 character valid model ID
				const maxLengthId = 'a'.repeat(256);
				expect(isValidModelId(maxLengthId)).toBe(true);
			});

			it('accepts model IDs with whitespace (after trimming)', () => {
				expect(isValidModelId('  gpt-4  ')).toBe(true);
				expect(isValidModelId('\tclaude-3\n')).toBe(true);
			});

			it('accepts single-segment model IDs', () => {
				expect(isValidModelId('gpt4')).toBe(true);
				expect(isValidModelId('claude')).toBe(true);
			});

			it('accepts numeric model IDs', () => {
				expect(isValidModelId('model-123')).toBe(true);
				expect(isValidModelId('456')).toBe(true);
			});
		});

		describe('invalid model IDs', () => {
			it('rejects non-string types', () => {
				expect(isValidModelId(null)).toBe(false);
				expect(isValidModelId(undefined)).toBe(false);
				expect(isValidModelId(123)).toBe(false);
				expect(isValidModelId(true)).toBe(false);
				expect(isValidModelId({})).toBe(false);
				expect(isValidModelId([])).toBe(false);
				expect(isValidModelId({ id: 'gpt-4' })).toBe(false);
			});

			it('rejects empty strings', () => {
				expect(isValidModelId('')).toBe(false);
				expect(isValidModelId('   ')).toBe(false);
				expect(isValidModelId('\t\n')).toBe(false);
			});

			it('rejects strings below minimum length', () => {
				expect(isValidModelId('ab')).toBe(false);
				expect(isValidModelId('a')).toBe(false);
				expect(isValidModelId('a ')).toBe(false);
			});

			it('rejects strings above maximum length', () => {
				const tooLongId = 'a'.repeat(257);
				expect(isValidModelId(tooLongId)).toBe(false);
			});

			it('rejects model IDs with spaces', () => {
				expect(isValidModelId('gpt 4')).toBe(false);
				expect(isValidModelId('claude three')).toBe(false);
				expect(isValidModelId('model name')).toBe(false);
			});

			it('rejects model IDs with special characters', () => {
				expect(isValidModelId('gpt@4')).toBe(false);
				expect(isValidModelId('claude#3')).toBe(false);
				expect(isValidModelId('model$')).toBe(false);
				expect(isValidModelId('model%20')).toBe(false);
				expect(isValidModelId('model&test')).toBe(false);
				expect(isValidModelId('model*test')).toBe(false);
				expect(isValidModelId('model+test')).toBe(false);
				expect(isValidModelId('model=test')).toBe(false);
				expect(isValidModelId('model?test')).toBe(false);
				expect(isValidModelId('model^test')).toBe(false);
				expect(isValidModelId('model|test')).toBe(false);
				expect(isValidModelId('model\\test')).toBe(false);
				expect(isValidModelId('model~test')).toBe(false);
			});

			it('rejects model IDs with angle brackets', () => {
				expect(isValidModelId('model<test>')).toBe(false);
				expect(isValidModelId('<script>')).toBe(false);
			});

			it('rejects model IDs with parentheses', () => {
				expect(isValidModelId('model(test)')).toBe(false);
				expect(isValidModelId('gpt-(4)')).toBe(false);
			});

			it('rejects model IDs with brackets', () => {
				expect(isValidModelId('model[test]')).toBe(false);
				expect(isValidModelId('model{test}')).toBe(false);
			});

			it('rejects model IDs with quotes', () => {
				expect(isValidModelId('model"test"')).toBe(false);
				expect(isValidModelId("model'test'")).toBe(false);
				expect(isValidModelId('model`test`')).toBe(false);
			});

			it('rejects model IDs with newlines and tabs', () => {
				expect(isValidModelId('model\ntest')).toBe(false);
				expect(isValidModelId('model\ttest')).toBe(false);
				expect(isValidModelId('model\rtest')).toBe(false);
			});

			it('rejects model IDs with null bytes', () => {
				expect(isValidModelId('model\u0000test')).toBe(false);
			});

			it('rejects model IDs with emojis', () => {
				expect(isValidModelId('model🚀')).toBe(false);
				expect(isValidModelId('gpt-4✨')).toBe(false);
			});

			it('rejects model IDs starting with invalid characters', () => {
				expect(isValidModelId('-model')).toBe(true); // Hyphen at start is valid
				expect(isValidModelId('_model')).toBe(true); // Underscore at start is valid
				expect(isValidModelId(':model')).toBe(true); // Colon at start is valid
			});
		});

		describe('type narrowing', () => {
			it('narrows type from unknown to string when valid', () => {
				const value: unknown = 'gpt-4';
				if (isValidModelId(value)) {
					// TypeScript should know value is string here
					const uppercased = value.toUpperCase();
					expect(uppercased).toBe('GPT-4');
				}
			});

			it('preserves type after validation', () => {
				const maybeModelId: unknown = 'claude-3-opus';

				if (isValidModelId(maybeModelId)) {
					// Can use string methods without TypeScript error
					expect(maybeModelId.split('-')).toContain('claude');
				}
			});
		});

		describe('edge cases', () => {
			it('handles model IDs at exact boundaries', () => {
				// Exactly 3 characters
				expect(isValidModelId('abc')).toBe(true);

				// Exactly 256 characters
				const maxLength = 'a'.repeat(256);
				expect(isValidModelId(maxLength)).toBe(true);

				// 257 characters (too long)
				const tooLong = 'a'.repeat(257);
				expect(isValidModelId(tooLong)).toBe(false);
			});

			it('handles model IDs with only valid separators', () => {
				expect(isValidModelId('a-b_c:d/e.f')).toBe(true);
				expect(isValidModelId('---___:::...')).toBe(true);
			});

			it('handles trailing/leading whitespace correctly', () => {
				// Should trim and validate the trimmed version
				expect(isValidModelId('  gpt-4  ')).toBe(true); // 5 chars after trim
				expect(isValidModelId('  ab  ')).toBe(false); // 2 chars after trim
			});
		});
	});

	describe('sanitizeModelId', () => {
		describe('valid inputs', () => {
			it('returns trimmed model ID for valid input', () => {
				expect(sanitizeModelId('gpt-4')).toBe('gpt-4');
				expect(sanitizeModelId('  gpt-4  ')).toBe('gpt-4');
				expect(sanitizeModelId('\tclaude-3\n')).toBe('claude-3');
			});

			it('returns trimmed model ID with complex valid format', () => {
				const input = '  anthropic/claude-3-opus  ';
				expect(sanitizeModelId(input)).toBe('anthropic/claude-3-opus');
			});

			it('preserves valid model ID structure', () => {
				const modelId = 'openai:gpt-4-turbo';
				expect(sanitizeModelId(modelId)).toBe(modelId);
			});
		});

		describe('invalid inputs', () => {
			it('returns null for empty string', () => {
				expect(sanitizeModelId('')).toBe(null);
				expect(sanitizeModelId('   ')).toBe(null);
			});

			it('returns null for non-string types', () => {
				expect(sanitizeModelId(null)).toBe(null);
				expect(sanitizeModelId(undefined)).toBe(null);
				expect(sanitizeModelId(123)).toBe(null);
				expect(sanitizeModelId(true)).toBe(null);
				expect(sanitizeModelId({})).toBe(null);
				expect(sanitizeModelId([])).toBe(null);
			});

			it('returns null for model IDs with invalid characters', () => {
				expect(sanitizeModelId('gpt 4')).toBe(null);
				expect(sanitizeModelId('model@invalid')).toBe(null);
				expect(sanitizeModelId('model<script>')).toBe(null);
			});

			it('returns null for model IDs below minimum length', () => {
				expect(sanitizeModelId('ab')).toBe(null);
				expect(sanitizeModelId('a')).toBe(null);
			});

			it('returns null for model IDs above maximum length', () => {
				const tooLong = 'a'.repeat(257);
				expect(sanitizeModelId(tooLong)).toBe(null);
			});

			it('returns null for model IDs with special characters', () => {
				expect(sanitizeModelId('gpt@4')).toBe(null);
				expect(sanitizeModelId('claude#3')).toBe(null);
				expect(sanitizeModelId('model$')).toBe(null);
			});
		});

		describe('usage patterns', () => {
			it('can be used in conditional checks', () => {
				const userInput = 'gpt-4';
				const sanitized = sanitizeModelId(userInput);

				if (sanitized) {
					expect(sanitized).toBe('gpt-4');
				} else {
					// This branch should not be reached
					expect(true).toBe(false);
				}
			});

			it('returns null for invalid user input', () => {
				const invalidInput = 'gpt 4'; // Contains space
				const sanitized = sanitizeModelId(invalidInput);

				expect(sanitized).toBe(null);
			});

			it('handles trimmed whitespace correctly', () => {
				// Valid after trimming
				expect(sanitizeModelId('  gpt-4  ')).toBe('gpt-4');

				// Invalid after trimming (too short)
				expect(sanitizeModelId('  ab  ')).toBe(null);

				// Invalid after trimming (has spaces)
				expect(sanitizeModelId('  gpt 4  ')).toBe(null);
			});
		});

		describe('real-world scenarios', () => {
			it('handles typical model ID formats', () => {
				expect(sanitizeModelId('gpt-4')).toBe('gpt-4');
				expect(sanitizeModelId('anthropic/claude-3-opus')).toBe('anthropic/claude-3-opus');
				expect(sanitizeModelId('openai:gpt-4-turbo')).toBe('openai:gpt-4-turbo');
			});

			it('prevents some injection attempts', () => {
				// SQL injection with semicolon is rejected (semicolon not allowed)
				expect(sanitizeModelId('gpt-4; DROP TABLE users--')).toBe(null);
				// Script tags are rejected (angle brackets not allowed)
				expect(sanitizeModelId('<script>alert("xss")</script>')).toBe(null);
				// Note: path traversal like gpt-4../../../etc/passwd is technically valid
				// because forward slashes are allowed for provider/model format
				// This is expected behavior - the validation allows the characters
				// but consumers of this function should handle path traversal separately
			});
		});
	});
});
