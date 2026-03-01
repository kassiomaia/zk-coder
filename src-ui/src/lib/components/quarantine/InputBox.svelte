<script lang="ts">
	/**
	 * InputBox Component
	 * Multi-line text input for composing messages with attachment support.
	 */
	import styles from './InputBox.module.css';

	interface FileAttachment {
		id: string;
		name: string;
		type: string;
		size: number;
		preview?: string;
	}

	interface Props {
		/** Placeholder text */
		placeholder?: string;

		/** Current input value */
		value: string;

		/** Value change handler */
		onchange: (value: string) => void;

		/** Submit handler */
		onsubmit: () => void;

		/** Disabled state */
		disabled?: boolean;

		/** Loading state */
		loading?: boolean;

		/** Attached files */
		attachments?: FileAttachment[];

		/** File attach handler */
		onattach?: (files: File[]) => void;

		/** Remove attachment handler */
		onremoveattachment?: (id: string) => void;

		/** Maximum character limit */
		maxLength?: number;
	}

	let {
		placeholder = 'Type your message...',
		value,
		onchange,
		onsubmit,
		disabled = false,
		loading = false,
		attachments = [],
		onattach,
		onremoveattachment,
		maxLength
	}: Props = $props();

	let textareaRef: HTMLTextAreaElement | undefined = $state();
	let fileInputRef: HTMLInputElement | undefined = $state();

	// Auto-resize textarea
	$effect(() => {
		if (textareaRef) {
			textareaRef.style.height = 'auto';
			textareaRef.style.height = `${Math.min(textareaRef.scrollHeight, 200)}px`;
		}
	});

	function handleInput(e: Event) {
		const target = e.target as HTMLTextAreaElement;
		onchange(target.value);
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault();
			if (!disabled && !loading && value.trim()) {
				onsubmit();
			}
		}
	}

	function handleFileSelect(e: Event) {
		const target = e.target as HTMLInputElement;
		if (target.files && onattach) {
			onattach(Array.from(target.files));
			target.value = '';
		}
	}

	function formatSize(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
		return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
	}

	let characterCount = $derived(value.length);
	let isOverLimit = $derived(maxLength !== undefined && characterCount > maxLength);
	let canSubmit = $derived(!disabled && !loading && value.trim() && !isOverLimit);

	let containerClasses = $derived(
		[styles.container, disabled && styles.disabled].filter(Boolean).join(' ')
	);
</script>

<div class={containerClasses}>
	<!-- Attachments -->
	{#if attachments.length > 0}
		<div class={styles.attachments}>
			{#each attachments as file (file.id)}
				<div class={styles.attachment}>
					{#if file.preview}
						<img src={file.preview} alt={file.name} class={styles.attachmentPreview} />
					{:else}
						<svg
							class={styles.fileIcon}
							width="16"
							height="16"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							stroke-linecap="round"
							stroke-linejoin="round"
						>
							<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
							<polyline points="14 2 14 8 20 8" />
						</svg>
					{/if}
					<span class={styles.attachmentName}>{file.name}</span>
					<span class={styles.attachmentSize}>{formatSize(file.size)}</span>
					<button
						class={styles.attachmentRemove}
						onclick={() => onremoveattachment?.(file.id)}
						aria-label="Remove {file.name}"
						type="button"
					>
						<svg
							width="14"
							height="14"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							stroke-linecap="round"
							stroke-linejoin="round"
						>
							<line x1="18" y1="6" x2="6" y2="18" />
							<line x1="6" y1="6" x2="18" y2="18" />
						</svg>
					</button>
				</div>
			{/each}
		</div>
	{/if}

	<!-- Input area -->
	<div class={styles.inputWrapper}>
		<textarea
			bind:this={textareaRef}
			class={styles.textarea}
			{placeholder}
			{value}
			{disabled}
			oninput={handleInput}
			onkeydown={handleKeydown}
			rows={1}
			aria-label="Message input"
		></textarea>
	</div>

	<!-- Actions bar -->
	<div class={styles.actions}>
		<div class={styles.leftActions}>
			{#if onattach}
				<button
					class={styles.actionButton}
					onclick={() => fileInputRef?.click()}
					{disabled}
					aria-label="Attach file"
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
						<path
							d="m21.44 11.05-9.19 9.19a6 6 0 0 1-8.49-8.49l8.57-8.57A4 4 0 1 1 18 8.84l-8.59 8.57a2 2 0 0 1-2.83-2.83l8.49-8.48"
						/>
					</svg>
				</button>
				<input
					bind:this={fileInputRef}
					type="file"
					multiple
					accept="image/*,.pdf,.txt,.md"
					onchange={handleFileSelect}
					class={styles.fileInput}
				/>
			{/if}
		</div>

		<div class={styles.rightActions}>
			{#if maxLength}
				<span
					class={[styles.characterCount, isOverLimit && styles.overLimit].filter(Boolean).join(' ')}
				>
					{characterCount}/{maxLength}
				</span>
			{/if}

			<button
				class={[styles.sendButton, !canSubmit && styles.disabled].filter(Boolean).join(' ')}
				onclick={onsubmit}
				disabled={!canSubmit}
				aria-label="Send message"
				type="button"
			>
				{#if loading}
					<svg
						class={styles.spinner}
						width="20"
						height="20"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
					>
						<circle cx="12" cy="12" r="10" stroke-dasharray="60" stroke-dashoffset="20">
							<animateTransform
								attributeName="transform"
								type="rotate"
								from="0 12 12"
								to="360 12 12"
								dur="1s"
								repeatCount="indefinite"
							/>
						</circle>
					</svg>
				{:else}
					<svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
						<path d="M2.01 21L23 12 2.01 3 2 10l15 2-15 2z" />
					</svg>
				{/if}
			</button>
		</div>
	</div>
</div>
