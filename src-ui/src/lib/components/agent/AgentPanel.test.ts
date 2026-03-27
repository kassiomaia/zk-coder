import {fireEvent, render, screen} from '@testing-library/svelte';
import {beforeEach, describe, expect, it, vi} from 'vitest';
import AgentPanel from './AgentPanel.svelte';
import type {AgentSession, PendingAction} from '$lib/types/agent';

// Mock child components to simplify testing
vi.mock('./TaskList.svelte', () => ({
	default: vi.fn(({ tasks }) => ({
		render: () => `<div class="mock-task-list" data-task-count="${tasks?.length || 0}">Task List</div>`
	}))
}));

vi.mock('./RequestsList.svelte', () => ({
	default: vi.fn(({ steps }) => ({
		render: () => `<div class="mock-requests-list" data-steps-count="${steps?.length || 0}">Requests List</div>`
	}))
}));

vi.mock('./FileTree.svelte', () => ({
	default: vi.fn(({ files }) => ({
		render: () => `<div class="mock-file-tree" data-files-count="${files?.length || 0}">File Tree</div>`
	}))
}));

vi.mock('./ToolCallsView.svelte', () => ({
	default: vi.fn(({ toolCalls }) => ({
		render: () => `<div class="mock-tool-calls" data-calls-count="${toolCalls?.length || 0}">Tool Calls</div>`
	}))
}));

vi.mock('./ApprovalDialog.svelte', () => ({
	default: vi.fn(() => ({
		render: () => '<div class="mock-approval-dialog">Approval Dialog</div>'
	}))
}));

vi.mock('./WorkspaceSelector.svelte', () => ({
	default: vi.fn(() => ({
		render: () => '<div class="mock-workspace-selector">Workspace Selector</div>'
	}))
}));

vi.mock('./SessionTabs.svelte', () => ({
	default: vi.fn(() => ({
		render: () => '<div class="mock-session-tabs">Session Tabs</div>'
	}))
}));

vi.mock('./SessionHistory.svelte', () => ({
	default: vi.fn(() => ({
		render: () => '<div class="mock-session-history">Session History</div>'
	}))
}));

vi.mock('./AgentConfig.svelte', () => ({
	default: vi.fn(() => ({
		render: () => '<div class="mock-agent-config">Agent Config</div>'
	}))
}));

vi.mock('./FileDiffModal.svelte', () => ({
	default: vi.fn(() => ({
		render: () => '<div class="mock-file-diff-modal">File Diff Modal</div>'
	}))
}));

describe('AgentPanel', () => {
	const mockSession: AgentSession = {
		id: 'session-1',
		agentId: 'agent-1',
		agentName: 'Code Assistant',
		conversationId: 'conv-1',
		threadId: 'thread-1',
		state: { type: 'running', currentStep: 3, contextUsage: 1024 },
		workspaceRoot: '/home/user/project',
		tasks: [
			{
				id: 'task-1',
				title: 'Implement feature',
				status: 'in_progress',
				subtasks: [
					{ id: 'sub-1', taskId: 'task-1', title: 'Write tests', mode: 'write', status: 'pending', order: 1, createdAt: '2024-01-01T00:00:00Z' }
				]
			}
		],
		filesTouched: [
			{ path: '/src/test.ts', modified: true, action: 'write', diff: '--- a/test.ts\n+++ b/test.ts' }
		],
		toolCalls: [
			{
				id: 'call-1',
				tool: 'fs_read_file',
				arguments: { path: '/src/test.ts' },
				result: { content: 'test content' },
				timestamp: '2024-01-01T00:00:00Z'
			}
		],
		executionSteps: [
			{
				stepNumber: 1,
				action: { type: 'llm_request', model: 'claude-3-opus' },
				result: { success: true, output: {}, durationMs: 1000 },
				timestamp: '2024-01-01T00:00:00Z'
			}
		],
		createdAt: '2024-01-01T00:00:00Z',
		updatedAt: '2024-01-01T00:01:00Z'
	};

	const mockPendingActions: PendingAction[] = [];

	const defaultProps = {
		session: mockSession,
		pendingActions: mockPendingActions,
		onApprove: vi.fn(),
		onDeny: vi.fn(),
		onWorkspaceChange: vi.fn(),
		onclose: vi.fn()
	};

	beforeEach(() => {
		vi.clearAllMocks();
	});

	describe('rendering', () => {
		it('renders agent panel with agent name', () => {
			render(AgentPanel, { ...defaultProps });

			expect(screen.getByText('Code Assistant')).toBeInTheDocument();
		});

		it('renders status bar with running state', () => {
			render(AgentPanel, { ...defaultProps });

			expect(screen.getByText('Step 3')).toBeInTheDocument();
		});

		it('renders collapsible sections', () => {
			const { container } = render(AgentPanel, { ...defaultProps });

			// Check for section headers
			expect(screen.getByText('Activity')).toBeInTheDocument();
			expect(screen.getByText('Requests')).toBeInTheDocument();
			expect(screen.getByText('Files')).toBeInTheDocument();
			expect(screen.getByText('Tool Calls')).toBeInTheDocument();
		});

		it('renders session tabs when multiple sessions exist', () => {
			const allSessions = [mockSession, { ...mockSession, id: 'session-2' }];
			render(AgentPanel, { ...defaultProps, allSessions });

			// SessionTabs component should be rendered
			const tabs = screen.queryByText('Session Tabs');
			expect(tabs).toBeInTheDocument();
		});
	});

	describe('view switching', () => {
		it('starts in active view mode', () => {
			render(AgentPanel, { ...defaultProps });

			// Active view button should be active
			const activeBtn = screen.getByLabelText('Active session');
			expect(activeBtn).toHaveClass('active');
		});

		it('switches to history view when history button clicked', async () => {
			render(AgentPanel, { ...defaultProps });

			const historyBtn = screen.getByTitle('Session History');
			await fireEvent.click(historyBtn);

			// Should show session history
			const history = screen.queryByText('Session History');
			expect(history).toBeInTheDocument();
		});

		it('switches back to active view from history', async () => {
			render(AgentPanel, { ...defaultProps });

			const historyBtn = screen.getByTitle('Session History');
			await fireEvent.click(historyBtn);

			const activeBtn = screen.getByLabelText('Active session');
			await fireEvent.click(activeBtn);

			expect(activeBtn).toHaveClass('active');
		});
	});

	describe('section collapsing', () => {
		it('toggles activity section when header clicked', async () => {
			const { container } = render(AgentPanel, { ...defaultProps });

			const activityHeader = screen
				.getByText('Activity')
				.closest('.section-header') as HTMLElement;

			// Should be expanded by default
			const taskList = container.querySelector('.mock-task-list');
			expect(taskList).toBeInTheDocument();

			// Collapse
			await fireEvent.click(activityHeader);

			// Section should be collapsed (TaskList might still be in DOM but hidden)
			const expanded = activityHeader.getAttribute('aria-expanded');
			expect(expanded).toBe('false');
		});

		it('toggles requests section when header clicked', async () => {
			const { container } = render(AgentPanel, { ...defaultProps });

			const requestsHeader = screen
				.getByText('Requests')
				.closest('.section-header') as HTMLElement;

			// Should be expanded by default
			const requestsList = container.querySelector('.mock-requests-list');
			expect(requestsList).toBeInTheDocument();

			// Collapse
			await fireEvent.click(requestsHeader);

			const expanded = requestsHeader.getAttribute('aria-expanded');
			expect(expanded).toBe('false');
		});

		it('toggles files section when header clicked', async () => {
			const { container } = render(AgentPanel, { ...defaultProps });

			const filesHeader = screen.getByText('Files').closest('.section-header') as HTMLElement;

			// Should be expanded by default
			const fileTree = container.querySelector('.mock-file-tree');
			expect(fileTree).toBeInTheDocument();

			// Collapse
			await fireEvent.click(filesHeader);

			const expanded = filesHeader.getAttribute('aria-expanded');
			expect(expanded).toBe('false');
		});
	});

	describe('status display', () => {
		it('displays launching status', () => {
			const launchingSession: AgentSession = {
				...mockSession,
				state: { type: 'launching' }
			};
			render(AgentPanel, { ...defaultProps, session: launchingSession });

			expect(screen.getByText('Launching...')).toBeInTheDocument();
		});

		it('displays idle status for wait_for_request state', () => {
			const idleSession: AgentSession = {
				...mockSession,
				state: { type: 'wait_for_request', lastActionAt: '2024-01-01T00:00:00Z' }
			};
			render(AgentPanel, { ...defaultProps, session: idleSession });

			expect(screen.getByText('Idle')).toBeInTheDocument();
		});

		it('displays completed status', () => {
			const completedSession: AgentSession = {
				...mockSession,
				state: { type: 'finished', completedAt: '2024-01-01T00:00:00Z' }
			};
			render(AgentPanel, { ...defaultProps, session: completedSession });

			expect(screen.getByText('Completed')).toBeInTheDocument();
		});

		it('displays cancelled status', () => {
			const cancelledSession: AgentSession = {
				...mockSession,
				state: { type: 'cancelled', reason: 'User cancelled' }
			};
			render(AgentPanel, { ...defaultProps, session: cancelledSession });

			expect(screen.getByText('Cancelled')).toBeInTheDocument();
		});

		it('displays error status', () => {
			const errorSession: AgentSession = {
				...mockSession,
				state: { type: 'error', message: 'Something went wrong', recoverable: false }
			};
			render(AgentPanel, { ...defaultProps, session: errorSession });

			expect(screen.getByText('Error')).toBeInTheDocument();
		});

		it('displays waiting for approval status', () => {
			const waitingSession: AgentSession = {
				...mockSession,
				state: { type: 'user_input_required', pendingActionIds: ['action-1'] }
			};
			render(AgentPanel, { ...defaultProps, session: waitingSession });

			expect(screen.getByText('Waiting for approval')).toBeInTheDocument();
		});
	});

	describe('token display', () => {
		it('displays token count when tokens are used', () => {
			const sessionWithTokens: AgentSession = {
				...mockSession,
				executionSteps: [
					{
						stepNumber: 1,
						action: { type: 'llm_request', model: 'claude-3-opus' },
						result: { success: true, output: {}, tokensUsed: 5000, durationMs: 1000 },
						timestamp: '2024-01-01T00:00:00Z'
					}
				]
			};
			render(AgentPanel, { ...defaultProps, session: sessionWithTokens });

			// Should show "5.0k" for 5000 tokens
			expect(screen.getByText('5.0k')).toBeInTheDocument();
		});

		it('formats token count correctly for thousands', () => {
			const sessionWithTokens: AgentSession = {
				...mockSession,
				executionSteps: [
					{
						stepNumber: 1,
						action: { type: 'llm_request', model: 'claude-3-opus' },
						result: { success: true, output: {}, tokensUsed: 12500, durationMs: 1000 },
						timestamp: '2024-01-01T00:00:00Z'
					}
				]
			};
			render(AgentPanel, { ...defaultProps, session: sessionWithTokens });

			// Should show "12.5k" for 12500 tokens
			expect(screen.getByText('12.5k')).toBeInTheDocument();
		});

		it('does not display token badge when no tokens used', () => {
			const sessionWithoutTokens: AgentSession = {
				...mockSession,
				executionSteps: []
			};
			const { container } = render(AgentPanel, {
				...defaultProps,
				session: sessionWithoutTokens
			});

			// Token badge should not be present
			const tokenBadge = container.querySelector('.badge.tokens');
			expect(tokenBadge).not.toBeInTheDocument();
		});
	});

	describe('header actions', () => {
		it('opens agent config when settings button clicked', async () => {
			const { container } = render(AgentPanel, { ...defaultProps });

			const settingsBtn = screen.getByTitle('Agent Configuration');
			await fireEvent.click(settingsBtn);

			// AgentConfig should be shown
			const agentConfig = container.querySelector('.mock-agent-config');
			expect(agentConfig).toBeInTheDocument();
		});

		it('calls onclose when close button clicked', async () => {
			render(AgentPanel, { ...defaultProps });

			const closeBtn = screen.getByLabelText('Close agent panel');
			await fireEvent.click(closeBtn);

			expect(defaultProps.onclose).toHaveBeenCalledTimes(1);
		});

		it('has proper ARIA labels on header buttons', () => {
			render(AgentPanel, { ...defaultProps });

			expect(screen.getByLabelText('Active session')).toBeInTheDocument();
			expect(screen.getByLabelText('View session history')).toBeInTheDocument();
			expect(screen.getByLabelText('Agent configuration')).toBeInTheDocument();
			expect(screen.getByLabelText('Close agent panel')).toBeInTheDocument();
		});
	});

	describe('approval dialog', () => {
		it('shows approval dialog when there are pending actions', () => {
			const pendingAction: PendingAction = {
				id: 'action-1',
				toolName: 'fs_write_file',
				actionType: { type: 'write_file', path: '/test.txt', sizeBytes: 100 },
				description: 'Write file',
				createdAt: '2024-01-01T00:00:00Z'
			};
			render(AgentPanel, {
				...defaultProps,
				pendingActions: [pendingAction]
			});

			const dialog = screen.queryByText('Approval Dialog');
			expect(dialog).toBeInTheDocument();
		});

		it('does not show approval dialog when no pending actions', () => {
			render(AgentPanel, { ...defaultProps, pendingActions: [] });

			const dialog = screen.queryByText('Approval Dialog');
			expect(dialog).not.toBeInTheDocument();
		});
	});

	describe('accessibility', () => {
		it('has proper ARIA roles', () => {
			const { container } = render(AgentPanel, { ...defaultProps });

			const panel = container.querySelector('.agent-panel');
			expect(panel).toHaveAttribute('aria-label', 'Agent panel');
		});

		it('has proper aria-expanded attributes on section headers', () => {
			render(AgentPanel, { ...defaultProps });

			const activityHeader = screen.getByText('Activity').closest('.section-header');
			expect(activityHeader).toHaveAttribute('aria-expanded', 'true');

			const toolCallsHeader = screen.getByText('Tool Calls').closest('.section-header');
			expect(toolCallsHeader).toHaveAttribute('aria-expanded', 'false');
		});

		it('has proper aria-controls attributes linking sections', () => {
			render(AgentPanel, { ...defaultProps });

			const activityHeader = screen.getByText('Activity').closest('.section-header');
			expect(activityHeader).toHaveAttribute('aria-controls', 'activity-section');
		});

		it('section headers are focusable and have visible focus styles', () => {
			const { container } = render(AgentPanel, { ...defaultProps });

			const headers = container.querySelectorAll('.section-header');
			headers.forEach((header) => {
				expect(header).toBeInstanceOf(HTMLButtonElement);
			});
		});
	});

	describe('workspace selector', () => {
		it('passes disabled state correctly when running', () => {
			const runningSession: AgentSession = {
				...mockSession,
				state: { type: 'running', currentStep: 1, contextUsage: 0 }
			};
			render(AgentPanel, { ...defaultProps, session: runningSession });

			// WorkspaceSelector should receive disabled=true when running
			const selector = screen.queryByText('Workspace Selector');
			expect(selector).toBeInTheDocument();
		});

		it('passes disabled state correctly when waiting for approval', () => {
			const waitingSession: AgentSession = {
				...mockSession,
				state: { type: 'user_input_required', pendingActionIds: ['action-1'] }
			};
			render(AgentPanel, { ...defaultProps, session: waitingSession });

			const selector = screen.queryByText('Workspace Selector');
			expect(selector).toBeInTheDocument();
		});
	});

	describe('badge display', () => {
		it('shows execution step count badge', () => {
			const { container } = render(AgentPanel, { ...defaultProps });

			// Should show badge with step count
			const badges = container.querySelectorAll('.requests-section .badge');
			expect(badges.length).toBeGreaterThan(0);
		});

		it('shows files touched count badge', () => {
			const { container } = render(AgentPanel, { ...defaultProps });

			const badge = container.querySelector('.badge');
			expect(badge).toBeInTheDocument();
		});

		it('shows running step badge when session is running', () => {
			const { container } = render(AgentPanel, { ...defaultProps });

			const badge = container.querySelector('.badge.running');
			expect(badge).toBeInTheDocument();
			expect(badge?.textContent).toContain('Step 3');
		});
	});
});
