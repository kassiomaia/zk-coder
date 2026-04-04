/**
 * Tests for File API Client
 */
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { type FileContentResponse, getFileContent } from '../file';
import { invoke } from '@tauri-apps/api/core';

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
	invoke: vi.fn()
}));

const mockInvoke = vi.mocked(invoke);

describe('file API', () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	describe('getFileContent', () => {
		it('should return file content for existing file', async () => {
			const mockResponse: FileContentResponse = {
				path: 'src/main.ts',
				exists: true,
				current_content: 'console.log("hello");',
				original_content: null,
				diff: null,
				is_new: true
			};
			mockInvoke.mockResolvedValueOnce(mockResponse);

			const result = await getFileContent('/workspace', 'src/main.ts', 'session-123');

			expect(mockInvoke).toHaveBeenCalledWith('get_file_content', {
				workspaceRoot: '/workspace',
				filePath: 'src/main.ts',
				sessionId: 'session-123'
			});
			expect(result.path).toBe('src/main.ts');
			expect(result.exists).toBe(true);
			expect(result.current_content).toBe('console.log("hello");');
			expect(result.is_new).toBe(true);
		});

		it('should return file content with original content and diff', async () => {
			const mockResponse: FileContentResponse = {
				path: 'src/utils.ts',
				exists: true,
				current_content: 'function add(a: number, b: number) {\n  return a + b;\n}',
				original_content: 'function add(a, b) {\n  return a + b;\n}',
				diff: '- function add(a, b) {\n+ function add(a: number, b: number) {\n    return a + b;\n}',
				is_new: false
			};
			mockInvoke.mockResolvedValueOnce(mockResponse);

			const result = await getFileContent('/workspace', 'src/utils.ts', 'session-123');

			expect(result.exists).toBe(true);
			expect(result.original_content).toContain('function add(a, b)');
			expect(result.diff).toContain('- function add(a, b)');
			expect(result.is_new).toBe(false);
		});

		it('should return null current_content for non-existent file', async () => {
			const mockResponse: FileContentResponse = {
				path: 'src/missing.ts',
				exists: false,
				current_content: null,
				original_content: null,
				diff: null,
				is_new: false
			};
			mockInvoke.mockResolvedValueOnce(mockResponse);

			const result = await getFileContent('/workspace', 'src/missing.ts', 'session-123');

			expect(result.exists).toBe(false);
			expect(result.current_content).toBeNull();
		});

		it('should handle deleted file with original content', async () => {
			const mockResponse: FileContentResponse = {
				path: 'src/deleted.ts',
				exists: false,
				current_content: null,
				original_content: 'const x = 1;',
				diff: '- const x = 1;',
				is_new: false
			};
			mockInvoke.mockResolvedValueOnce(mockResponse);

			const result = await getFileContent('/workspace', 'src/deleted.ts', 'session-123');

			expect(result.exists).toBe(false);
			expect(result.original_content).toBe('const x = 1;');
			expect(result.diff).toContain('- const x = 1;');
		});

		it('should handle empty file', async () => {
			const mockResponse: FileContentResponse = {
				path: 'src/empty.ts',
				exists: true,
				current_content: '',
				original_content: '',
				diff: null,
				is_new: false
			};
			mockInvoke.mockResolvedValueOnce(mockResponse);

			const result = await getFileContent('/workspace', 'src/empty.ts', 'session-123');

			expect(result.exists).toBe(true);
			expect(result.current_content).toBe('');
		});

		it('should handle large file content', async () => {
			const largeContent = 'x'.repeat(10000);
			const mockResponse: FileContentResponse = {
				path: 'src/large.ts',
				exists: true,
				current_content: largeContent,
				original_content: null,
				diff: null,
				is_new: true
			};
			mockInvoke.mockResolvedValueOnce(mockResponse);

			const result = await getFileContent('/workspace', 'src/large.ts', 'session-123');

			expect(result.current_content).toHaveLength(10000);
		});

		it('should handle special characters in file path', async () => {
			const mockResponse: FileContentResponse = {
				path: 'src/path with spaces/file.ts',
				exists: true,
				current_content: 'content',
				original_content: null,
				diff: null,
				is_new: true
			};
			mockInvoke.mockResolvedValueOnce(mockResponse);

			const result = await getFileContent(
				'/workspace',
				'src/path with spaces/file.ts',
				'session-123'
			);

			expect(result.path).toBe('src/path with spaces/file.ts');
		});

		it('should handle different session IDs', async () => {
			const mockResponse: FileContentResponse = {
				path: 'src/test.ts',
				exists: true,
				current_content: 'content',
				original_content: null,
				diff: null,
				is_new: true
			};
			mockInvoke.mockResolvedValueOnce(mockResponse);

			await getFileContent('/workspace', 'src/test.ts', 'session-abc');
			await getFileContent('/workspace', 'src/test.ts', 'session-xyz');

			expect(mockInvoke).toHaveBeenNthCalledWith(1, 'get_file_content', {
				workspaceRoot: '/workspace',
				filePath: 'src/test.ts',
				sessionId: 'session-abc'
			});
			expect(mockInvoke).toHaveBeenNthCalledWith(2, 'get_file_content', {
				workspaceRoot: '/workspace',
				filePath: 'src/test.ts',
				sessionId: 'session-xyz'
			});
		});

		it('should handle unicode content', async () => {
			const unicodeContent = 'Hello 世界 🌍 Ñoño';
			const mockResponse: FileContentResponse = {
				path: 'src/unicode.ts',
				exists: true,
				current_content: unicodeContent,
				original_content: null,
				diff: null,
				is_new: true
			};
			mockInvoke.mockResolvedValueOnce(mockResponse);

			const result = await getFileContent('/workspace', 'src/unicode.ts', 'session-123');

			expect(result.current_content).toBe(unicodeContent);
		});

		it('should handle workspace root with trailing slash', async () => {
			const mockResponse: FileContentResponse = {
				path: 'src/test.ts',
				exists: true,
				current_content: 'content',
				original_content: null,
				diff: null,
				is_new: true
			};
			mockInvoke.mockResolvedValueOnce(mockResponse);

			await getFileContent('/workspace/', 'src/test.ts', 'session-123');

			expect(mockInvoke).toHaveBeenCalledWith('get_file_content', {
				workspaceRoot: '/workspace/',
				filePath: 'src/test.ts',
				sessionId: 'session-123'
			});
		});
	});
});
