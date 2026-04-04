<script lang="ts">
	/**
	 * EmptyThreadState Component
	 *
	 * Action-oriented empty state positioned slightly above center.
	 * Provides clear guidance and a primary call to action.
	 *
	 * Contexts:
	 * - 'none': No conversation selected (main chat area)
	 * - 'conversation': Conversation selected but no thread
	 * - 'thread': Thread selected but no messages
	 */
	import styles from './EmptyThreadState.module.css';

	type EmptyContext = 'none' | 'conversation' | 'thread';

	interface Props {
		/** Whether the input has focus or content */
		hasInputActivity?: boolean;
		/** Whether secondary elements should be visible */
		showSecondary?: boolean;
		/** The context of the empty state */
		context?: EmptyContext;
		/** Optional conversation title for context */
		conversationTitle?: string;
		/** Create conversation callback */
		onCreateConversation?: () => void;
	}

	let {
		hasInputActivity = false,
		showSecondary = true,
		context = 'none',
		conversationTitle,
		onCreateConversation
	}: Props = $props();

	// Animation state management
	let isTransitioning = $state(false);

	// Derive display content based on context
	let headlineText = $derived(() => {
		switch (context) {
			case 'thread':
				return 'Start a conversation';
			case 'conversation':
				return 'Begin your first thread';
			case 'none':
			default:
				return 'Ready to collaborate';
		}
	});

	let hintText = $derived(() => {
		switch (context) {
			case 'thread':
				return 'Type your message below to begin working with AI';
			case 'conversation':
				return 'Create a thread from the sidebar or send a message directly';
			case 'none':
			default:
				return 'Select a conversation or create a new one to get started';
		}
	});

	let subText = $derived(() => {
		if (context === 'conversation' && conversationTitle) {
			return `in "${conversationTitle}"`;
		}
		return '';
	});

	let actionText = $derived(() => {
		switch (context) {
			case 'thread':
				return null; // No action needed, just type
			case 'conversation':
				return 'Create a thread';
			case 'none':
			default:
				return 'New conversation';
		}
	});

	function handleAction() {
		if (context === 'none') {
			onCreateConversation?.();
		}
		// Other contexts handled by parent
	}
</script>

<div
	class={[
		styles.container,
		hasInputActivity && styles.active,
		isTransitioning && styles.transitioning,
		context && styles[context]
	]
		.filter(Boolean)
		.join(' ')}
	aria-live="polite"
	aria-atomic="true"
>
	<!-- Icon with subtle animation -->
	<div class={styles.iconWrapper}>
		<div class={styles.emptyIcon}>
			{#if context === 'none'}
				<!-- Sparkle icon for new possibilities -->
				<svg
					width="28"
					height="28"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="1.5"
				>
					<path d="M12 3v18" />
					<path d="M3 12h18" />
					<path d="M5.6 5.6l12.8 12.8" />
					<path d="M18.4 5.6L5.6 18.4" />
					<circle cx="12" cy="12" r="3" />
				</svg>
			{:else if context === 'conversation'}
				<!-- Thread icon for branching -->
				<svg
					width="28"
					height="28"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="1.5"
				>
					<circle cx="12" cy="5" r="2" />
					<circle cx="12" cy="12" r="2" />
					<circle cx="12" cy="19" r="2" />
					<path d="M12 7v2" />
					<path d="M12 14v2" />
				</svg>
			{:else}
				<!-- Message icon for thread context -->
				<svg
					width="28"
					height="28"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="1.5"
				>
					<path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
					<path d="M8 10h.01" />
					<path d="M12 10h.01" />
					<path d="M16 10h.01" />
				</svg>
			{/if}
		</div>
	</div>

	<!-- Headline with optional subtext -->
	<h2 class={styles.headline}>
		{#if !hasInputActivity}
			{headlineText()}
			{#if subText()}
				<span class={styles.subText}>{subText()}</span>
			{/if}
		{:else}
			<span class={styles.fadingText}>Ready when you are</span>
		{/if}
	</h2>

	<!-- Hint text -->
	<p class={styles.hint}>
		{#if !hasInputActivity}
			{hintText()}
		{:else}
			Press Enter to send your message
		{/if}
	</p>

	<!-- Action button -->
	{#if actionText() && !hasInputActivity}
		<button class={styles.actionButton} onclick={handleAction} type="button">
			{actionText()}
			<svg
				width="14"
				height="14"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
			>
				<path d="M5 12h14" />
				<path d="M12 5l7 7-7 7" />
			</svg>
		</button>
	{/if}
</div>
