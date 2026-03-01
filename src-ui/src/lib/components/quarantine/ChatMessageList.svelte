<script lang="ts">
	/**
	 * ChatMessageList Component
	 * Renders a list of chat messages with auto-scroll to bottom.
	 */
	import ChatMessage from './ChatMessage.svelte';
	import styles from './ChatMessageList.module.css';

	interface Message {
		id: string;
		role: 'user' | 'assistant' | 'system';
		content: string;
		timestamp: Date;
		status?: 'sending' | 'sent' | 'error';
	}

	interface Props {
		/** Array of messages to display */
		messages: Message[];

		/** Show typing indicator */
		isTyping?: boolean;

		/** Edit message handler */
		onEditMessage?: (id: string) => void;

		/** Delete message handler */
		onDeleteMessage?: (id: string) => void;

		/** Retry failed message handler */
		onRetryMessage?: (id: string) => void;
	}

	let {
		messages,
		isTyping = false,
		onEditMessage,
		onDeleteMessage,
		onRetryMessage
	}: Props = $props();

	let containerRef: HTMLDivElement | undefined = $state();
	let shouldAutoScroll = $state(true);

	// Auto-scroll to bottom when new messages arrive
	$effect(() => {
		const ref = containerRef;
		if (ref && shouldAutoScroll) {
			// Use requestAnimationFrame to ensure DOM is updated
			requestAnimationFrame(() => {
				if (ref) {
					ref.scrollTop = ref.scrollHeight;
				}
			});
		}
	});

	function handleScroll() {
		if (!containerRef) return;

		const { scrollTop, scrollHeight, clientHeight } = containerRef;
		const isNearBottom = scrollHeight - scrollTop - clientHeight < 100;

		// Only auto-scroll if user is near bottom
		shouldAutoScroll = isNearBottom;
	}

	function scrollToBottom() {
		if (containerRef) {
			containerRef.scrollTop = containerRef.scrollHeight;
			shouldAutoScroll = true;
		}
	}
</script>

<div
	bind:this={containerRef}
	class={styles.container}
	onscroll={handleScroll}
	role="log"
	aria-label="Chat messages"
	aria-live="polite"
>
	<div class={styles.messageList}>
		{#each messages as message (message.id)}
			<ChatMessage
				id={message.id}
				role={message.role}
				content={message.content}
				timestamp={message.timestamp}
				status={message.status}
				onEdit={onEditMessage ? () => onEditMessage(message.id) : undefined}
				onDelete={onDeleteMessage ? () => onDeleteMessage(message.id) : undefined}
				onRetry={onRetryMessage ? () => onRetryMessage(message.id) : undefined}
			/>
		{/each}

		{#if isTyping}
			<div class={styles.typingIndicator} aria-label="Assistant is typing">
				<div class={styles.typingDots}>
					<span class={styles.dot}></span>
					<span class={styles.dot}></span>
					<span class={styles.dot}></span>
				</div>
			</div>
		{/if}
	</div>

	{#if !shouldAutoScroll && messages.length > 0}
		<button
			class={styles.scrollToBottom}
			onclick={scrollToBottom}
			aria-label="Scroll to latest messages"
			type="button"
		>
			<svg
				width="20"
				height="20"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"
				stroke-linejoin="round"
			>
				<polyline points="6 9 12 15 18 9" />
			</svg>
		</button>
	{/if}
</div>
