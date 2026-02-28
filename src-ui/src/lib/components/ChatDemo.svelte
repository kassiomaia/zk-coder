<script lang="ts">
	/**
	 * ChatDemo Component
	 * Demo page showcasing the chat components with sample data.
	 */
	import { ChatArea } from '$lib/components';
	import styles from './ChatDemo.module.css';

	interface Message {
		id: string;
		role: 'user' | 'assistant' | 'system';
		content: string;
		timestamp: Date;
		status?: 'sending' | 'sent' | 'error';
	}

	// Sample messages for demo
	let messages = $state<Message[]>([
		{
			id: '1',
			role: 'system',
			content: 'Welcome to Coder! How can I help you today?',
			timestamp: new Date(Date.now() - 60000 * 5)
		},
		{
			id: '2',
			role: 'user',
			content: 'Can you help me create a React component?',
			timestamp: new Date(Date.now() - 60000 * 4)
		},
		{
			id: '3',
			role: 'assistant',
			content: `Of course! I'd be happy to help you create a React component. Here's a simple example to get you started:

<pre><code>import React from 'react';

interface ButtonProps {
  label: string;
  onClick: () => void;
  variant?: 'primary' | 'secondary';
}

export function Button({ label, onClick, variant = 'primary' }: ButtonProps) {
  return (
    &lt;button
      className={\`btn btn-\${variant}\`}
      onClick={onClick}
    &gt;
      {label}
    &lt;/button&gt;
  );
}</code></pre>

This is a reusable button component with TypeScript support. Let me know if you'd like me to customize it further!`,
			timestamp: new Date(Date.now() - 60000 * 3)
		},
		{
			id: '4',
			role: 'user',
			content: 'That looks great! Can you add disabled state support?',
			timestamp: new Date(Date.now() - 60000 * 2)
		},
		{
			id: '5',
			role: 'assistant',
			content: `Sure! Here's the updated component with disabled state:

<pre><code>import React from 'react';

interface ButtonProps {
  label: string;
  onClick: () => void;
  variant?: 'primary' | 'secondary';
  disabled?: boolean;
}

export function Button({
  label,
  onClick,
  variant = 'primary',
  disabled = false
}: ButtonProps) {
  return (
    &lt;button
      className={\`btn btn-\${variant}\`}
      onClick={onClick}
      disabled={disabled}
    &gt;
      {label}
    &lt;/button&gt;
  );
}</code></pre>

I've added a \`disabled\` prop that defaults to \`false\`. When set to \`true\`, the button will be non-interactive.`,
			timestamp: new Date(Date.now() - 60000)
		}
	]);

	let inputValue = $state('');
	let isTyping = $state(false);
	let isLoading = $state(false);

	function generateId(): string {
		return `msg_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
	}

	async function handleSubmit() {
		if (!inputValue.trim()) return;

		const userMessage: Message = {
			id: generateId(),
			role: 'user',
			content: inputValue.trim(),
			timestamp: new Date(),
			status: 'sending'
		};

		messages = [...messages, userMessage];
		inputValue = '';
		isLoading = true;

		// Simulate API call
		await new Promise((resolve) => setTimeout(resolve, 500));

		// Update user message status
		messages = messages.map((m) =>
			m.id === userMessage.id ? { ...m, status: 'sent' as const } : m
		);

		// Simulate assistant typing
		isTyping = true;

		await new Promise((resolve) => setTimeout(resolve, 1500));

		// Add assistant response
		const assistantMessage: Message = {
			id: generateId(),
			role: 'assistant',
			content: `I received your message: "${userMessage.content}"\n\nThis is a demo response. In a real application, this would be the AI's response to your query.`,
			timestamp: new Date()
		};

		messages = [...messages, assistantMessage];
		isTyping = false;
		isLoading = false;
	}

	function handleInputChange(value: string) {
		inputValue = value;
	}

	function handleEditMessage(id: string) {
		const message = messages.find((m) => m.id === id);
		if (message && message.role === 'user') {
			inputValue = message.content;
			messages = messages.filter((m) => m.id !== id);
		}
	}

	function handleDeleteMessage(id: string) {
		messages = messages.filter((m) => m.id !== id);
	}

	function handleRetryMessage(id: string) {
		messages = messages.map((m) => (m.id === id ? { ...m, status: 'sending' as const } : m));

		// Simulate retry
		setTimeout(() => {
			messages = messages.map((m) => (m.id === id ? { ...m, status: 'sent' as const } : m));
		}, 1000);
	}
</script>

<div class={styles.demoContainer}>
	<header class={styles.header}>
		<h1>Chat Components Demo</h1>
		<p>Interactive demo showcasing the Coder chat interface components</p>
	</header>

	<div class={styles.chatWrapper}>
		<ChatArea
			{messages}
			{inputValue}
			{isTyping}
			isDisabled={false}
			{isLoading}
			onInputChange={handleInputChange}
			onSubmit={handleSubmit}
			onEditMessage={handleEditMessage}
			onDeleteMessage={handleDeleteMessage}
			onRetryMessage={handleRetryMessage}
		/>
	</div>
</div>
