<script lang="ts">
    /**
     * ChatInputContainer Component
     * Wraps ChatInput with state management and animations.
     */
    import ChatInput from './input/ChatInput.svelte';
    import styles from './ChatInputContainer.module.css';

    interface Props {
		/** Current input value (bindable) */
		value: string;

		/** Value change handler */
		onchange: (value: string) => void;

		/** Submit handler */
		onsubmit: () => void;

		/** Focus handler */
		onfocus?: () => void;

		/** Blur handler */
		onblur?: () => void;

		/** Placeholder text */
		placeholder?: string;

		/** Loading state */
		loading?: boolean;

		/** Whether the thread-linked agent session is active */
		isAgentWorking?: boolean;

		/** Session ID linked to the visible thread */
		agentSessionId?: string | null;

		/** Whether thread is resolved (hides input with animation) */
		threadResolved?: boolean;

		/** Selected model ID */
		modelId?: string | null;

		/** Model change handler */
		onModelChange?: (modelId: string) => void;

		/** Open settings handler */
		onOpenSettings?: () => void;
	}

	let {
		value = $bindable(),
		onchange,
		onsubmit,
		onfocus,
		onblur,
		placeholder = 'Send a message...',
		loading = false,
		isAgentWorking = false,
		agentSessionId = null,
		threadResolved = false,
		modelId,
		onModelChange,
		onOpenSettings
	}: Props = $props();

	// Animation state for input container
	let inputAnimatingIn = $state(false);
	let inputAnimatingOut = $state(false);
	let showInput = $state(true);
	let previousResolved = $state(false);

	// Watch for threadResolved changes and animate transitions
	$effect(() => {
		const currentResolved = threadResolved;

		// First run - initialize state without animation
		if (previousResolved === false && !inputAnimatingIn && !inputAnimatingOut) {
			showInput = !currentResolved;
			previousResolved = currentResolved;
			return;
		}

		// Only animate when the resolved state actually changes
		if (currentResolved !== previousResolved) {
			const wasResolved = previousResolved;
			const nowResolved = currentResolved;

			previousResolved = currentResolved;

			if (wasResolved && !nowResolved) {
				// Transition: locked -> unlocked
				// Wait for banner to slide out (200ms), then slide input up
				setTimeout(() => {
					showInput = true;
					inputAnimatingIn = true;
					setTimeout(() => {
						inputAnimatingIn = false;
					}, 200);
				}, 200);
			} else if (!wasResolved && nowResolved) {
				// Transition: unlocked -> locked
				// Animate input out (slide down), then banner will slide in
				inputAnimatingOut = true;
				setTimeout(() => {
					showInput = false;
					inputAnimatingOut = false;
				}, 150);
			}
		}
	});
</script>

{#if showInput}
	<div
		class={[styles.inputContainer, inputAnimatingIn && styles.animateIn]
			.filter(Boolean)
			.join(' ')}
	>
		<ChatInput
			{value}
			onchange={onchange}
			onsubmit={onsubmit}
			onfocus={onfocus}
			onblur={onblur}
			{placeholder}
			{loading}
			{isAgentWorking}
			{agentSessionId}
			showIntent={true}
			modelId={modelId}
			onModelChange={onModelChange}
			onOpenSettings={onOpenSettings}
		/>
	</div>
{/if}
