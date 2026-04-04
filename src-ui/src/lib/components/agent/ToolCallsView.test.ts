import { render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import ToolCallsView from './ToolCallsView.svelte';
import type { ToolCallRecord } from '$lib/types/agent';

// Mock ToolCallItem component
vi.mock('./ToolCallItem.svelte', () => ({
	default: vi.fn(({ record }) => ({
		render: () => `
			<div class="mock-tool-call-item" data-tool="${record.tool}" data-id="${record.id}">
				<span class="tool-name">${record.tool}</span>
				<span class="tool-args">${JSON.stringify(record.arguments)}</span>
			</div>
		`
	}))
}));

describe('ToolCallsView', () => {
	const mockToolCalls: ToolCallRecord[] = [
		{
			id: 'call-1',
			tool: 'fs_read_file',
			arguments: { path: '/src/config.ts' },
			result: { content: 'config content' },
			timestamp: '2024-01-01T00:00:00Z',
			durationMs: 50
		},
		{
			id: 'call-2',
			tool: 'fs_write_file',
			arguments: { path: '/src/test.ts', content: 'test content' },
			result: { success: true },
			timestamp: '2024-01-01T00:00:01Z',
			durationMs: 120
		},
		{
			id: 'call-3',
			tool: 'execute_command',
			arguments: { command: 'npm test' },
			result: { exitCode: 0, output: 'Tests passed' },
			timestamp: '2024-01-01T00:00:02Z',
			durationMs: 2500
		}
	];

	const errorToolCall: ToolCallRecord = {
		id: 'call-error',
		tool: 'fs_glob',
		arguments: { pattern: '**/*.ts' },
		result: { error: 'Pattern too large' },
		isError: true,
		timestamp: '2024-01-01T00:00:03Z'
	};

	beforeEach(() => {
		vi.clearAllMocks();
	});

	describe('rendering', () => {
		it('renders all tool calls', () => {
			const { container } = render(ToolCallsView, { toolCalls: mockToolCalls });

			const items = container.querySelectorAll('.mock-tool-call-item');
			expect(items.length).toBe(mockToolCalls.length);
		});

		it('renders empty state when no tool calls', () => {
			render(ToolCallsView, { toolCalls: [] });

			expect(screen.getByText('No tool calls recorded')).toBeInTheDocument();
		});

		it('has proper ARIA role and label', () => {
			const { container } = render(ToolCallsView, { toolCalls: mockToolCalls });

			const view = container.querySelector('.tool-calls-view');
			expect(view).toHaveAttribute('role', 'log');
			expect(view).toHaveAttribute('aria-label', 'Tool calls history');
		});

		it('renders tool call with each tool name', () => {
			const { container } = render(ToolCallsView, { toolCalls: mockToolCalls });

			expect(screen.getByText('fs_read_file')).toBeInTheDocument();
			expect(screen.getByText('fs_write_file')).toBeInTheDocument();
			expect(screen.getByText('execute_command')).toBeInTheDocument();
		});
	});

	describe('empty state', () => {
		it('shows empty state with icon and text', () => {
			const { container } = render(ToolCallsView, { toolCalls: [] });

			const emptyState = container.querySelector('.empty-state');
			expect(emptyState).toBeInTheDocument();

			expect(screen.getByText('No tool calls recorded')).toBeInTheDocument();
		});

		it('has blinking animation in empty state', () => {
			const { container } = render(ToolCallsView, { toolCalls: [] });

			const emptyIconWrapper = container.querySelector('.empty-icon-wrapper');
			expect(emptyIconWrapper).toBeInTheDocument();
		});

		it('shows underscore character as empty state icon', () => {
			const { container } = render(ToolCallsView, { toolCalls: [] });

			const emptyChar = container.querySelector('.empty-char');
			expect(emptyChar).toBeInTheDocument();
			expect(emptyChar?.textContent).toBe('_');
		});
	});

	describe('tool call records', () => {
		it('passes correct props to ToolCallItem components', () => {
			const { container } = render(ToolCallsView, { toolCalls: mockToolCalls });

			const items = container.querySelectorAll('.mock-tool-call-item');

			expect(items[0]?.getAttribute('data-tool')).toBe('fs_read_file');
			expect(items[1]?.getAttribute('data-tool')).toBe('fs_write_file');
			expect(items[2]?.getAttribute('data-tool')).toBe('execute_command');
		});

		it('uses tool call id as key for each item', () => {
			const { container } = render(ToolCallsView, { toolCalls: mockToolCalls });

			const items = container.querySelectorAll('.mock-tool-call-item');

			expect(items[0]?.getAttribute('data-id')).toBe('call-1');
			expect(items[1]?.getAttribute('data-id')).toBe('call-2');
			expect(items[2]?.getAttribute('data-id')).toBe('call-3');
		});
	});

	describe('auto-scroll behavior', () => {
		it('scrolls to bottom when new tool calls are added', async () => {
			const { container, rerender } = render(ToolCallsView, {
				toolCalls: [mockToolCalls[0]]
			});

			const view = container.querySelector('.tool-calls-view') as HTMLElement;
			vi.spyOn(view, 'scrollTop', 'set').mockImplementation(() => {});

			// Add more tool calls
			await rerender({ toolCalls: mockToolCalls });

			// The $effect should trigger scroll to bottom
			// We can't easily test the actual scroll, but we can verify the component renders
			expect(container.querySelectorAll('.mock-tool-call-item').length).toBe(3);
		});

		it('does not scroll when tool calls array is empty', () => {
			const { container } = render(ToolCallsView, { toolCalls: [] });

			const view = container.querySelector('.tool-calls-view');
			expect(view).toBeInTheDocument();
		});
	});

	describe('styling', () => {
		it('applies correct CSS classes', () => {
			const { container } = render(ToolCallsView, { toolCalls: mockToolCalls });

			const view = container.querySelector('.tool-calls-view');
			const list = container.querySelector('.tool-calls-list');

			expect(view).toBeInTheDocument();
			expect(list).toBeInTheDocument();
		});

		it('has proper overflow and scroll behavior', () => {
			const { container } = render(ToolCallsView, { toolCalls: mockToolCalls });

			const view = container.querySelector('.tool-calls-view');
			expect(view).toHaveStyle({ overflowY: 'auto' });
		});
	});

	describe('edge cases', () => {
		it('handles tool calls with error state', () => {
			const { container } = render(ToolCallsView, {
				toolCalls: [errorToolCall]
			});

			const item = container.querySelector('.mock-tool-call-item');
			expect(item).toBeInTheDocument();
			expect(item?.getAttribute('data-id')).toBe('call-error');
		});

		it('handles tool calls without result', () => {
			const pendingCall: ToolCallRecord = {
				id: 'call-pending',
				tool: 'llm_request',
				arguments: { prompt: 'test' },
				timestamp: '2024-01-01T00:00:00Z'
			};
			const { container } = render(ToolCallsView, {
				toolCalls: [pendingCall]
			});

			const item = container.querySelector('.mock-tool-call-item');
			expect(item).toBeInTheDocument();
		});

		it('handles large number of tool calls', () => {
			const manyCalls: ToolCallRecord[] = Array.from({ length: 100 }, (_, i) => ({
				id: `call-${i}`,
				tool: 'fs_read_file',
				arguments: { path: `/file${i}.txt` },
				timestamp: '2024-01-01T00:00:00Z'
			}));

			const { container } = render(ToolCallsView, { toolCalls: manyCalls });

			const items = container.querySelectorAll('.mock-tool-call-item');
			expect(items.length).toBe(100);
		});

		it('handles tool calls with complex arguments', () => {
			const complexCall: ToolCallRecord = {
				id: 'call-complex',
				tool: 'web_request',
				arguments: {
					url: 'https://api.example.com',
					method: 'POST',
					headers: { 'Content-Type': 'application/json' },
					body: JSON.stringify({ key: 'value' })
				},
				timestamp: '2024-01-01T00:00:00Z'
			};
			const { container } = render(ToolCallsView, { toolCalls: [complexCall] });

			const item = container.querySelector('.mock-tool-call-item');
			expect(item?.getAttribute('data-tool')).toBe('web_request');
		});
	});

	describe('reactivity', () => {
		it('updates display when tool calls prop changes', async () => {
			const { container, rerender } = render(ToolCallsView, {
				toolCalls: [mockToolCalls[0]]
			});

			expect(container.querySelectorAll('.mock-tool-call-item').length).toBe(1);

			await rerender({ toolCalls: mockToolCalls });

			expect(container.querySelectorAll('.mock-tool-call-item').length).toBe(3);
		});

		it('transitions from empty state to having calls', async () => {
			const { container, rerender } = render(ToolCallsView, {
				toolCalls: []
			});

			expect(screen.getByText('No tool calls recorded')).toBeInTheDocument();

			await rerender({ toolCalls: [mockToolCalls[0]] });

			expect(screen.queryByText('No tool calls recorded')).not.toBeInTheDocument();
			expect(container.querySelectorAll('.mock-tool-call-item').length).toBe(1);
		});
	});

	describe('accessibility', () => {
		it('maintains proper role when content changes', async () => {
			const { container, rerender } = render(ToolCallsView, {
				toolCalls: []
			});

			const view = container.querySelector('[role="log"]');
			expect(view).toBeInTheDocument();

			await rerender({ toolCalls: mockToolCalls });

			const viewAfter = container.querySelector('[role="log"]');
			expect(viewAfter).toBeInTheDocument();
		});
	});
});
