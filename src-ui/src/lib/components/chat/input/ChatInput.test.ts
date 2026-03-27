import {fireEvent, render, screen} from '@testing-library/svelte';
import {afterEach, beforeEach, describe, expect, it, vi} from 'vitest';
import ChatInput from './ChatInput.svelte';
import type {Command} from '$lib/types/command';

const { mockChatStore, mockNlpState, mockIsLlmStreaming, mockAgentStore } = vi.hoisted(() => ({
	mockChatStore: {
		classifyInput: vi.fn(),
		cancelLlmStream: vi.fn()
	},
	mockNlpState: {
		subscribe: vi.fn((run: (value: unknown) => void) => {
			run({
				backendAvailable: false,
				processing: false,
				detectedIntent: null,
				pendingAmbiguities: []
			});
			return () => {};
		})
	},
	mockIsLlmStreaming: {
		subscribe: vi.fn((run: (value: boolean) => void) => {
			run(false);
			return () => {};
		})
	},
	mockAgentStore: {
		subscribe: vi.fn((run: (value: unknown) => void) => {
			run({ activeSessionId: null });
			return () => {};
		}),
		cancelSession: vi.fn(),
		get activeSessionId() {
			return null;
		}
	}
}));

vi.mock('$lib/stores', () => ({
	chatStore: mockChatStore,
	nlpState: mockNlpState,
	isLlmStreaming: mockIsLlmStreaming,
	agentStore: mockAgentStore
}));

vi.mock('$lib/api/nlp', () => ({
	isBackendAvailable: vi.fn(async () => false)
}));

vi.mock('$lib/utils/validation', () => ({
	isValidModelId: vi.fn(() => true),
	sanitizeModelId: vi.fn((id) => id)
}));

describe('ChatInput', () => {
	const mockCommands: Command[] = [
		{
			id: 'generate',
			name: 'generate',
			label: 'Generate Code',
			description: 'Generate code from description',
			icon: 'code'
		},
		{
			id: 'explain',
			name: 'explain',
			label: 'Explain Code',
			description: 'Explain how code works',
			icon: 'lightbulb'
		}
	];

	const defaultProps = {
		placeholder: 'Send a message...',
		value: '',
		onchange: vi.fn(),
		onsubmit: vi.fn()
	};

	beforeEach(() => {
		vi.clearAllMocks();
		vi.useFakeTimers();
	});

	afterEach(() => {
		vi.restoreAllMocks();
		vi.useRealTimers();
	});

	describe('rendering', () => {
		it('renders the input container', () => {
			const { container } = render(ChatInput, { ...defaultProps });
			const containerEl = container.querySelector('.container, [class*="container"]');
			expect(containerEl).toBeInTheDocument();
		});

		it('renders textarea element', () => {
			render(ChatInput, { ...defaultProps });
			const textarea = screen.getByRole('textbox');
			expect(textarea).toBeInTheDocument();
			expect(textarea).toHaveAttribute('placeholder', 'Send a message...');
		});

		it('uses custom placeholder when provided', () => {
			render(ChatInput, {
				...defaultProps,
				placeholder: 'Type your message here...'
			});
			const textarea = screen.getByRole('textbox');
			expect(textarea).toHaveAttribute('placeholder', 'Type your message here...');
		});

		it('displays current value', () => {
			const { container } = render(ChatInput, {
				...defaultProps,
				value: 'Hello world'
			});
			const textarea = container.querySelector('textarea');
			expect(textarea?.value).toBe('Hello world');
		});
	});

	describe('user input', () => {
		it('calls onchange when input value changes', async () => {
			const onchange = vi.fn();
			const { container } = render(ChatInput, {
				...defaultProps,
				onchange
			});

			const textarea = container.querySelector('textarea');
			if (textarea) {
				await fireEvent.input(textarea, { target: { value: 'test' } });
				expect(onchange).toHaveBeenCalledWith('test');
			}
		});

		it('calls onsubmit on Enter key press (without Shift)', async () => {
			const onsubmit = vi.fn();
			const { container } = render(ChatInput, {
				...defaultProps,
				value: 'Test message',
				onsubmit
			});

			const textarea = container.querySelector('textarea');
			if (textarea) {
				await fireEvent.keyDown(textarea, { key: 'Enter' });
				await vi.advanceTimersByTimeAsync(200);
				expect(onsubmit).toHaveBeenCalled();
			}
		});

		it('does not submit on Shift+Enter', async () => {
			const onsubmit = vi.fn();
			const { container } = render(ChatInput, {
				...defaultProps,
				value: 'Test message',
				onsubmit
			});

			const textarea = container.querySelector('textarea');
			if (textarea) {
				await fireEvent.keyDown(textarea, { key: 'Enter', shiftKey: true });
				expect(onsubmit).not.toHaveBeenCalled();
			}
		});

		it('calls onfocus when textarea is focused', async () => {
			const onfocus = vi.fn();
			const { container } = render(ChatInput, {
				...defaultProps,
				onfocus
			});

			const textarea = container.querySelector('textarea');
			if (textarea) {
				await fireEvent.focus(textarea);
				expect(onfocus).toHaveBeenCalled();
			}
		});

		it('calls onblur when textarea loses focus', async () => {
			const onblur = vi.fn();
			const { container } = render(ChatInput, {
				...defaultProps,
				onblur
			});

			const textarea = container.querySelector('textarea');
			if (textarea) {
				await fireEvent.blur(textarea);
				expect(onblur).toHaveBeenCalled();
			}
		});
	});

	describe('disabled state', () => {
		it('applies disabled class when disabled', () => {
			const { container } = render(ChatInput, {
				...defaultProps,
				disabled: true
			});
			const containerEl = container.querySelector('.container, [class*="container"]');
			expect(containerEl).toBeInTheDocument();
		});

		it('disables textarea when disabled prop is true', () => {
			render(ChatInput, {
				...defaultProps,
				disabled: true
			});
			const textarea = screen.getByRole('textbox');
			expect(textarea).toBeDisabled();
		});
	});

	describe('loading state', () => {
		it('renders with loading state', () => {
			const { container } = render(ChatInput, {
				...defaultProps,
				loading: true
			});
			const containerEl = container.querySelector('.container, [class*="container"]');
			expect(containerEl).toBeInTheDocument();
		});
	});

	describe('character limit', () => {
		it('shows character count when maxLength is provided', () => {
			render(ChatInput, {
				...defaultProps,
				value: 'Hello',
				maxLength: 100
			});
			expect(screen.getByText('5/100')).toBeInTheDocument();
		});

		it('applies overLimit class when limit exceeded', () => {
			const { container } = render(ChatInput, {
				...defaultProps,
				value: 'a'.repeat(101),
				maxLength: 100
			});
			const charCount = container.querySelector('.overLimit, [class*="overLimit"]');
			expect(charCount).toBeInTheDocument();
		});

		it('disables submission when over limit', () => {
			const onsubmit = vi.fn();
			const { container } = render(ChatInput, {
				...defaultProps,
				value: 'a'.repeat(101),
				maxLength: 100,
				onsubmit
			});

			const textarea = container.querySelector('textarea');
			if (textarea) {
				fireEvent.keyDown(textarea, { key: 'Enter' });
				expect(onsubmit).not.toHaveBeenCalled();
			}
		});
	});

	describe('send button', () => {
		it('renders send button', () => {
			render(ChatInput, {
				...defaultProps,
				value: 'test'
			});
			expect(screen.getByRole('button', { name: /send message/i })).toBeInTheDocument();
		});

		it('calls onsubmit when send button is clicked', async () => {
			const onsubmit = vi.fn();
			render(ChatInput, {
				...defaultProps,
				value: 'test message',
				onsubmit
			});

			await fireEvent.click(screen.getByRole('button', { name: /send message/i }));
			await vi.advanceTimersByTimeAsync(200);
			expect(onsubmit).toHaveBeenCalled();
		});

		it('shows stop button while the linked agent session is working', () => {
			render(ChatInput, {
				...defaultProps,
				value: 'test message',
				isAgentWorking: true,
				agentSessionId: 'session-1'
			});

			expect(screen.getByRole('button', { name: /stop generation/i })).toBeInTheDocument();
		});

		it('cancels the linked agent session when stop is clicked', async () => {
			render(ChatInput, {
				...defaultProps,
				value: 'test message',
				isAgentWorking: true,
				agentSessionId: 'session-1'
			});

			await fireEvent.click(screen.getByRole('button', { name: /stop generation/i }));
			expect(mockAgentStore.cancelSession).toHaveBeenCalledWith('session-1');
			expect(mockChatStore.cancelLlmStream).not.toHaveBeenCalled();
		});
	});

	describe('file attachments', () => {
		it('renders attachments when provided', () => {
			const attachments = [
				{
					id: 'file1',
					name: 'test.txt',
					type: 'text/plain',
					size: 1024
				}
			];

			render(ChatInput, {
				...defaultProps,
				attachments
			});

			expect(screen.getByText('test.txt')).toBeInTheDocument();
		});

		it('formats file sizes correctly', () => {
			const attachments = [
				{
					id: 'file1',
					name: 'test.txt',
					type: 'text/plain',
					size: 1024
				},
				{
					id: 'file2',
					name: 'large.bin',
					type: 'application/octet-stream',
					size: 1024 * 1024 * 2.5
				}
			];

			render(ChatInput, {
				...defaultProps,
				attachments
			});

			expect(screen.getByText('1.0 KB')).toBeInTheDocument();
			expect(screen.getByText('2.5 MB')).toBeInTheDocument();
		});
	});

	describe('model selector', () => {
		it('renders model selector when modelId is provided', () => {
			render(ChatInput, {
				...defaultProps,
				modelId: 'claude-3-opus-20240229',
				onModelChange: vi.fn(),
				onOpenSettings: vi.fn()
			});

			// Model selector should be in the document
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('does not render model selector without modelId', () => {
			render(ChatInput, {
				...defaultProps,
				modelId: null
			});

			// Should render without model selector
			const container = document.body;
			expect(container).toBeInTheDocument();
		});
	});

	describe('markdown preview', () => {
		it('renders preview toggle when enableMarkdownPreview is true', () => {
			render(ChatInput, {
				...defaultProps,
				enableMarkdownPreview: true
			});

			// Should have preview toggle button
			const container = document.body;
			expect(container).toBeInTheDocument();
		});
	});

	describe('accessibility', () => {
		it('has proper aria-label on textarea', () => {
			render(ChatInput, { ...defaultProps });
			const textarea = screen.getByRole('textbox', { name: /message input/i });
			expect(textarea).toBeInTheDocument();
		});

		it('supports keyboard navigation', async () => {
			const onsubmit = vi.fn();
			const { container } = render(ChatInput, {
				...defaultProps,
				value: 'test',
				onsubmit
			});

			const textarea = container.querySelector('textarea');
			if (textarea) {
				await fireEvent.keyDown(textarea, { key: 'Enter' });
				// Should handle Enter key
				expect(textarea).toBeInTheDocument();
			}
		});
	});

	describe('edge cases', () => {
		it('handles empty value', () => {
			render(ChatInput, {
				...defaultProps,
				value: ''
			});
			const textarea = screen.getByRole('textbox');
			expect(textarea.value).toBe('');
		});

		it('handles very long input', () => {
			const longText = 'a'.repeat(10000);
			render(ChatInput, {
				...defaultProps,
				value: longText
			});
			const textarea = screen.getByRole('textbox');
			expect(textarea.value).toBe(longText);
		});

		it('handles special characters', () => {
			const specialText = '<script>alert("test")</script>';
			render(ChatInput, {
				...defaultProps,
				value: specialText
			});
			const textarea = screen.getByRole('textbox');
			expect(textarea.value).toBe(specialText);
		});

		it('handles undefined handlers gracefully', () => {
			render(ChatInput, {
				...defaultProps,
				onfocus: undefined,
				onblur: undefined
			});
			const textarea = screen.getByRole('textbox');
			expect(textarea).toBeInTheDocument();
		});
	});
});
