/**
 * Tests for NLP API Client
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
	addContextMessage,
	analyzeCode,
	applyResolutions,
	classifyIntent,
	clearConversationContext,
	createDebouncedClassifier,
	detectAmbiguities,
	detectMultiIntent,
	getConversationContext,
	isBackendAvailable,
	NlpApiError,
	resolveAmbiguity,
	updateProjectFiles
} from '../nlp';
import { invoke } from '@tauri-apps/api/core';

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
	invoke: vi.fn()
}));

const mockInvoke = vi.mocked(invoke);

describe('nlp API', () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	// ============================================================================
	// Intent Classification
	// ============================================================================

	describe('classifyIntent', () => {
		it('should classify user intent', async () => {
			const mockResult = {
				intent: 'generate_code' as const,
				confidence: 0.95,
				alternative_intents: [
					['refactor_code', 0.05],
					['explain_code', 0.02]
				]
			};
			mockInvoke.mockResolvedValueOnce(mockResult);

			const result = await classifyIntent({
				input: 'Write a function to sort an array',
				conversationId: 'conv-1'
			});

			expect(mockInvoke).toHaveBeenCalledWith('classify_intent', {
				input: 'Write a function to sort an array',
				conversation_id: 'conv-1'
			});
			expect(result.intent).toBe('generate_code');
			expect(result.confidence).toBe(0.95);
			expect(result.alternative_intents).toHaveLength(2);
		});

		it('should return unknown intent for unclear input', async () => {
			const mockResult = {
				intent: 'unknown' as const,
				confidence: 0.1,
				alternative_intents: []
			};
			mockInvoke.mockResolvedValueOnce(mockResult);

			const result = await classifyIntent({
				input: 'asdfgh',
				conversationId: 'conv-1'
			});

			expect(result.intent).toBe('unknown');
			expect(result.confidence).toBe(0.1);
		});

		it('should throw NlpApiError on failure', async () => {
			mockInvoke.mockRejectedValueOnce(new Error('NLP service unavailable'));

			await expect(
				classifyIntent({
					input: 'test',
					conversationId: 'conv-1'
				})
			).rejects.toThrow(NlpApiError);
		});

		it('should handle empty input', async () => {
			const mockResult = {
				intent: 'unknown' as const,
				confidence: 0.0,
				alternative_intents: []
			};
			mockInvoke.mockResolvedValueOnce(mockResult);

			const result = await classifyIntent({
				input: '',
				conversationId: 'conv-1'
			});

			expect(result.intent).toBe('unknown');
		});
	});

	// ============================================================================
	// Context Management
	// ============================================================================

	describe('getConversationContext', () => {
		it('should return conversation context', async () => {
			const mockContext = {
				recent_messages: [
					{
						id: 'msg-1',
						role: 'user' as const,
						content: 'Hello',
						timestamp: '2024-01-01T00:00:00Z',
						tokens: 5
					},
					{
						id: 'msg-2',
						role: 'assistant' as const,
						content: 'Hi there!',
						timestamp: '2024-01-01T00:01:00Z',
						tokens: 8
					}
				],
				metadata: {
					current_file: {
						path: 'src/main.ts',
						language: 'typescript'
					},
					active_symbols: [
						{
							name: 'calculate',
							kind: 'function' as const,
							file_path: 'src/math.ts',
							line: 42
						}
					],
					summary: 'Previous discussion about math utilities'
				}
			};
			mockInvoke.mockResolvedValueOnce(mockContext);

			const result = await getConversationContext('conv-1');

			expect(mockInvoke).toHaveBeenCalledWith('get_conversation_context', {
				conversation_id: 'conv-1'
			});
			expect(result.recent_messages).toHaveLength(2);
			expect(result.metadata.current_file?.path).toBe('src/main.ts');
		});

		it('should handle empty context', async () => {
			mockInvoke.mockResolvedValueOnce({
				recent_messages: [],
				metadata: {
					active_symbols: []
				}
			});

			const result = await getConversationContext('conv-1');

			expect(result.recent_messages).toEqual([]);
			expect(result.metadata.active_symbols).toEqual([]);
		});
	});

	describe('addContextMessage', () => {
		it('should add user message to context', async () => {
			mockInvoke.mockResolvedValueOnce(undefined);

			await addContextMessage({
				conversationId: 'conv-1',
				role: 'user',
				content: 'What is the weather?'
			});

			expect(mockInvoke).toHaveBeenCalledWith('add_context_message', {
				conversation_id: 'conv-1',
				role: 'user',
				content: 'What is the weather?'
			});
		});

		it('should add assistant message to context', async () => {
			mockInvoke.mockResolvedValueOnce(undefined);

			await addContextMessage({
				conversationId: 'conv-1',
				role: 'assistant',
				content: 'The weather is sunny.'
			});

			expect(mockInvoke).toHaveBeenCalledWith('add_context_message', {
				conversation_id: 'conv-1',
				role: 'assistant',
				content: 'The weather is sunny.'
			});
		});

		it('should add system message to context', async () => {
			mockInvoke.mockResolvedValueOnce(undefined);

			await addContextMessage({
				conversationId: 'conv-1',
				role: 'system',
				content: 'You are a helpful assistant.'
			});

			expect(mockInvoke).toHaveBeenCalledWith('add_context_message', {
				conversation_id: 'conv-1',
				role: 'system',
				content: 'You are a helpful assistant.'
			});
		});
	});

	describe('clearConversationContext', () => {
		it('should clear conversation context', async () => {
			mockInvoke.mockResolvedValueOnce(undefined);

			await clearConversationContext('conv-1');

			expect(mockInvoke).toHaveBeenCalledWith('clear_conversation_context', {
				conversation_id: 'conv-1'
			});
		});
	});

	// ============================================================================
	// Multi-Intent Detection
	// ============================================================================

	describe('detectMultiIntent', () => {
		it('should detect multiple intents', async () => {
			const mockResult = {
				intents: [
					{
						intent: 'generate_code' as const,
						params: {
							language: 'typescript'
						},
						span: {
							start: 0,
							end: 20,
							text: 'Create a function'
						},
						confidence: 0.95
					},
					{
						intent: 'write_test' as const,
						params: {},
						span: {
							start: 21,
							end: 35,
							text: 'and test it'
						},
						confidence: 0.9
					}
				],
				relationships: ['sequential' as const],
				execution_order: [0, 1]
			};
			mockInvoke.mockResolvedValueOnce(mockResult);

			const result = await detectMultiIntent({
				input: 'Create a function and test it',
				conversationId: 'conv-1'
			});

			expect(mockInvoke).toHaveBeenCalledWith('detect_multi_intent', {
				input: 'Create a function and test it',
				conversation_id: 'conv-1'
			});
			expect(result.intents).toHaveLength(2);
			expect(result.intents[0].intent).toBe('generate_code');
			expect(result.relationships[0]).toBe('sequential');
		});

		it('should return single intent for simple input', async () => {
			const mockResult = {
				intents: [
					{
						intent: 'explain_code' as const,
						params: {},
						span: {
							start: 0,
							end: 10,
							text: 'Explain this'
						},
						confidence: 0.98
					}
				],
				relationships: [],
				execution_order: [0]
			};
			mockInvoke.mockResolvedValueOnce(mockResult);

			const result = await detectMultiIntent({
				input: 'Explain this',
				conversationId: 'conv-1'
			});

			expect(result.intents).toHaveLength(1);
		});
	});

	// ============================================================================
	// Ambiguity Detection
	// ============================================================================

	describe('detectAmbiguities', () => {
		it('should detect ambiguities in input', async () => {
			const mockAmbiguities = [
				{
					id: 'amb-1',
					ambiguity_type: 'missing_file' as const,
					span: {
						start: 10,
						end: 20,
						text: 'that file'
					},
					severity: 'high' as const,
					clarification: {
						question: 'Which file do you mean?',
						options: [
							{ value: 'src/main.ts', label: 'src/main.ts' },
							{ value: 'src/app.ts', label: 'src/app.ts' }
						],
						allows_freeform: true
					},
					suggestions: [
						{
							value: 'src/main.ts',
							confidence: 0.7,
							reason: 'Most recently accessed'
						}
					]
				}
			];
			mockInvoke.mockResolvedValueOnce(mockAmbiguities);

			const result = await detectAmbiguities({
				input: 'Fix the bug in that file',
				conversationId: 'conv-1'
			});

			expect(mockInvoke).toHaveBeenCalledWith('detect_ambiguities', {
				input: 'Fix the bug in that file',
				conversation_id: 'conv-1'
			});
			expect(result).toHaveLength(1);
			expect(result[0].ambiguity_type).toBe('missing_file');
			expect(result[0].clarification.options).toHaveLength(2);
		});

		it('should return empty array for unambiguous input', async () => {
			mockInvoke.mockResolvedValueOnce([]);

			const result = await detectAmbiguities({
				input: 'What is 2 + 2?',
				conversationId: 'conv-1'
			});

			expect(result).toEqual([]);
		});
	});

	// ============================================================================
	// Ambiguity Resolution
	// ============================================================================

	describe('resolveAmbiguity', () => {
		it('should resolve ambiguity with selected option', async () => {
			const mockResolved = {
				id: 'amb-1',
				original_text: 'that file',
				resolution: 'src/main.ts',
				resolution_type: 'selected' as const
			};
			mockInvoke.mockResolvedValueOnce(mockResolved);

			const result = await resolveAmbiguity({
				ambiguityId: 'amb-1',
				originalText: 'that file',
				resolution: 'src/main.ts',
				resolutionType: 'selected'
			});

			expect(mockInvoke).toHaveBeenCalledWith('resolve_ambiguity', {
				ambiguity_id: 'amb-1',
				original_text: 'that file',
				resolution: 'src/main.ts',
				resolution_type: 'selected'
			});
			expect(result.resolution).toBe('src/main.ts');
		});

		it('should resolve ambiguity with custom input', async () => {
			const mockResolved = {
				id: 'amb-1',
				original_text: 'the function',
				resolution: 'calculateSum',
				resolution_type: 'custom' as const
			};
			mockInvoke.mockResolvedValueOnce(mockResolved);

			const result = await resolveAmbiguity({
				ambiguityId: 'amb-1',
				originalText: 'the function',
				resolution: 'calculateSum',
				resolutionType: 'custom'
			});

			expect(result.resolution_type).toBe('custom');
		});

		it('should dismiss ambiguity', async () => {
			const mockResolved = {
				id: 'amb-1',
				original_text: 'ambiguous text',
				resolution: '',
				resolution_type: 'dismissed' as const
			};
			mockInvoke.mockResolvedValueOnce(mockResolved);

			const result = await resolveAmbiguity({
				ambiguityId: 'amb-1',
				originalText: 'ambiguous text',
				resolution: '',
				resolutionType: 'dismissed'
			});

			expect(result.resolution_type).toBe('dismissed');
		});
	});

	describe('applyResolutions', () => {
		it('should apply multiple resolutions to input', async () => {
			mockInvoke.mockResolvedValueOnce('Fix the bug in src/main.ts and update src/utils.ts');

			const result = await applyResolutions({
				input: 'Fix the bug in that file and update this other file',
				resolutions: {
					'amb-1': 'src/main.ts',
					'amb-2': 'src/utils.ts'
				}
			});

			expect(mockInvoke).toHaveBeenCalledWith('apply_resolutions', {
				input: 'Fix the bug in that file and update this other file',
				resolutions: {
					'amb-1': 'src/main.ts',
					'amb-2': 'src/utils.ts'
				}
			});
			expect(result).toContain('src/main.ts');
			expect(result).toContain('src/utils.ts');
		});

		it('should handle empty resolutions', async () => {
			mockInvoke.mockResolvedValueOnce('Original input unchanged');

			const result = await applyResolutions({
				input: 'Original input unchanged',
				resolutions: {}
			});

			expect(result).toBe('Original input unchanged');
		});
	});

	// ============================================================================
	// Code Analysis
	// ============================================================================

	describe('analyzeCode', () => {
		it('should analyze TypeScript code', async () => {
			const mockAnalysis = {
				language: 'typescript' as const,
				structure: {
					functions: [
						{
							name: 'add',
							parameters: ['a', 'b'],
							return_type: 'number',
							line_start: 0,
							line_end: 2
						}
					],
					classes: [],
					imports: []
				},
				complexity: {
					cyclomatic: 1,
					lines: 3,
					nesting: 0
				}
			};
			mockInvoke.mockResolvedValueOnce(mockAnalysis);

			const result = await analyzeCode({
				code: 'function add(a: number, b: number) { return a + b; }',
				language: 'typescript'
			});

			expect(mockInvoke).toHaveBeenCalledWith('analyze_code', {
				code: 'function add(a: number, b: number) { return a + b; }',
				language: 'typescript'
			});
			expect(result.language).toBe('typescript');
			expect(result.structure.functions).toHaveLength(1);
		});

		it('should analyze Python code', async () => {
			const mockAnalysis = {
				language: 'python' as const,
				structure: {
					functions: [
						{
							name: 'calculate',
							parameters: ['x', 'y'],
							line_start: 0,
							line_end: 2
						}
					],
					classes: [],
					imports: []
				},
				complexity: {
					cyclomatic: 1,
					lines: 3,
					nesting: 0
				}
			};
			mockInvoke.mockResolvedValueOnce(mockAnalysis);

			const result = await analyzeCode({
				code: 'def calculate(x, y):\n    return x + y',
				language: 'python'
			});

			expect(result.language).toBe('python');
		});

		it('should analyze Rust code', async () => {
			const mockAnalysis = {
				language: 'rust' as const,
				structure: {
					functions: [
						{
							name: 'main',
							parameters: [],
							return_type: '()',
							line_start: 0,
							line_end: 2
						}
					],
					classes: [],
					imports: []
				},
				complexity: {
					cyclomatic: 1,
					lines: 3,
					nesting: 0
				}
			};
			mockInvoke.mockResolvedValueOnce(mockAnalysis);

			const result = await analyzeCode({
				code: 'fn main() {\n    println!("Hello");\n}',
				language: 'rust'
			});

			expect(result.language).toBe('rust');
		});

		it('should handle unknown language', async () => {
			const mockAnalysis = {
				language: 'unknown' as const,
				structure: {
					functions: [],
					classes: [],
					imports: []
				},
				complexity: {
					cyclomatic: 0,
					lines: 0,
					nesting: 0
				}
			};
			mockInvoke.mockResolvedValueOnce(mockAnalysis);

			const result = await analyzeCode({
				code: 'some code',
				language: 'unknown'
			});

			expect(result.language).toBe('unknown');
		});

		it('should analyze complex code structure', async () => {
			const mockAnalysis = {
				language: 'typescript' as const,
				structure: {
					functions: [
						{
							name: 'processData',
							parameters: ['data', 'options'],
							return_type: 'Promise<Result>',
							line_start: 10,
							line_end: 45
						},
						{
							name: 'validate',
							parameters: ['input'],
							return_type: 'boolean',
							line_start: 47,
							line_end: 55
						}
					],
					classes: [
						{
							name: 'Processor',
							methods: [
								{
									name: 'process',
									parameters: ['data'],
									return_type: 'Result',
									line_start: 5,
									line_end: 8
								}
							],
							line_start: 0,
							line_end: 60
						}
					],
					imports: ["import { Result } from './types'"]
				},
				complexity: {
					cyclomatic: 8,
					lines: 60,
					nesting: 3
				}
			};
			mockInvoke.mockResolvedValueOnce(mockAnalysis);

			const result = await analyzeCode({
				code: 'complex code here',
				language: 'typescript'
			});

			expect(result.structure.functions).toHaveLength(2);
			expect(result.structure.classes).toHaveLength(1);
			expect(result.complexity.cyclomatic).toBe(8);
		});
	});

	describe('updateProjectFiles', () => {
		it('should update project files', async () => {
			mockInvoke.mockResolvedValueOnce(undefined);

			await updateProjectFiles(['src/main.ts', 'src/utils.ts', 'src/app.ts']);

			expect(mockInvoke).toHaveBeenCalledWith('update_project_files', {
				files: ['src/main.ts', 'src/utils.ts', 'src/app.ts']
			});
		});

		it('should handle empty file list', async () => {
			mockInvoke.mockResolvedValueOnce(undefined);

			await updateProjectFiles([]);

			expect(mockInvoke).toHaveBeenCalledWith('update_project_files', {
				files: []
			});
		});
	});

	// ============================================================================
	// Convenience Functions
	// ============================================================================

	describe('createDebouncedClassifier', () => {
		beforeEach(() => {
			vi.useFakeTimers();
		});

		afterEach(() => {
			vi.useRealTimers();
		});

		it('should debounce classification calls', async () => {
			const mockResult = {
				intent: 'generate_code' as const,
				confidence: 0.95,
				alternative_intents: []
			};
			mockInvoke.mockResolvedValueOnce(mockResult);

			const classifier = createDebouncedClassifier(300);

			const promise1 = classifier.classify({
				input: 'Write code',
				conversationId: 'conv-1'
			});
			const promise2 = classifier.classify({
				input: 'Write more code',
				conversationId: 'conv-1'
			});
			const promise3 = classifier.classify({
				input: 'Write even more code',
				conversationId: 'conv-1'
			});

			// Fast forward to trigger debounce
			vi.advanceTimersByTime(300);

			const result = await promise3;
			expect(result).not.toBeNull();
			expect(result?.intent).toBe('generate_code');
		});

		it('should cancel pending classification', () => {
			const classifier = createDebouncedClassifier(300);

			classifier.classify({
				input: 'Write code',
				conversationId: 'conv-1'
			});

			classifier.cancel();

			vi.advanceTimersByTime(300);
			// Should not throw or hang
		});

		it('should return null on error', async () => {
			mockInvoke.mockRejectedValueOnce(new Error('NLP error'));

			const classifier = createDebouncedClassifier(100);

			const promise = classifier.classify({
				input: 'test',
				conversationId: 'conv-1'
			});

			vi.advanceTimersByTime(100);

			const result = await promise;
			expect(result).toBeNull();
		});

		it('should use custom delay', async () => {
			const mockResult = {
				intent: 'explain_code' as const,
				confidence: 0.9,
				alternative_intents: []
			};
			mockInvoke.mockResolvedValueOnce(mockResult);

			const classifier = createDebouncedClassifier(500);

			classifier.classify({
				input: 'Explain this',
				conversationId: 'conv-1'
			});

			vi.advanceTimersByTime(499);
			// Should not have called yet

			classifier.classify({
				input: 'Explain that',
				conversationId: 'conv-1'
			});

			vi.advanceTimersByTime(1);
			// Still not enough time since second call

			vi.advanceTimersByTime(499);

			// Now invoke should have been called
			expect(mockInvoke).toHaveBeenCalled();
		});
	});

	describe('isBackendAvailable', () => {
		it('should return true when backend is available', async () => {
			mockInvoke.mockResolvedValueOnce('2.0.0');

			const result = await isBackendAvailable();

			expect(result).toBe(true);
		});

		it('should return false when backend is not available', async () => {
			mockInvoke.mockRejectedValueOnce(new Error('Backend not available'));

			const result = await isBackendAvailable();

			expect(result).toBe(false);
		});

		it('should handle non-string response', async () => {
			mockInvoke.mockResolvedValueOnce({ version: '2.0.0' });

			const result = await isBackendAvailable();

			expect(result).toBe(true);
		});
	});

	// ============================================================================
	// Error Handling
	// ============================================================================

	describe('Error handling', () => {
		it('should include command name in error', async () => {
			mockInvoke.mockRejectedValueOnce(new Error('Backend error'));

			await expect(
				classifyIntent({
					input: 'test',
					conversationId: 'conv-1'
				})
			).rejects.toThrow('classify_intent');
		});

		it('should include original cause in error', async () => {
			const originalError = new Error('Network error');
			mockInvoke.mockRejectedValueOnce(originalError);

			try {
				await classifyIntent({
					input: 'test',
					conversationId: 'conv-1'
				});
				expect.fail('Should have thrown');
			} catch (error) {
				expect(error).toBeInstanceOf(NlpApiError);
				expect((error as NlpApiError).cause).toBe(originalError);
			}
		});

		it('should handle non-Error errors', async () => {
			mockInvoke.mockRejectedValueOnce('String error');

			await expect(
				classifyIntent({
					input: 'test',
					conversationId: 'conv-1'
				})
			).rejects.toThrow(NlpApiError);
		});
	});
});
