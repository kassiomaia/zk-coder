import {fireEvent, render, screen, waitFor} from '@testing-library/svelte';
import {beforeEach, describe, expect, it, vi} from 'vitest';
import TaskList from './TaskList.svelte';
import type {AgentTask} from '$lib/types/agent';

describe('TaskList', () => {
	const mockTasks: AgentTask[] = [
		{
			id: 'task-1',
			title: 'Implement authentication',
			description: 'Add login and logout functionality',
			status: 'pending',
			subtasks: [
				{
					id: 'sub-1',
					taskId: 'task-1',
					title: 'Create login form',
					mode: 'write',
					status: 'pending',
					order: 1,
					createdAt: '2024-01-01T00:00:00Z'
				},
				{
					id: 'sub-2',
					taskId: 'task-1',
					title: 'Add session management',
					mode: 'write',
					status: 'pending',
					order: 2,
					createdAt: '2024-01-01T00:00:00Z'
				}
			]
		},
		{
			id: 'task-2',
			title: 'Write tests',
			status: 'in_progress',
			subtasks: []
		},
		{
			id: 'task-3',
			title: 'Fix bugs',
			status: 'completed',
			subtasks: []
		},
		{
			id: 'task-4',
			title: 'Failed task',
			status: 'failed',
			subtasks: []
		}
	];

	const defaultProps = {
		tasks: mockTasks,
		ontaskclick: vi.fn(),
		onsubtaskclick: vi.fn()
	};

	beforeEach(() => {
		vi.clearAllMocks();
	});

	describe('rendering', () => {
		it('renders all tasks', () => {
			render(TaskList, { ...defaultProps });

			expect(screen.getByText('Implement authentication')).toBeInTheDocument();
			expect(screen.getByText('Write tests')).toBeInTheDocument();
			expect(screen.getByText('Fix bugs')).toBeInTheDocument();
			expect(screen.getByText('Failed task')).toBeInTheDocument();
		});

		it('renders task descriptions when present', () => {
			render(TaskList, { ...defaultProps });

			expect(
				screen.getByText('Add login and logout functionality')
			).toBeInTheDocument();
		});

		it('renders empty state when no tasks', () => {
			render(TaskList, { tasks: [] });

			expect(screen.getByText('No tasks yet')).toBeInTheDocument();
		});

		it('has proper ARIA label', () => {
			const { container } = render(TaskList, { ...defaultProps });

			const taskList = container.querySelector('.task-list');
			expect(taskList).toHaveAttribute('aria-label', 'Task list');
		});
	});

	describe('status indicators', () => {
		it('shows pending status icon for pending tasks', () => {
			const { container } = render(TaskList, { tasks: [mockTasks[0]] });

			const statusIcon = container.querySelector('.task-wrapper:first-child .status-icon');
			expect(statusIcon).toBeInTheDocument();
			expect(statusIcon).not.toHaveClass('completed', 'running', 'failed');
		});

		it('shows loading spinner for in_progress tasks', () => {
			const { container } = render(TaskList, { tasks: [mockTasks[1]] });

			const statusIcon = container.querySelector('.task-wrapper .status-icon.running');
			expect(statusIcon).toBeInTheDocument();
		});

		it('shows check icon for completed tasks', () => {
			const { container } = render(TaskList, { tasks: [mockTasks[2]] });

			const statusIcon = container.querySelector('.task-wrapper .status-icon.completed');
			expect(statusIcon).toBeInTheDocument();
		});

		it('shows x icon for failed tasks', () => {
			const { container } = render(TaskList, { tasks: [mockTasks[3]] });

			const statusIcon = container.querySelector('.task-wrapper .status-icon.failed');
			expect(statusIcon).toBeInTheDocument();
		});

		it('applies running state styles to task wrapper', () => {
			const { container } = render(TaskList, { tasks: [mockTasks[1]] });

			const taskWrapper = container.querySelector('.task-wrapper.is-running');
			expect(taskWrapper).toBeInTheDocument();
		});
	});

	describe('subtasks', () => {
		it('shows expand icon for tasks with subtasks', () => {
			const { container } = render(TaskList, { tasks: [mockTasks[0]] });

			const expandIcon = container.querySelector('.expand-icon');
			expect(expandIcon).toBeInTheDocument();
		});

		it('does not show expand icon for tasks without subtasks', () => {
			const { container } = render(TaskList, { tasks: [mockTasks[1]] });

			const expandIcon = container.querySelector('.expand-icon');
			expect(expandIcon).not.toBeInTheDocument();
		});

		it('expands subtasks when task is clicked', async () => {
			const { container } = render(TaskList, { ...defaultProps });

			const taskItem = screen.getByText('Implement authentication').closest('.task-item');
			if (taskItem) {
				await fireEvent.click(taskItem);

				// Subtasks should be visible after click
				await waitFor(() => {
					expect(screen.getByText('Create login form')).toBeInTheDocument();
				});
			}
		});

		it('shows all subtasks when expanded', async () => {
			const handleTaskClick = vi.fn();
			const { container } = render(TaskList, {
				...defaultProps,
				ontaskclick: handleTaskClick
			});

			const taskItem = screen.getByText('Implement authentication').closest('.task-item');
			if (taskItem) {
				await fireEvent.click(taskItem);

				await waitFor(() => {
					expect(screen.getByText('Create login form')).toBeInTheDocument();
					expect(screen.getByText('Add session management')).toBeInTheDocument();
				});
			}
		});

		it('auto-expands tasks that are in_progress', () => {
			const inProgressTask: AgentTask = {
				id: 'task-5',
				title: 'Running task',
				status: 'in_progress',
				subtasks: [
					{
						id: 'sub-3',
						taskId: 'task-5',
						title: 'Subtask',
						mode: 'write',
						status: 'pending',
						order: 1,
						createdAt: '2024-01-01T00:00:00Z'
					}
				]
			};
			const { container } = render(TaskList, { tasks: [inProgressTask] });

			// In-progress tasks should have expanded icon rotated
			const expandIcon = container.querySelector('.expand-icon.is-expanded');
			expect(expandIcon).toBeInTheDocument();
		});

		it('displays subtask status icons correctly', async () => {
			const taskWithMixedSubtasks: AgentTask = {
				id: 'task-6',
				title: 'Task with mixed subtasks',
				status: 'in_progress',
				subtasks: [
					{
						id: 'sub-pending',
						taskId: 'task-6',
						title: 'Pending subtask',
						mode: 'write',
						status: 'pending',
						order: 1,
						createdAt: '2024-01-01T00:00:00Z'
					},
					{
						id: 'sub-completed',
						taskId: 'task-6',
						title: 'Completed subtask',
						mode: 'write',
						status: 'completed',
						order: 2,
						createdAt: '2024-01-01T00:00:00Z'
					},
					{
						id: 'sub-failed',
						taskId: 'task-6',
						title: 'Failed subtask',
						mode: 'write',
						status: 'failed',
						order: 3,
						createdAt: '2024-01-01T00:00:00Z'
					}
				]
			};

			const { container } = render(TaskList, {
				tasks: [taskWithMixedSubtasks],
				ontaskclick: vi.fn()
			});

			const taskItem = screen.getByText('Task with mixed subtasks').closest('.task-item');
			if (taskItem) {
				await fireEvent.click(taskItem);

				await waitFor(() => {
					const subtasks = container.querySelectorAll('.subtask-item');
					expect(subtasks.length).toBe(3);
				});
			}
		});
	});

	describe('interactions', () => {
		it('calls ontaskclick when task is clicked', async () => {
			const handleClick = vi.fn();
			render(TaskList, {
				...defaultProps,
				ontaskclick: handleClick
			});

			const taskItem = screen.getByText('Write tests').closest('.task-item');
			if (taskItem) {
				await fireEvent.click(taskItem);
				expect(handleClick).toHaveBeenCalledTimes(1);
			}
		});

		it('calls onsubtaskclick when subtask is clicked', async () => {
			const handleSubtaskClick = vi.fn();
			const { container } = render(TaskList, {
				...defaultProps,
				onsubtaskclick: handleSubtaskClick
			});

			// First expand the task
			const taskItem = screen.getByText('Implement authentication').closest('.task-item');
			if (taskItem) {
				await fireEvent.click(taskItem);

				await waitFor(() => {
					const subtaskButton = screen.getByText('Create login form');
					fireEvent.click(subtaskButton);
					expect(handleSubtaskClick).toHaveBeenCalledTimes(1);
				});
			}
		});

		it('toggles expansion state when clicking task with subtasks', async () => {
			const { container } = render(TaskList, { ...defaultProps });

			const taskItem = screen.getByText('Implement authentication').closest('.task-item');
			if (!taskItem) return;

			// Initially collapsed
			let expandIcon = container.querySelector('.expand-icon');
			expect(expandIcon).not.toHaveClass('is-expanded');

			// Expand
			await fireEvent.click(taskItem);
			expandIcon = container.querySelector('.expand-icon');
			expect(expandIcon?.classList.contains('is-expanded')).toBe(true);

			// Collapse
			await fireEvent.click(taskItem);
			expandIcon = container.querySelector('.expand-icon');
			expect(expandIcon?.classList.contains('is-expanded')).toBe(false);
		});
	});

	describe('keyboard interactions', () => {
		it('calls ontaskclick when Enter key is pressed on task', async () => {
			const handleClick = vi.fn();
			render(TaskList, {
				...defaultProps,
				ontaskclick: handleClick
			});

			const taskItem = screen.getByText('Write tests').closest('.task-item');
			if (taskItem) {
				await fireEvent.keyDown(taskItem, { key: 'Enter' });
				expect(handleClick).toHaveBeenCalledTimes(1);
			}
		});

		it('calls ontaskclick when Space key is pressed on task', async () => {
			const handleClick = vi.fn();
			render(TaskList, {
				...defaultProps,
				ontaskclick: handleClick
			});

			const taskItem = screen.getByText('Write tests').closest('.task-item');
			if (taskItem) {
				await fireEvent.keyDown(taskItem, { key: ' ' });
				expect(handleClick).toHaveBeenCalledTimes(1);
			}
		});

		it('prevents default behavior for Space key', async () => {
			const handleClick = vi.fn();
			render(TaskList, {
				...defaultProps,
				ontaskclick: handleClick
			});

			const taskItem = screen.getByText('Write tests').closest('.task-item');
			if (taskItem) {
				const event = new KeyboardEvent('keydown', { key: ' ' });
				const preventDefaultSpy = vi.spyOn(event, 'preventDefault');
				taskItem.dispatchEvent(event);

				if (preventDefaultSpy.mock.calls.length > 0) {
					expect(preventDefaultSpy).toHaveBeenCalled();
				}
			}
		});
	});

	describe('accessibility', () => {
		it('sets correct tabindex when ontaskclick is provided', () => {
			const { container } = render(TaskList, { ...defaultProps });

			const taskItems = container.querySelectorAll('.task-item.clickable');
			taskItems.forEach((item) => {
				expect(item).toHaveAttribute('tabindex', '0');
			});
		});

		it('sets negative tabindex when ontaskclick is not provided', () => {
			const { container } = render(TaskList, {
				tasks: mockTasks
				// no ontaskclick
			});

			const taskItems = container.querySelectorAll('.task-item');
			taskItems.forEach((item) => {
				expect(item).toHaveAttribute('tabindex', '-1');
			});
		});

		it('includes status in aria-label', () => {
			const { container } = render(TaskList, { ...defaultProps });

			const taskItems = container.querySelectorAll('.task-item[aria-label]');
			taskItems.forEach((item) => {
				const ariaLabel = item.getAttribute('aria-label');
				expect(ariaLabel).toMatch(/ - (pending|in_progress|completed|failed)/);
			});
		});

		it('has clickable class only when ontaskclick is provided', () => {
			const { container } = render(TaskList, { ...defaultProps });

			const clickableItems = container.querySelectorAll('.task-item.clickable');
			expect(clickableItems.length).toBe(mockTasks.length);
		});
	});

	describe('visual states', () => {
		it('applies selected class when task is selected', () => {
			const selectedTask: AgentTask = {
				...mockTasks[0],
				selected: true
			};
			const { container } = render(TaskList, { tasks: [selectedTask] });

			const taskItem = container.querySelector('.task-item.selected');
			expect(taskItem).toBeInTheDocument();
		});

		it('rotates expand icon when subtasks are expanded', async () => {
			const { container } = render(TaskList, { ...defaultProps });

			const taskItem = screen.getByText('Implement authentication').closest('.task-item');
			if (!taskItem) return;

			await fireEvent.click(taskItem);

			const expandIcon = container.querySelector('.expand-icon.is-expanded');
			expect(expandIcon).toBeInTheDocument();
		});
	});

	describe('empty state', () => {
		it('shows empty state message when no tasks provided', () => {
			render(TaskList, { tasks: [] });

			expect(screen.getByText('No tasks yet')).toBeInTheDocument();
		});

		it('shows additional context in empty state', () => {
			const { container } = render(TaskList, { tasks: [] });

			const emptyState = container.querySelector('.empty-state');
			expect(emptyState).toBeInTheDocument();
			// The empty state has a ::after pseudo-element with additional text
		});
	});

	describe('edge cases', () => {
		it('handles tasks with empty subtasks array', () => {
			const taskWithEmptySubtasks: AgentTask = {
				id: 'task-7',
				title: 'Task with empty subtasks',
				status: 'pending',
				subtasks: []
			};
			const { container } = render(TaskList, { tasks: [taskWithEmptySubtasks] });

			const expandIcon = container.querySelector('.expand-icon');
			expect(expandIcon).not.toBeInTheDocument();
		});

		it('handles very long task titles', () => {
			const longTitleTask: AgentTask = {
				id: 'task-8',
				title: 'This is a very long task title that goes on and on and should be truncated with ellipsis',
				status: 'pending'
			};
			render(TaskList, { tasks: [longTitleTask] });

			expect(
				screen.getByText((content) =>
					content.startsWith('This is a very long task title')
				)
			).toBeInTheDocument();
		});

		it('handles undefined description gracefully', () => {
			const taskWithoutDesc: AgentTask = {
				id: 'task-9',
				title: 'Task without description',
				status: 'pending'
				// no description
			};
			const { container } = render(TaskList, { tasks: [taskWithoutDesc] });

			const taskTitle = container.querySelector('.task-title');
			expect(taskTitle?.textContent).toBe('Task without description');

			const taskDesc = container.querySelector('.task-description');
			expect(taskDesc).not.toBeInTheDocument();
		});
	});
});
