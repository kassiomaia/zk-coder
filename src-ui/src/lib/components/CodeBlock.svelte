<script lang="ts">
	/**
	 * CodeBlock Component
	 * Displays syntax-highlighted code with copy functionality.
	 */
	import styles from './CodeBlock.module.css';

	interface Props {
		/** Programming language for syntax highlighting */
		language: string;

		/** Code content */
		code: string;

		/** Optional filename */
		filename?: string;

		/** Show line numbers */
		showLineNumbers?: boolean;

		/** Lines to highlight (1-indexed) */
		highlightedLines?: number[];

		/** Copy callback */
		onCopy?: () => void;

		/** Edit callback */
		onEdit?: () => void;
	}

	let {
		language,
		code,
		filename,
		showLineNumbers = true,
		highlightedLines = [],
		onCopy,
		onEdit
	}: Props = $props();

	let copied = $state(false);

	const lines = code.split('\n');

	async function handleCopy() {
		try {
			await navigator.clipboard.writeText(code);
			copied = true;
			onCopy?.();
			setTimeout(() => (copied = false), 2000);
		} catch (err) {
			console.error('Failed to copy code:', err);
		}
	}

	function isHighlighted(lineNum: number): boolean {
		return highlightedLines.includes(lineNum);
	}
</script>

<div class={styles.container}>
	<header class={styles.header}>
		<div class={styles.headerLeft}>
			<span class={styles.language}>{language}</span>
			{#if filename}
				<span class={styles.filename}>{filename}</span>
			{/if}
		</div>
		<div class={styles.actions}>
			{#if onEdit}
				<button class={styles.actionButton} onclick={onEdit} aria-label="Edit code" type="button">
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
						<path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" />
						<path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" />
					</svg>
				</button>
			{/if}
			<button class={styles.actionButton} onclick={handleCopy} aria-label="Copy code" type="button">
				{#if copied}
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
						<polyline points="20 6 9 17 4 12" />
					</svg>
				{:else}
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
						<rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
						<path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
					</svg>
				{/if}
				<span>{copied ? 'Copied!' : 'Copy'}</span>
			</button>
		</div>
	</header>

	<pre class={styles.code}><code
			>{#each lines as line, i}
				<div
					class={[styles.line, isHighlighted(i + 1) && styles.highlighted]
						.filter(Boolean)
						.join(' ')}>
      {#if showLineNumbers}
						<span class={styles.lineNumber} aria-hidden="true">{i + 1}</span>
					{/if}
      <span class={styles.lineContent}>{line}</span>
    </div>{/each}
  </code></pre>
</div>
