<script lang="ts">
	/**
	 * EmptyThreadState Component
	 *
	 * A sophisticated empty state that transitions smoothly to active state.
	 * Features anticipatory animations, progressive disclosure, and layout reservation.
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
	}

	let {
		hasInputActivity = false,
		showSecondary = true,
		context = 'none',
		conversationTitle
	}: Props = $props();

	// Animation state management
	let isTransitioning = $state(false);
	let headlinePulsePhase = $state(0);

	// Derive display content based on context
	let headlineText = $derived(() => {
		switch (context) {
			case 'thread':
				return 'Start a conversation';
			case 'conversation':
				return 'Start a new thread';
			case 'none':
			default:
				return 'Select or create a conversation';
		}
	});

	let hintText = $derived(() => {
		switch (context) {
			case 'thread':
				return 'Type a message below to begin';
			case 'conversation':
				return 'Create a thread from the sidebar or send a message';
			case 'none':
			default:
				return 'Choose a conversation from the sidebar to get started';
		}
	});

	let subText = $derived(() => {
		if (context === 'conversation' && conversationTitle) {
			return `in "${conversationTitle}"`;
		}
		return '';
	});
</script>

<div
	class={[styles.container, hasInputActivity && styles.active, isTransitioning && styles.transitioning, context && styles[context]].filter(Boolean).join(' ')}
>
	<!-- Clean headline wrapper without pulse -->
	<div class={styles.headlineWrapper}>
		<!-- Contextual icon -->
		<div class={styles.emptyIcon}>
			{#if context === 'none'}
				<!-- Chat/conversation icon -->
				<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
					<path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
					<path d="M8 10h.01" />
					<path d="M12 10h.01" />
					<path d="M16 10h.01" />
				</svg>
			{:else if context === 'conversation'}
				<!-- Thread/fork icon -->
				<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
					<path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
					<path d="M12 7v5" />
					<path d="M9 9l3-2 3 2" />
				</svg>
			{:else}
				<!-- Default message icon -->
				<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
					<path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
					<path d="M8 10h.01" />
					<path d="M12 10h.01" />
					<path d="M16 10h.01" />
				</svg>
			{/if}
		</div>
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
	</div>

	<!-- Clean hint text -->
	<p class={styles.hint}>
		{#if !hasInputActivity}
			{hintText()}
		{:else}
			Press Enter to send
		{/if}
	</p>

	<!-- Context-specific action hint -->
	{#if context === 'none' && !hasInputActivity}
		<div class={styles.actionHint}>
			<span class={styles.actionText}>Or create a new conversation to get started</span>
		</div>
	{/if}
</div>
