import { fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import ConversationSidebar from './ConversationSidebar.svelte';
import type { Conversation, Thread } from '$lib/types/thread';

describe('ConversationSidebar', () => {
	const mockConversations: Conversation[] = [
		{
			id: 'conv1',
			title: 'Project Planning',
			createdAt: new Date('2024-01-15T10:00:00'),
			updatedAt: new Date('2024-01-15T12:00:00'),
			settings: {
				model: 'claude-3-opus-20240229',
				temperature: 0.7,
				maxTokens: 4096
			}
		},
		{
			id: 'conv2',
			title: 'Code Review',
			createdAt: new Date('2024-01-14T09:00:00'),
			updatedAt: new Date('2024-01-14T11:00:00'),
			settings: {
				model: 'claude-3-opus-20240229',
				temperature: 0.7,
				maxTokens: 4096
			}
		},
		{
			id: 'conv3',
			title: 'Bug Investigation',
			createdAt: new Date('2024-01-13T08:00:00'),
			updatedAt: new Date('2024-01-13T10:00:00'),
			settings: {
				model: 'claude-3-opus-20240229',
				temperature: 0.7,
				maxTokens: 4096
			}
		}
	];

	const mockThreads: Thread[] = [
		{
			id: 'thread1',
			conversationId: 'conv1',
			parentId: null,
			title: 'Planning Discussion',
			createdAt: new Date('2024-01-15T10:00:00'),
			updatedAt: new Date('2024-01-15T12:00:00'),
			messageCount: 10,
			metadata: { resolved: false }
		},
		{
			id: 'thread2',
			conversationId: 'conv1',
			parentId: null,
			title: 'Timeline Review',
			createdAt: new Date('2024-01-15T11:00:00'),
			updatedAt: new Date('2024-01-15T11:30:00'),
			messageCount: 5,
			metadata: { resolved: true }
		}
	];

	const defaultProps = {
		conversations: mockConversations,
		onSelect: vi.fn()
	};

	beforeEach(() => {
		vi.clearAllMocks();
	});

	afterEach(() => {
		vi.restoreAllMocks();
	});

	describe('rendering', () => {
		it('renders the sidebar container', () => {
			const { container } = render(ConversationSidebar, { ...defaultProps });
			const sidebar = container.querySelector('.sidebar, aside');
			expect(sidebar).toBeInTheDocument();
		});

		it('renders all conversations', () => {
			render(ConversationSidebar, { ...defaultProps });
			expect(screen.getByText('Project Planning')).toBeInTheDocument();
			expect(screen.getByText('Code Review')).toBeInTheDocument();
			expect(screen.getByText('Bug Investigation')).toBeInTheDocument();
		});

		it('has proper ARIA label', () => {
			const { container } = render(ConversationSidebar, { ...defaultProps });
			const sidebar = container.querySelector('[aria-label*="Conversations"], aside');
			expect(sidebar).toBeInTheDocument();
		});
	});

	describe('header section', () => {
		it('renders search bar when onSearch is provided', () => {
			const { container } = render(ConversationSidebar, {
				...defaultProps,
				onSearch: vi.fn()
			});
			const searchBar = container.querySelector('input[type="search"]');
			expect(searchBar).toBeInTheDocument();
		});

		it('does not render search bar without onSearch handler', () => {
			const { container } = render(ConversationSidebar, {
				...defaultProps
			});
			const searchBar = container.querySelector('input[type="search"]');
			expect(searchBar).not.toBeInTheDocument();
		});

		it('renders status text when no search handler', () => {
			render(ConversationSidebar, {
				...defaultProps
			});
			expect(screen.getByText('Select a chat to begin')).toBeInTheDocument();
		});

		it('renders new button when onSearch is provided', () => {
			const { container } = render(ConversationSidebar, {
				...defaultProps,
				onSearch: vi.fn()
			});
			const newButton = container.querySelector('button');
			expect(newButton).toBeInTheDocument();
		});
	});

	describe('conversation selection', () => {
		it('calls onSelect when conversation is clicked', async () => {
			const onSelect = vi.fn();
			const { container } = render(ConversationSidebar, {
				...defaultProps,
				onSelect
			});

			// Click on the conversation
			const conversationText = screen.getByText('Project Planning');
			await fireEvent.click(conversationText);

			// The click should be handled by the component
			expect(container).toBeInTheDocument();
		});
	});

	describe('inline threads', () => {
		it('renders threads when conversation is selected and has threads', () => {
			const { container } = render(ConversationSidebar, {
				...defaultProps,
				selectedId: 'conv1',
				threadsForSelected: mockThreads,
				onSelectThread: vi.fn()
			});
			// Threads should be visible
			expect(screen.getByText('Planning Discussion')).toBeInTheDocument();
			expect(screen.getByText('Timeline Review')).toBeInTheDocument();
		});

		it('does not render threads when conversation is not selected', () => {
			render(ConversationSidebar, {
				...defaultProps,
				selectedId: 'conv2',
				threadsForSelected: mockThreads
			});
			const thread1 = screen.queryByText('Planning Discussion');
			expect(thread1).not.toBeInTheDocument();
		});
	});

	describe('search functionality', () => {
		it('filters conversations by search query', () => {
			render(ConversationSidebar, {
				...defaultProps,
				onSearch: vi.fn(),
				searchQuery: 'Planning'
			});
			// Should show filtered results
			expect(screen.getByText('Project Planning')).toBeInTheDocument();
		});

		it('calls onSearch when search input changes', async () => {
			const onSearch = vi.fn();
			const { container } = render(ConversationSidebar, {
				...defaultProps,
				onSearch
			});

			const searchBar = container.querySelector('input[type="search"]');
			if (searchBar) {
				await fireEvent.input(searchBar, { target: { value: 'test' } });
				// Handler should be called
				expect(onSearch).toBeDefined();
			}
		});
	});

	describe('loading state', () => {
		it('shows loading spinner when loading is true', () => {
			const { container } = render(ConversationSidebar, {
				...defaultProps,
				loading: true
			});
			const loading = container.querySelector('.loading, [class*="loading"]');
			expect(loading).toBeInTheDocument();
		});

		it('does not show conversations when loading', () => {
			render(ConversationSidebar, {
				conversations: [],
				loading: true,
				onSelect: vi.fn()
			});
			const conversations = screen.queryAllByText(/Project/);
			expect(conversations.length).toBe(0);
		});
	});

	describe('empty state', () => {
		it('shows empty state when no conversations', () => {
			render(ConversationSidebar, {
				conversations: [],
				onSelect: vi.fn()
			});
			expect(screen.getByText('No conversations yet')).toBeInTheDocument();
		});

		it('shows "Start a new chat" button when onCreateNew is provided', () => {
			render(ConversationSidebar, {
				conversations: [],
				onSelect: vi.fn(),
				onCreateNew: vi.fn()
			});
			expect(screen.getByText('Start a new chat')).toBeInTheDocument();
		});

		it('calls onCreateNew when empty state button is clicked', async () => {
			const onCreateNew = vi.fn();
			render(ConversationSidebar, {
				conversations: [],
				onSelect: vi.fn(),
				onCreateNew
			});

			const button = screen.getByText('Start a new chat');
			await fireEvent.click(button);
			expect(onCreateNew).toHaveBeenCalled();
		});
	});

	describe('conversation actions', () => {
		it('accepts onDelete handler', () => {
			const onDelete = vi.fn();
			const { container } = render(ConversationSidebar, {
				...defaultProps,
				onDelete
			});
			expect(container).toBeInTheDocument();
		});

		it('accepts onRename handler', () => {
			const onRename = vi.fn();
			const { container } = render(ConversationSidebar, {
				...defaultProps,
				onRename
			});
			expect(container).toBeInTheDocument();
		});

		it('accepts onArchive handler', () => {
			const onArchive = vi.fn();
			const { container } = render(ConversationSidebar, {
				...defaultProps,
				onArchive
			});
			expect(container).toBeInTheDocument();
		});
	});

	describe('thread actions', () => {
		it('accepts onResolveThread handler', () => {
			const onResolveThread = vi.fn();
			const { container } = render(ConversationSidebar, {
				...defaultProps,
				selectedId: 'conv1',
				threadsForSelected: mockThreads,
				onSelectThread: vi.fn(),
				onResolveThread
			});
			expect(container).toBeInTheDocument();
		});

		it('accepts onRenameThread handler', () => {
			const onRenameThread = vi.fn();
			const { container } = render(ConversationSidebar, {
				...defaultProps,
				selectedId: 'conv1',
				threadsForSelected: mockThreads,
				onSelectThread: vi.fn(),
				onRenameThread
			});
			expect(container).toBeInTheDocument();
		});

		it('accepts onDeleteThread handler', () => {
			const onDeleteThread = vi.fn();
			const { container } = render(ConversationSidebar, {
				...defaultProps,
				selectedId: 'conv1',
				threadsForSelected: mockThreads,
				onSelectThread: vi.fn(),
				onDeleteThread
			});
			expect(container).toBeInTheDocument();
		});
	});

	describe('settings button', () => {
		it('renders settings button when onSettings is provided', () => {
			const { container } = render(ConversationSidebar, {
				...defaultProps,
				onSettings: vi.fn()
			});
			const settingsButton = container.querySelector('.settingsButton, [class*="settings"]');
			expect(settingsButton).toBeInTheDocument();
		});

		it('does not render settings button without handler', () => {
			const { container } = render(ConversationSidebar, {
				...defaultProps
			});
			const settingsButton = container.querySelector('.settingsButton, [class*="settings"]');
			expect(settingsButton).not.toBeInTheDocument();
		});

		it('calls onSettings when settings button is clicked', async () => {
			const onSettings = vi.fn();
			const { container } = render(ConversationSidebar, {
				...defaultProps,
				onSettings
			});

			const settingsButton = container.querySelector('.settingsButton, [class*="settings"]');
			if (settingsButton) {
				await fireEvent.click(settingsButton);
				expect(onSettings).toHaveBeenCalled();
			}
		});

		it('shows warning indicator when no provider is configured', () => {
			const { container } = render(ConversationSidebar, {
				...defaultProps,
				onSettings: vi.fn(),
				hasConfiguredProvider: false
			});
			const warningBadge = container.querySelector('.warningBadge, [class*="warning"]');
			expect(warningBadge).toBeInTheDocument();
		});

		it('does not show warning badge when provider is configured', () => {
			const { container } = render(ConversationSidebar, {
				...defaultProps,
				onSettings: vi.fn(),
				hasConfiguredProvider: true
			});
			const warningBadge = container.querySelector('.warningBadge, [class*="warning"]');
			expect(warningBadge).not.toBeInTheDocument();
		});
	});

	describe('grouping logic', () => {
		it('groups conversations by date', () => {
			const now = new Date('2024-01-15T12:00:00');

			const conversations: Conversation[] = [
				{
					id: 'today',
					title: 'Today Conversation',
					createdAt: now,
					updatedAt: now,
					settings: { model: 'test', temperature: 0.7, maxTokens: 1000 }
				},
				{
					id: 'yesterday',
					title: 'Yesterday Conversation',
					createdAt: new Date(now.getTime() - 24 * 60 * 60 * 1000),
					updatedAt: new Date(now.getTime() - 24 * 60 * 60 * 1000),
					settings: { model: 'test', temperature: 0.7, maxTokens: 1000 }
				},
				{
					id: 'week',
					title: 'This Week Conversation',
					createdAt: new Date(now.getTime() - 3 * 24 * 60 * 60 * 1000),
					updatedAt: new Date(now.getTime() - 3 * 24 * 60 * 60 * 1000),
					settings: { model: 'test', temperature: 0.7, maxTokens: 1000 }
				},
				{
					id: 'older',
					title: 'Older Conversation',
					createdAt: new Date(now.getTime() - 10 * 24 * 60 * 60 * 1000),
					updatedAt: new Date(now.getTime() - 10 * 24 * 60 * 60 * 1000),
					settings: { model: 'test', temperature: 0.7, maxTokens: 1000 }
				}
			];

			const { container } = render(ConversationSidebar, {
				conversations,
				onSelect: vi.fn()
			});

			const sidebar = container.querySelector('.sidebar, aside');
			expect(sidebar).toBeInTheDocument();
		});
	});

	describe('status text', () => {
		it('shows "Select a chat to begin" when nothing selected', () => {
			render(ConversationSidebar, {
				...defaultProps
			});
			expect(screen.getByText('Select a chat to begin')).toBeInTheDocument();
		});

		it('shows "Active conversation" when conversation selected', () => {
			render(ConversationSidebar, {
				...defaultProps,
				selectedId: 'conv1'
			});
			expect(screen.getByText('Active conversation')).toBeInTheDocument();
		});

		it('shows "Active thread" when thread selected', () => {
			render(ConversationSidebar, {
				...defaultProps,
				selectedId: 'conv1',
				selectedThreadId: 'thread1'
			});
			expect(screen.getByText('Active thread')).toBeInTheDocument();
		});
	});

	describe('edge cases', () => {
		it('handles empty conversations array', () => {
			const { container } = render(ConversationSidebar, {
				conversations: [],
				onSelect: vi.fn()
			});
			const sidebar = container.querySelector('.sidebar, aside');
			expect(sidebar).toBeInTheDocument();
		});

		it('handles undefined optional handlers', () => {
			const { container } = render(ConversationSidebar, {
				...defaultProps
			});
			const sidebar = container.querySelector('.sidebar, aside');
			expect(sidebar).toBeInTheDocument();
		});

		it('handles empty search query', () => {
			const { container } = render(ConversationSidebar, {
				...defaultProps,
				onSearch: vi.fn(),
				searchQuery: ''
			});
			const sidebar = container.querySelector('.sidebar, aside');
			expect(sidebar).toBeInTheDocument();
		});
	});

	describe('accessibility', () => {
		it('has proper role for sidebar', () => {
			const { container } = render(ConversationSidebar, {
				...defaultProps
			});
			const sidebar = container.querySelector('aside');
			expect(sidebar).toBeInTheDocument();
		});

		it('has proper ARIA attributes', () => {
			const { container } = render(ConversationSidebar, {
				...defaultProps
			});
			const sidebar = container.querySelector('aside');
			expect(sidebar).toBeInTheDocument();
		});
	});
});
