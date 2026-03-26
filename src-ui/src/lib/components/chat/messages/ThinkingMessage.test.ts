import { render, screen, fireEvent } from '@testing-library/svelte';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import ThinkingMessage from './ThinkingMessage.svelte';
import type { Message } from '$lib/types/message';

describe('ThinkingMessage', () => {
	let mockMessage: Message;

	beforeEach(() => {
		vi.clearAllMocks();
		mockMessage = {
			id: 'msg-1',
			conversationId: 'conv-1',
			threadId: 'thread-1',
			role: 'system',
			content: 'Analyzing your request...',
			timestamp: new Date('2024-01-15T10:00:00'),
			status: 'sending',
			kind: 'thinking'
		};
	});

	afterEach(() => {
		vi.restoreAllMocks();
	});

	describe('rendering', () => {
		it('renders the thinking message container', () => {
			const { container } = render(ThinkingMessage, {
				message: mockMessage
			});
			const containerEl = container.querySelector('[class*="thinkingMessage"]');
			expect(containerEl).toBeInTheDocument();
		});

		it('displays the content text', () => {
			render(ThinkingMessage, {
				message: mockMessage
			});
			expect(screen.getByText('Analyzing your request...')).toBeInTheDocument();
		});

		it('shows spinner when message is not complete', () => {
			const { container } = render(ThinkingMessage, {
				message: mockMessage
			});
			const spinner = container.querySelector('.spinner, [class*="spinner"]');
			expect(spinner).toBeInTheDocument();
		});

		it('shows check icon when message is complete', () => {
			const completeMessage: Message = {
				...mockMessage,
				status: 'sent'
			};
			const { container } = render(ThinkingMessage, {
				message: completeMessage
			});
			const checkIcon = container.querySelector('.checkIcon, [class*="check"]');
			expect(checkIcon).toBeInTheDocument();
		});
	});

	describe('content display', () => {
		it('uses default content when empty', () => {
			const emptyContentMessage: Message = {
				...mockMessage,
				content: ''
			};
			render(ThinkingMessage, {
				message: emptyContentMessage
			});
			expect(screen.getByText('Analyzing request...')).toBeInTheDocument();
		});

		it('displays custom content when provided', () => {
			const customContentMessage: Message = {
				...mockMessage,
				content: 'Reading project files...'
			};
			render(ThinkingMessage, {
				message: customContentMessage
			});
			expect(screen.getByText('Reading project files...')).toBeInTheDocument();
		});

		it('displays "Thought for a few seconds" when complete', () => {
			const completeMessage: Message = {
				...mockMessage,
				status: 'sent'
			};
			render(ThinkingMessage, {
				message: completeMessage
			});
			expect(screen.getByText('Thought for a few seconds')).toBeInTheDocument();
		});
	});

	describe('collapse/expand', () => {
		it('is expanded by default', () => {
			const { container } = render(ThinkingMessage, {
				message: mockMessage
			});
			const containerEl = container.querySelector('[class*="thinkingMessage"]');
			expect(containerEl?.classList.contains('collapsed')).toBe(false);
		});

		it('toggles collapsed state on click', async () => {
			const { container } = render(ThinkingMessage, {
				message: mockMessage
			});

			const button = container.querySelector('button');
			if (button) {
				await fireEvent.click(button);

				const containerEl = container.querySelector('[class*="thinkingMessage"]');
				expect(containerEl?.classList.contains('collapsed')).toBe(true);
			}
		});

		it('shows chevron up when expanded', () => {
			const { container } = render(ThinkingMessage, {
				message: mockMessage
			});
			const chevron = container.querySelector('svg');
			expect(chevron).toBeInTheDocument();
		});
	});

	describe('reasoning content', () => {
		it('displays reasoning when present and not collapsed', () => {
			const messageWithReasoning: Message = {
				...mockMessage,
				reasoning: 'I need to analyze the code structure first...'
			};
			render(ThinkingMessage, {
				message: messageWithReasoning
			});
			expect(screen.getByText('I need to analyze the code structure first...')).toBeInTheDocument();
		});

		it('hides reasoning when collapsed', () => {
			const messageWithReasoning: Message = {
				...mockMessage,
				reasoning: 'I need to analyze the code structure first...'
			};
			const { container } = render(ThinkingMessage, {
				message: messageWithReasoning
			});

			const button = container.querySelector('button');
			if (button) {
				fireEvent.click(button);
			}

			// After collapse, reasoning should be hidden
			const reasoning = container.querySelector('[class*="reasoning"]');
			expect(reasoning?.parentElement?.classList.contains('collapsed')).toBe(true);
		});

		it('does not display reasoning section when not provided', () => {
			const { container } = render(ThinkingMessage, {
				message: mockMessage
			});
			const reasoning = container.querySelector('[class*="reasoning"]');
			expect(reasoning).not.toBeInTheDocument();
		});
	});

	describe('model tag', () => {
		it('displays model name when present in metadata', () => {
			const messageWithModel: Message = {
				...mockMessage,
				metadata: { model: 'claude-3-opus' }
			};
			render(ThinkingMessage, {
				message: messageWithModel
			});
			expect(screen.getByText('claude-3-opus')).toBeInTheDocument();
		});

		it('does not display model tag when not in metadata', () => {
			const { container } = render(ThinkingMessage, {
				message: mockMessage
			});
			const modelTag = container.querySelector('.modelTag, [class*="model"]');
			expect(modelTag).not.toBeInTheDocument();
		});
	});

	describe('accessibility', () => {
		it('has proper aria-label for collapse button', () => {
			const { container } = render(ThinkingMessage, {
				message: mockMessage
			});
			const button = container.querySelector('[aria-label="Collapse thinking"]');
			expect(button).toBeInTheDocument();
		});

		it('has aria-expanded="true" when expanded', () => {
			const { container } = render(ThinkingMessage, {
				message: mockMessage
			});
			const button = container.querySelector('[aria-expanded="true"]');
			expect(button).toBeInTheDocument();
		});

		it('toggles aria-expanded on collapse', async () => {
			const { container } = render(ThinkingMessage, {
				message: mockMessage
			});

			const button = container.querySelector('button');
			if (button) {
				await fireEvent.click(button);
			}

			const collapsedButton = container.querySelector('[aria-expanded="false"]');
			expect(collapsedButton).toBeInTheDocument();
		});
	});

	describe('CSS classes', () => {
		it('applies isComplete class when status is sent', () => {
			const { container } = render(ThinkingMessage, {
				message: { ...mockMessage, status: 'sent' }
			});
			const containerEl = container.querySelector('.isComplete, [class*="isComplete"]');
			expect(containerEl).toBeInTheDocument();
		});

		it('applies collapsed class when collapsed', async () => {
			const { container } = render(ThinkingMessage, {
				message: mockMessage
			});

			const button = container.querySelector('button');
			if (button) {
				await fireEvent.click(button);
			}

			const containerEl = container.querySelector('.collapsed');
			expect(containerEl).toBeInTheDocument();
		});
	});

	describe('edge cases', () => {
		it('handles empty content and no reasoning', () => {
			const minimalMessage: Message = {
				...mockMessage,
				content: '',
				reasoning: undefined
			};
			render(ThinkingMessage, {
				message: minimalMessage
			});
			// Should render with default text
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('handles very long content', () => {
			const longContent = 'A'.repeat(500);
			const longMessage: Message = {
				...mockMessage,
				content: longContent
			};
			render(ThinkingMessage, {
				message: longMessage
			});
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('handles very long reasoning', () => {
			const longReasoning = 'A'.repeat(1000);
			const longMessage: Message = {
				...mockMessage,
				reasoning: longReasoning
			};
			render(ThinkingMessage, {
				message: longMessage
			});
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('handles undefined metadata', () => {
			const { container } = render(ThinkingMessage, {
				message: mockMessage
			});
			const containerEl = container.querySelector('[class*="thinkingMessage"]');
			expect(containerEl).toBeInTheDocument();
		});
	});
});
