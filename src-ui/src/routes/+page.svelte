<script lang="ts">
	/**
	 * Main Page - Chat Interface Demo
	 * Demonstrates the chat interface with mock data.
	 */
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import ChatLayout from '$lib/components/layout/ChatLayout.svelte';
	import type { Conversation, Thread } from '$lib/types/thread';
	import type { Message } from '$lib/types/message';

	// App state
	let appInfo = $state<{ name: string; version: string } | null>(null);
	let backendStatus = $state('Connecting...');

	// Chat state
	let conversations = $state<Conversation[]>([]);
	let selectedConversation = $state<Conversation | undefined>(undefined);
	let selectedThread = $state<Thread | undefined>(undefined);
	let messages = $state<Message[]>([]);
	let loading = $state(false);

	// Mock data
	function createMockData() {
		const now = new Date();

		const mockMessages: Message[] = [
			{
				id: 'm1',
				conversationId: 'c1',
				role: 'user',
				content: 'Hello! Can you help me write a function to calculate the Fibonacci sequence?',
				timestamp: new Date(now.getTime() - 300000),
				status: 'delivered'
			},
			{
				id: 'm2',
				conversationId: 'c1',
				role: 'assistant',
				content: `Of course! Here's a recursive implementation of the Fibonacci sequence in JavaScript:

\`\`\`javascript
function fibonacci(n) {
  if (n <= 1) return n;
  return fibonacci(n - 1) + fibonacci(n - 2);
}

// Example usage:
console.log(fibonacci(10)); // 55
\`\`\`

This is a simple recursive approach. For better performance with larger numbers, you might want to use memoization or an iterative approach.

Would you like me to show you an optimized version?`,
				timestamp: new Date(now.getTime() - 280000),
				status: 'delivered',
				metadata: {
					tokens: 89,
					model: 'claude-3-opus'
				}
			},
			{
				id: 'm3',
				conversationId: 'c1',
				role: 'user',
				content: 'Yes, please show me the optimized version with memoization!',
				timestamp: new Date(now.getTime() - 200000),
				status: 'delivered'
			},
			{
				id: 'm4',
				conversationId: 'c1',
				role: 'assistant',
				content: `Here's an optimized version using memoization:

\`\`\`javascript
const fibonacci = (function() {
  const cache = new Map();

  return function fib(n) {
    if (n <= 1) return n;

    if (cache.has(n)) {
      return cache.get(n);
    }

    const result = fib(n - 1) + fib(n - 2);
    cache.set(n, result);
    return result;
  };
})();

// Or using an iterative approach (O(n) time, O(1) space):
function fibonacciIterative(n) {
  if (n <= 1) return n;

  let prev = 0, curr = 1;
  for (let i = 2; i <= n; i++) {
    [prev, curr] = [curr, prev + curr];
  }
  return curr;
}
\`\`\`

The **iterative approach** is generally preferred as it has O(n) time complexity and O(1) space complexity, while the recursive version has O(2^n) time complexity without memoization.`,
				timestamp: new Date(now.getTime() - 180000),
				status: 'delivered',
				metadata: {
					tokens: 156,
					model: 'claude-3-opus'
				}
			}
		];

		const mockThreads: Thread[] = [
			{
				id: 't1',
				conversationId: 'c1',
				parentId: null,
				title: 'Fibonacci Implementation',
				createdAt: new Date(now.getTime() - 300000),
				updatedAt: new Date(now.getTime() - 180000),
				messageCount: 4,
				metadata: {
					resolved: false,
					summary: 'Discussing different approaches to implement Fibonacci sequence'
				}
			},
			{
				id: 't2',
				conversationId: 'c1',
				parentId: null,
				title: 'Code Review Questions',
				createdAt: new Date(now.getTime() - 86400000),
				updatedAt: new Date(now.getTime() - 80000000),
				messageCount: 2,
				metadata: {
					resolved: true,
					summary: 'Resolved: Code review best practices'
				}
			}
		];

		const mockConversations: Conversation[] = [
			{
				id: 'c1',
				title: 'JavaScript Algorithms',
				createdAt: new Date(now.getTime() - 86400000),
				updatedAt: new Date(now.getTime() - 180000),
				threads: mockThreads,
				settings: {
					model: 'claude-3-opus',
					temperature: 0.7,
					maxTokens: 4096
				}
			},
			{
				id: 'c2',
				title: 'React Component Design',
				createdAt: new Date(now.getTime() - 172800000),
				updatedAt: new Date(now.getTime() - 172800000),
				threads: [],
				settings: {
					model: 'claude-3-sonnet',
					temperature: 0.7,
					maxTokens: 2048
				}
			},
			{
				id: 'c3',
				title: 'Database Schema Review',
				createdAt: new Date(now.getTime() - 604800000),
				updatedAt: new Date(now.getTime() - 604800000),
				threads: [],
				settings: {
					model: 'claude-3-opus',
					temperature: 0.5,
					maxTokens: 4096
				}
			}
		];

		conversations = mockConversations;
		selectedConversation = mockConversations[0];
		selectedThread = mockThreads[0];
		messages = mockMessages;
	}

	// Initialize
	onMount(async () => {
		try {
			const health = await invoke<string>('health_check');
			backendStatus = health;
			const info = await invoke<{ name: string; version: string }>('get_app_info');
			appInfo = info;
		} catch (error) {
			backendStatus = 'Backend unavailable (using mock data)';
			console.log('Backend not available:', error);
		}

		// Load mock data
		createMockData();
	});

	// Handlers
	function handleSelectConversation(conversation: Conversation) {
		selectedConversation = conversation;
		selectedThread = conversation.threads[0];
		messages = [];
		// In a real app, fetch messages for the thread
	}

	function handleSelectThread(thread: Thread) {
		selectedThread = thread;
		// In a real app, fetch messages for this thread
	}

	function handleSendMessage(content: string) {
		const newMessage: Message = {
			id: `m${Date.now()}`,
			conversationId: selectedConversation?.id || '',
			role: 'user',
			content,
			timestamp: new Date(),
			status: 'sending'
		};
		messages = [...messages, newMessage];

		// Simulate AI response
		setTimeout(() => {
			const aiResponse: Message = {
				id: `m${Date.now() + 1}`,
				conversationId: selectedConversation?.id || '',
				role: 'assistant',
				content: `I received your message: "${content.substring(0, 50)}${content.length > 50 ? '...' : ''}"\n\nThis is a demo response. In a real implementation, this would be generated by the AI backend.`,
				timestamp: new Date(),
				status: 'delivered',
				metadata: {
					tokens: Math.floor(Math.random() * 100) + 20,
					model: 'claude-3-opus'
				}
			};
			messages = [...messages, aiResponse];
		}, 1000);
	}

	function handleCreateConversation() {
		const newConversation: Conversation = {
			id: `c${Date.now()}`,
			title: 'New Conversation',
			createdAt: new Date(),
			updatedAt: new Date(),
			threads: [],
			settings: {
				model: 'claude-3-opus',
				temperature: 0.7,
				maxTokens: 4096
			}
		};
		conversations = [newConversation, ...conversations];
		selectedConversation = newConversation;
		selectedThread = undefined;
		messages = [];
	}

	function handleCreateThread() {
		if (!selectedConversation) return;

		const newThread: Thread = {
			id: `t${Date.now()}`,
			conversationId: selectedConversation.id,
			parentId: null,
			title: 'New Thread',
			createdAt: new Date(),
			updatedAt: new Date(),
			messageCount: 0,
			metadata: {
				resolved: false
			}
		};

		selectedConversation.threads = [newThread, ...selectedConversation.threads];
		selectedThread = newThread;
		messages = [];
	}

	function handleResolveThread(thread: Thread) {
		thread.metadata.resolved = !thread.metadata.resolved;
		conversations = [...conversations]; // Trigger reactivity
	}

	function handleDeleteConversation(conversation: Conversation) {
		conversations = conversations.filter((c) => c.id !== conversation.id);
		if (selectedConversation?.id === conversation.id) {
			selectedConversation = conversations[0];
			selectedThread = selectedConversation?.threads[0];
		}
	}

	function handleSearch(query: string) {
		// In a real app, filter conversations based on search
		console.log('Search:', query);
	}
</script>

<svelte:head>
	<title>{appInfo?.name || 'Coder'} - AI Assistant</title>
</svelte:head>

<ChatLayout
	{conversations}
	{selectedConversation}
	{selectedThread}
	{messages}
	onSelectConversation={handleSelectConversation}
	onSelectThread={handleSelectThread}
	onSendMessage={handleSendMessage}
	onCreateConversation={handleCreateConversation}
	onCreateThread={handleCreateThread}
	onResolveThread={handleResolveThread}
	onDeleteConversation={handleDeleteConversation}
	onSearch={handleSearch}
	{loading}
/>

<!-- Status indicator -->
<div class="status-bar">
	<span class="status">{backendStatus}</span>
	{#if appInfo}
		<span class="version">{appInfo.name} v{appInfo.version}</span>
	{/if}
</div>

<style>
	.status-bar {
		position: fixed;
		bottom: 0;
		left: 0;
		right: 0;
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: var(--space-1) var(--space-4);
		background: var(--color-bg-secondary);
		border-top: 1px solid var(--color-border-primary);
		font-size: var(--text-xs);
		color: var(--color-text-muted);
		z-index: var(--z-toast);
	}

	.status {
		font-family: var(--font-mono);
	}

	.version {
		font-family: var(--font-mono);
		opacity: 0.7;
	}

	@media (max-width: 600px) {
		.status-bar {
			padding: var(--space-1) var(--space-2);
			font-size: var(--text-2xs);
		}
	}
</style>
