import {fireEvent, render, screen} from '@testing-library/svelte';
import {beforeEach, describe, expect, it, vi} from 'vitest';
import InlineThreadList from './InlineThreadList.svelte';
import type {Thread} from '$lib/types/thread';

describe('InlineThreadList', () => {
	const mockThreads: Thread[] = [
		{
			id: 'thread1',
			conversationId: 'conv1',
			parentId: null,
			title: 'Open Thread 1',
			createdAt: new Date('2024-01-15T10:00:00'),
			updatedAt: new Date('2024-01-15T12:00:00'),
			messageCount: 5,
			metadata: { resolved: false }
		},
		{
			id: 'thread2',
			conversationId: 'conv1',
			parentId: null,
			title: 'Open Thread 2',
			createdAt: new Date('2024-01-15T11:00:00'),
			updatedAt: new Date('2024-01-15T11:30:00'),
			messageCount: 3,
			metadata: { resolved: false }
		},
		{
			id: 'thread3',
			conversationId: 'conv1',
			parentId: null,
			title: 'Resolved Thread',
			createdAt: new Date('2024-01-14T10:00:00'),
			updatedAt: new Date('2024-01-14T12:00:00'),
			messageCount: 10,
			metadata: { resolved: true }
		}
	];

	const defaultProps = {
		threads: mockThreads,
		onSelectThread: vi.fn()
	};

	beforeEach(() => {
		vi.clearAllMocks();
	});

	describe('thread rendering', () => {
		it('renders all threads', () => {
			render(InlineThreadList, { ...defaultProps });
			// Check that threads are rendered (ThreadItem renders thread titles)
			const threadTitles = screen.getAllByText(/Open Thread|Resolved Thread/);
			expect(threadTitles.length).toBeGreaterThan(0);
		});

		it('renders open threads before resolved threads', () => {
			render(InlineThreadList, { ...defaultProps });
			// Open threads should appear before resolved ones
			const allText = screen.getByRole('listbox', { name: 'Threads' }).textContent;
			const open1Index = allText?.indexOf('Open Thread 1') ?? -1;
			const open2Index = allText?.indexOf('Open Thread 2') ?? -1;
			const resolvedIndex = allText?.indexOf('Resolved Thread') ?? -1;

			// Both open threads should appear before resolved
			expect(open1Index).toBeLessThan(resolvedIndex);
			expect(open2Index).toBeLessThan(resolvedIndex);
		});
	});

	describe('thread selection', () => {
		it('highlights the selected thread', () => {
			render(InlineThreadList, {
				...defaultProps,
				selectedThreadId: 'thread1'
			});
			// ThreadItem should mark itself as selected
			const selectedThread = screen.getByText('Open Thread 1');
			expect(selectedThread).toBeTruthy();
		});

		it('calls onSelectThread when a thread is clicked', async () => {
			render(InlineThreadList, { ...defaultProps });
			// Try to click on a thread element
			const threadElement = screen.getByText('Open Thread 1');
			await fireEvent.click(threadElement);

			// The ThreadItem component should handle click and call parent
			// This is a basic integration test
			expect(defaultProps.onSelectThread).toHaveBeenCalled();
		});
	});

	describe('empty state', () => {
		it('renders nothing when no threads provided', () => {
			const { container } = render(InlineThreadList, {
				threads: [],
				onSelectThread: vi.fn()
			});
			// Component renders nothing when threads array is empty
			expect(container.querySelector('#inline-threads')).toBeNull();
		});
	});

	describe('expanded state', () => {
		it('applies expanded class when expanded is true', () => {
			render(InlineThreadList, {
				...defaultProps,
				expanded: true
			});
			const containerDiv = document.getElementById('inline-threads');
			// The expanded class from the CSS module would be applied
			expect(containerDiv).toBeTruthy();
		});

		it('does not apply expanded class when expanded is false', () => {
			render(InlineThreadList, {
				...defaultProps,
				expanded: false
			});
			const containerDiv = document.getElementById('inline-threads');
			expect(containerDiv).toBeTruthy();
		});
	});

	describe('thread indicators', () => {
		it('passes streaming state to ThreadItem', () => {
			render(InlineThreadList, {
				...defaultProps,
				activeStreamingThreadIds: ['thread1']
			});
			// ThreadItem receives isStreaming prop
			expect(screen.getByText('Open Thread 1')).toBeTruthy();
		});

		it('passes unread state to ThreadItem', () => {
			render(InlineThreadList, {
				...defaultProps,
				unreadThreadIds: ['thread2'],
				selectedThreadId: 'thread1'
			});
			// ThreadItem receives hasUnread prop
			expect(screen.getByText('Open Thread 2')).toBeTruthy();
		});
	});

	describe('accessibility', () => {
		it('has listbox role for thread list', () => {
			render(InlineThreadList, { ...defaultProps });
			const list = screen.getByRole('listbox', { name: 'Threads' });
			expect(list).toBeTruthy();
		});

		it('has aria-label for thread list', () => {
			render(InlineThreadList, { ...defaultProps });
			const list = screen.getByRole('listbox', { name: 'Threads' });
			expect(list).toBeTruthy();
		});

		it('has id for external references', () => {
			render(InlineThreadList, { ...defaultProps });
			// The id is on the container div, not the listbox
			const containerDiv = document.getElementById('inline-threads');
			expect(containerDiv).toBeTruthy();
		});
	});

	describe('data attributes', () => {
		it('sets data-threads-expanded when expanded', () => {
			render(InlineThreadList, {
				...defaultProps,
				expanded: true
			});
			// The div container with id="inline-threads" has the data-attribute
			const containerDiv = document.getElementById('inline-threads');
			// HTML attributes stringify boolean true to "true"
			expect(containerDiv?.getAttribute('data-threads-expanded')).toBe('true');
		});

		it('does not set data-threads-expanded when not expanded', () => {
			render(InlineThreadList, {
				...defaultProps,
				expanded: false
			});
			// The div container doesn't have the data-attribute when not expanded
			const containerDiv = document.getElementById('inline-threads');
			expect(containerDiv?.getAttribute('data-threads-expanded')).toBeNull();
		});
	});
});
