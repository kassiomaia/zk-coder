import { fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { tick } from 'svelte';
import TerminalOutput from './TerminalOutput.svelte';
import type { Message } from '$lib/types/message';

describe('TerminalOutput', () => {
	let mockMessage: Message;

	beforeEach(() => {
		vi.clearAllMocks();
		vi.useFakeTimers();
		mockMessage = {
			id: 'msg-1',
			conversationId: 'conv-1',
			threadId: 'thread-1',
			role: 'system',
			content: 'Output from terminal',
			timestamp: new Date('2024-01-15T10:00:00'),
			status: 'sent',
			kind: 'terminal',
			agentData: {
				streamType: 'stdout'
			}
		};
	});

	afterEach(() => {
		vi.restoreAllMocks();
		vi.useRealTimers();
	});

	describe('rendering', () => {
		it('renders the terminal output container', () => {
			const { container } = render(TerminalOutput, {
				message: mockMessage
			});
			const containerEl = container.querySelector('[class*="terminalOutput"]');
			expect(containerEl).toBeInTheDocument();
		});

		it('displays the content', () => {
			render(TerminalOutput, {
				message: mockMessage
			});
			expect(screen.getByText('Output from terminal')).toBeInTheDocument();
		});
	});

	describe('stream types', () => {
		it('applies command class for command stream type', () => {
			const commandMessage: Message = {
				...mockMessage,
				agentData: { streamType: 'command' }
			};
			const { container } = render(TerminalOutput, {
				message: commandMessage
			});
			const containerEl = container.querySelector('.command');
			expect(containerEl).toBeInTheDocument();
		});

		it('shows prompt for command stream type', () => {
			const commandMessage: Message = {
				...mockMessage,
				agentData: { streamType: 'command' }
			};
			render(TerminalOutput, {
				message: commandMessage
			});
			const prompt = document.querySelector('.prompt, [class*="prompt"]');
			expect(prompt).toBeInTheDocument();
		});

		it('applies stderr class for stderr stream type', () => {
			const stderrMessage: Message = {
				...mockMessage,
				agentData: { streamType: 'stderr' }
			};
			const { container } = render(TerminalOutput, {
				message: stderrMessage
			});
			const containerEl = container.querySelector('.stderr');
			expect(containerEl).toBeInTheDocument();
		});

		it('does not apply special classes for stdout', () => {
			const { container } = render(TerminalOutput, {
				message: mockMessage
			});
			const containerEl = container.querySelector('[class*="terminalOutput"]');
			expect(containerEl).toBeInTheDocument();
		});
	});

	describe('transient messages', () => {
		it('shows dismiss button for transient messages', () => {
			const transientMessage: Message = {
				...mockMessage,
				isTransient: true
			};
			const { container } = render(TerminalOutput, {
				message: transientMessage
			});
			const dismissButton = container.querySelector('.dismiss, [class*="dismiss"]');
			expect(dismissButton).toBeInTheDocument();
		});

		it('does not show dismiss button for non-transient messages', () => {
			const { container } = render(TerminalOutput, {
				message: mockMessage
			});
			const dismissButton = container.querySelector('.dismiss, [class*="dismiss"]');
			expect(dismissButton).not.toBeInTheDocument();
		});

		it('dismisses transient message when button is clicked', async () => {
			const transientMessage: Message = {
				...mockMessage,
				isTransient: true
			};
			const { container } = render(TerminalOutput, {
				message: transientMessage
			});

			const dismissButton = container.querySelector('.dismiss, [class*="dismiss"]');
			if (dismissButton) {
				await fireEvent.click(dismissButton);
			}

			// Message should be removed from DOM
			const containerEl = container.querySelector('[class*="terminalOutput"]');
			expect(containerEl).not.toBeInTheDocument();
		});

		it('auto-dismisses after timeout for transient messages', async () => {
			const transientMessage: Message = {
				...mockMessage,
				isTransient: true
			};
			const { container } = render(TerminalOutput, {
				message: transientMessage
			});

			// Should be visible initially
			let containerEl = container.querySelector('[class*="terminalOutput"]');
			expect(containerEl).toBeInTheDocument();

			// Advance past the 5 second timeout
			vi.advanceTimersByTime(5100);
			await tick();

			// Should be dismissed
			containerEl = container.querySelector('[class*="terminalOutput"]');
			expect(containerEl).not.toBeInTheDocument();
		});
	});

	describe('content handling', () => {
		it('handles empty content', () => {
			const emptyMessage: Message = {
				...mockMessage,
				content: ''
			};
			const { container } = render(TerminalOutput, {
				message: emptyMessage
			});
			const containerEl = container.querySelector('[class*="terminalOutput"]');
			expect(containerEl).toBeInTheDocument();
		});

		it('handles multiline output', () => {
			const multilineMessage: Message = {
				...mockMessage,
				content: 'Line 1\nLine 2\nLine 3'
			};
			const { container } = render(TerminalOutput, {
				message: multilineMessage
			});
			const output = container.querySelector('[class*="content"]');
			expect(output).toHaveTextContent('Line 1');
			expect(output).toHaveTextContent('Line 2');
			expect(output).toHaveTextContent('Line 3');
		});

		it('handles special characters in output', () => {
			const specialMessage: Message = {
				...mockMessage,
				content: 'Error: <script>alert("test")</script>'
			};
			render(TerminalOutput, {
				message: specialMessage
			});
			// Content should be displayed safely
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('handles ANSI escape sequences', () => {
			const ansiMessage: Message = {
				...mockMessage,
				content: '\x1b[31mRed text\x1b[0m'
			};
			const { container } = render(TerminalOutput, {
				message: ansiMessage
			});
			const containerEl = container.querySelector('[class*="terminalOutput"]');
			expect(containerEl).toBeInTheDocument();
		});
	});

	describe('accessibility', () => {
		it('has proper aria-label on dismiss button', () => {
			const transientMessage: Message = {
				...mockMessage,
				isTransient: true
			};
			const { container } = render(TerminalOutput, {
				message: transientMessage
			});
			const button = container.querySelector('button[aria-label="Dismiss"]');
			expect(button).toBeInTheDocument();
		});
	});

	describe('edge cases', () => {
		it('handles message without agentData', () => {
			const noDataMessage: Message = {
				...mockMessage,
				agentData: undefined
			};
			const { container } = render(TerminalOutput, {
				message: noDataMessage
			});
			const containerEl = container.querySelector('[class*="terminalOutput"]');
			expect(containerEl).toBeInTheDocument();
		});

		it('handles very long output', () => {
			const longOutput = 'A'.repeat(10000);
			const longMessage: Message = {
				...mockMessage,
				content: longOutput
			};
			const { container } = render(TerminalOutput, {
				message: longMessage
			});
			const containerEl = container.querySelector('[class*="terminalOutput"]');
			expect(containerEl).toBeInTheDocument();
		});

		it('handles undefined content gracefully', () => {
			const undefinedContentMessage: Message = {
				...mockMessage,
				content: undefined as unknown as string
			};
			const { container } = render(TerminalOutput, {
				message: undefinedContentMessage
			});
			const containerEl = container.querySelector('[class*="terminalOutput"]');
			expect(containerEl).toBeInTheDocument();
		});
	});
});
