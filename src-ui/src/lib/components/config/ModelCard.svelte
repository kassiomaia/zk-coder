<script lang="ts">
    /**
     * ModelCard Component
     * Clickable card for selecting a model as default for an operation.
     */
    import styles from './ModelCard.module.css';
    import type {Model} from '$lib/types/config';

    interface Props {
		model: Model;
		selected?: boolean;
		onclick?: () => void;
	}

	let { model, selected = false, onclick }: Props = $props();

	let cardClasses = $derived([styles.card, selected && styles.selected].filter(Boolean).join(' '));
</script>

<button
	class={cardClasses}
	{onclick}
	type="button"
	aria-pressed={selected}
	aria-label="Select {model.name}"
>
	<div class={styles.modelInfo}>
		<span class={styles.modelName}>{model.name}</span>
		<span class={styles.modelId}>{model.id}</span>
	</div>

	{#if selected}
		<div class={styles.checkIcon}>
			<svg
				xmlns="http://www.w3.org/2000/svg"
				width="16"
				height="16"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
			>
				<polyline points="20 6 9 17 4 12" />
			</svg>
		</div>
	{/if}

	{#if model.capabilities.vision || model.capabilities.audio || model.capabilities.streaming}
		<div class={styles.capabilities}>
			{#if model.capabilities.vision}
				<span class={styles.capability} title="Supports vision/images">
					<svg
						xmlns="http://www.w3.org/2000/svg"
						width="12"
						height="12"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
					>
						<path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
						<circle cx="12" cy="12" r="3" />
					</svg>
				</span>
			{/if}
			{#if model.capabilities.audio}
				<span class={styles.capability} title="Supports audio">
					<svg
						xmlns="http://www.w3.org/2000/svg"
						width="12"
						height="12"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
					>
						<path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z" />
						<path d="M19 10v2a7 7 0 0 1-14 0v-2" />
						<line x1="12" y1="19" x2="12" y2="23" />
						<line x1="8" y1="23" x2="16" y2="23" />
					</svg>
				</span>
			{/if}
			{#if model.capabilities.streaming}
				<span class={styles.capability} title="Supports streaming">
					<svg
						xmlns="http://www.w3.org/2000/svg"
						width="12"
						height="12"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
					>
						<polyline points="22 12 18 12 15 21 9 3 6 12 2 12" />
					</svg>
				</span>
			{/if}
		</div>
	{/if}
</button>

<style>
	/* Styles are in ModelCard.module.css */
</style>
