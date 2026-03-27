import {fireEvent, render, screen} from '@testing-library/svelte';
import {afterEach, beforeEach, describe, expect, it, vi} from 'vitest';
import ToolCallMessage from './ToolCallMessage.svelte';
import type {Message} from '$lib/types/message';

describe('ToolCallMessage', () => {
	let mockMessage: Message;

	beforeEach(() => {
		vi.clearAllMocks();
		mockMessage = {
			id: 'msg-1',
			conversationId: 'conv-1',
			threadId: 'thread-1',
			role: 'assistant',
			content: 'Reading file',
			timestamp: new Date('2024-01-15T10:00:00'),
			status: 'sent',
			kind: 'tool_call',
			agentData: {
				toolName: 'fs_read_file',
				toolArgs: { path: '/path/to/file.txt' },
				toolCallStatus: 'success',
				toolResult: 'File content here'
			}
		};
	});

	afterEach(() => {
		vi.restoreAllMocks();
	});

	describe('rendering', () => {
		it('renders the tool call container', () => {
			const { container } = render(ToolCallMessage, {
				message: mockMessage
			});
			const containerEl = container.querySelector('[class*="toolCall"]');
			expect(containerEl).toBeInTheDocument();
		});

		it('displays the tool name', () => {
			render(ToolCallMessage, {
				message: mockMessage
			});
			expect(screen.getByText('fs_read_file')).toBeInTheDocument();
		});

		it('renders the header button', () => {
			const { container } = render(ToolCallMessage, {
				message: mockMessage
			});
			const button = container.querySelector('button');
			expect(button).toBeInTheDocument();
		});

	});

	describe('tool icons', () => {
		it('renders file icon for fs_read_file', () => {
			const { container } = render(ToolCallMessage, {
				message: mockMessage
			});
			const icon = container.querySelector('svg');
			expect(icon).toBeInTheDocument();
		});

		it('renders terminal icon for execute_command', () => {
			const terminalMessage: Message = {
				...mockMessage,
				agentData: {
					toolName: 'execute_command',
					toolArgs: { command: 'ls -la' },
					toolCallStatus: 'success'
				}
			};
			const { container } = render(ToolCallMessage, {
				message: terminalMessage
			});
			const icon = container.querySelector('svg');
			expect(icon).toBeInTheDocument();
		});

		it('renders web icon for web_request', () => {
			const webMessage: Message = {
				...mockMessage,
				agentData: {
					toolName: 'web_request',
					toolArgs: { url: 'https://example.com' },
					toolCallStatus: 'pending'
				}
			};
			const { container } = render(ToolCallMessage, {
				message: webMessage
			});
			const icon = container.querySelector('svg');
			expect(icon).toBeInTheDocument();
		});
	});

	describe('status indicators', () => {
		it('shows spinner for pending status', () => {
			const pendingMessage: Message = {
				...mockMessage,
				agentData: {
					toolName: 'fs_read_file',
					toolCallStatus: 'pending',
					toolArgs: { path: '/test.txt' }
				}
			};
			const { container } = render(ToolCallMessage, {
				message: pendingMessage
			});
			const spinner = container.querySelector('.spinner, [class*="spinner"]');
			expect(spinner).toBeInTheDocument();
		});

		it('shows check icon for success status', () => {
			const { container } = render(ToolCallMessage, {
				message: mockMessage
			});
			const statusIcon = container.querySelector('.statusIcon, [class*="status"]');
			expect(statusIcon).toBeInTheDocument();
		});

		it('shows alert icon for error status', () => {
			const errorMessage: Message = {
				...mockMessage,
				agentData: {
					toolName: 'fs_read_file',
					toolCallStatus: 'error',
					toolArgs: { path: '/test.txt' }
				}
			};
			const { container } = render(ToolCallMessage, {
				message: errorMessage
			});
			const errorIcon = container.querySelector('.error, [class*="error"]');
			expect(errorIcon).toBeInTheDocument();
		});

		it('applies hasError class when status is error', () => {
			const errorMessage: Message = {
				...mockMessage,
				agentData: {
					toolName: 'fs_read_file',
					toolCallStatus: 'error',
					toolArgs: { path: '/test.txt' }
				}
			};
			const { container } = render(ToolCallMessage, {
				message: errorMessage
			});
			const containerEl = container.querySelector('[class*="toolCall"] [class*="error"]');
			expect(containerEl).toBeInTheDocument();
		});
	});

	describe('expand/collapse', () => {
		it('is collapsed by default', () => {
			const { container } = render(ToolCallMessage, {
				message: mockMessage
			});
			const containerEl = container.querySelector('[class*="toolCall"]');
			expect(containerEl?.classList.contains('expanded')).toBe(false);
		});

		it('expands when header is clicked', async () => {
			const { container } = render(ToolCallMessage, {
				message: mockMessage
			});

			const button = container.querySelector('button');
			if (button) {
				await fireEvent.click(button);
			}

			// With CSS modules, the expanded class is scoped (e.g., ToolCallMessage_expanded_xyz)
			const containerEl = container.querySelector('[class*="toolCall"]');
			expect(containerEl?.className).toContain('expanded');
		});

		it('toggles collapse on second click', async () => {
			const { container } = render(ToolCallMessage, {
				message: mockMessage
			});

			const button = container.querySelector('button');
			if (button) {
				await fireEvent.click(button);
				await fireEvent.click(button);
			}

			const containerEl = container.querySelector('[class*="toolCall"]');
			expect(containerEl?.classList.contains('expanded')).toBe(false);
		});
	});

	describe('expanded content', () => {
		it('displays arguments section when expanded', async () => {
			const { container } = render(ToolCallMessage, {
				message: mockMessage
			});

			const button = container.querySelector('button');
			if (button) {
				await fireEvent.click(button);
			}

			expect(screen.getByText('Arguments')).toBeInTheDocument();
		});

		it('displays result section when tool has result', async () => {
			const { container } = render(ToolCallMessage, {
				message: mockMessage
			});

			const button = container.querySelector('button');
			if (button) {
				await fireEvent.click(button);
			}

			expect(screen.getByText('Result')).toBeInTheDocument();
			expect(screen.getByText('File content here')).toBeInTheDocument();
		});

		it('displays exit code when provided', async () => {
			const exitCodeMessage: Message = {
				...mockMessage,
				agentData: {
					...mockMessage.agentData,
					exitCode: 0
				}
			};

			const { container } = render(ToolCallMessage, {
				message: exitCodeMessage
			});

			const button = container.querySelector('button');
			if (button) {
				await fireEvent.click(button);
			}

			expect(screen.getByText('Exit code: 0')).toBeInTheDocument();
		});

		it('displays timestamp footer', async () => {
			const { container } = render(ToolCallMessage, {
				message: mockMessage
			});

			const button = container.querySelector('button');
			if (button) {
				await fireEvent.click(button);
			}

			expect(screen.getByText(/Executed at/)).toBeInTheDocument();
		});
	});

	describe('primary argument display', () => {
		it('displays path argument when present', () => {
			render(ToolCallMessage, {
				message: mockMessage
			});
			expect(screen.getByText('/path/to/file.txt')).toBeInTheDocument();
		});

		it('displays pattern argument when present', () => {
			const patternMessage: Message = {
				...mockMessage,
				agentData: {
					toolName: 'fs_glob',
					toolArgs: { pattern: '**/*.ts' },
					toolCallStatus: 'pending'
				}
			};
			render(ToolCallMessage, {
				message: patternMessage
			});
			expect(screen.getByText('**/*.ts')).toBeInTheDocument();
		});

		it('displays command argument when present', () => {
			const commandMessage: Message = {
				...mockMessage,
				agentData: {
					toolName: 'execute_command',
					toolArgs: { command: 'npm test' },
					toolCallStatus: 'pending'
				}
			};
			render(ToolCallMessage, {
				message: commandMessage
			});
			expect(screen.getByText('npm test')).toBeInTheDocument();
		});

		it('displays url argument when present', () => {
			const urlMessage: Message = {
				...mockMessage,
				agentData: {
					toolName: 'web_request',
					toolArgs: { url: 'https://example.com' },
					toolCallStatus: 'pending'
				}
			};
			render(ToolCallMessage, {
				message: urlMessage
			});
			expect(screen.getByText('https://example.com')).toBeInTheDocument();
		});
	});

	describe('accessibility', () => {
		it('has proper aria-label on header button', () => {
			const { container } = render(ToolCallMessage, {
				message: mockMessage
			});
			const button = container.querySelector('[aria-label="Expand tool call"]');
			expect(button).toBeInTheDocument();
		});

		it('toggles aria-label on expand', async () => {
			const { container } = render(ToolCallMessage, {
				message: mockMessage
			});

			const button = container.querySelector('button');
			if (button) {
				await fireEvent.click(button);
			}

			const collapseButton = container.querySelector('[aria-label="Collapse tool call"]');
			expect(collapseButton).toBeInTheDocument();
		});

		it('has aria-expanded attribute', () => {
			const { container } = render(ToolCallMessage, {
				message: mockMessage
			});
			const button = container.querySelector('[aria-expanded="false"]');
			expect(button).toBeInTheDocument();
		});
	});

	describe('chevron icon', () => {
		it('shows chevron down when collapsed', () => {
			const { container } = render(ToolCallMessage, {
				message: mockMessage
			});
			const chevron = container.querySelector('[class*="chevron"]');
			expect(chevron).toBeInTheDocument();
		});

		it('rotates chevron when expanded', async () => {
			const { container } = render(ToolCallMessage, {
				message: mockMessage
			});

			const button = container.querySelector('button');
			if (button) {
				await fireEvent.click(button);
			}

			const rotatedChevron = container.querySelector('.rotated');
			expect(rotatedChevron).toBeInTheDocument();
		});
	});

	describe('edge cases', () => {
		it('handles tool call without agentData', () => {
			const noDataMessage: Message = {
				...mockMessage,
				agentData: undefined
			};
			const { container } = render(ToolCallMessage, {
				message: noDataMessage
			});
			const containerEl = container.querySelector('[class*="toolCall"]');
			expect(containerEl).toBeInTheDocument();
		});

		it('handles unknown tool name', () => {
			const unknownToolMessage: Message = {
				...mockMessage,
				agentData: {
					toolName: 'unknown_tool',
					toolArgs: {},
					toolCallStatus: 'pending'
				}
			};
			render(ToolCallMessage, {
				message: unknownToolMessage
			});
			expect(screen.getByText('unknown_tool')).toBeInTheDocument();
		});

		it('handles empty tool args', () => {
			const emptyArgsMessage: Message = {
				...mockMessage,
				agentData: {
					toolName: 'fs_read_file',
					toolArgs: {},
					toolCallStatus: 'pending'
				}
			};
			const { container } = render(ToolCallMessage, {
				message: emptyArgsMessage
			});
			const containerEl = container.querySelector('[class*="toolCall"]');
			expect(containerEl).toBeInTheDocument();
		});

		it('handles very long tool result', () => {
			const longResult = 'A'.repeat(10000);
			const longResultMessage: Message = {
				...mockMessage,
				agentData: {
					...mockMessage.agentData,
					toolResult: longResult
				}
			};
			const { container } = render(ToolCallMessage, {
				message: longResultMessage
			});
			const containerEl = container.querySelector('[class*="toolCall"]');
			expect(containerEl).toBeInTheDocument();
		});
	});
});
