<script lang="ts">
	/**
	 * EnhancedCodeBlock Component
	 *
	 * Productized code block with strong visual hierarchy.
	 * Integrated with design system for consistent styling.
	 *
	 * Features:
	 * - Clear header with filename and language display
	 * - Line numbers with proper contrast
	 * - Syntax highlighting aligned with design system
	 * - Copy button with feedback animation
	 * - Collapsible for long blocks
	 */
	import styles from './CodeBlock.module.css';
	import { Check, Copy, ChevronDown, ChevronRight } from 'lucide-svelte';

	interface Props {
		/** Code content */
		code: string;
		/** Programming language */
		language?: string;
		/** Filename for display */
		filename?: string;
		/** Starting line number */
		startLine?: number;
		/** Whether to show line numbers */
		showLineNumbers?: boolean;
		/** Maximum height for scrollable blocks */
		maxHeight?: string;
		/** Copy button handler */
		onCopy?: () => void | Promise<void>;
	}

	let {
		code,
		language = '',
		filename,
		startLine = 1,
		showLineNumbers = true,
		maxHeight,
		onCopy
	}: Props = $props();

	// Internal state
	let isCollapsed = $state(false);
	let isCopying = $state(false);
	let copySuccess = $state(false);

	// Derived values
	let hasHeader = $derived(filename || language);
	let lineCount = $derived(code.split('\n').length);
	let shouldCollapse = $derived(lineCount > 20);

	// Format language for display
	function formatLanguage(lang: string): string {
		if (!lang) return 'text';
		const langMap: Record<string, string> = {
			js: 'JavaScript',
			ts: 'TypeScript',
			jsx: 'JavaScript (React)',
			tsx: 'TypeScript (React)',
			py: 'Python',
			rs: 'Rust',
			go: 'Go',
			java: 'Java',
			c: 'C',
			cpp: 'C++',
			cs: 'C#',
			php: 'PHP',
			rb: 'Ruby',
			sh: 'Shell',
			bash: 'Bash',
			zsh: 'Zsh',
			sql: 'SQL',
			html: 'HTML',
			css: 'CSS',
			scss: 'SCSS',
			json: 'JSON',
			xml: 'XML',
			yaml: 'YAML',
			yml: 'YAML',
			toml: 'TOML',
			markdown: 'Markdown',
			md: 'Markdown'
		};
		return langMap[lang.toLowerCase()] || lang;
	}

	// Generate line numbers
	function generateLineNumbers(): number[] {
		return Array.from({ length: lineCount }, (_, i) => startLine + i);
	}

	// Handle copy action
	async function handleCopy() {
		if (isCopying) return;

		isCopying = true;
		try {
			await navigator.clipboard.writeText(code);
			copySuccess = true;
			onCopy?.();
			setTimeout(() => {
				copySuccess = false;
			}, 2000);
		} catch (error) {
			console.warn('[CodeBlock] Failed to copy:', error);
		} finally {
			isCopying = false;
		}
	}

	// Handle toggle collapse
	function handleToggle() {
		isCollapsed = !isCollapsed;
	}
</script>

<div class={[styles.codeBlock, isCollapsed && styles.collapsed].filter(Boolean).join(' ')}>
	<!-- Header with filename and language -->
	{#if hasHeader}
		<header class={styles.header}>
			<div class={styles.headerLeft}>
				{#if filename}
					<span class={styles.filename}>{filename}</span>
					{#if language}
						<span class={styles.separator}>·</span>
					{/if}
				{/if}
				{#if language}
					<span class={styles.language}>{formatLanguage(language)}</span>
				{/if}
			</div>

			<div class={styles.headerRight}>
				{#if shouldCollapse}
					<button
						class={styles.collapseBtn}
						onclick={handleToggle}
						type="button"
						title={isCollapsed ? 'Expand code' : 'Collapse code'}
					>
						{#if isCollapsed}
							<ChevronRight size={14} strokeWidth={2} />
						{:else}
							<ChevronDown size={14} strokeWidth={2} />
						{/if}
					</button>
				{/if}

				<button
					class={[styles.copyBtn, copySuccess && styles.copied].join(' ')}
					onclick={handleCopy}
					disabled={isCopying}
					type="button"
					title={copySuccess ? 'Copied!' : 'Copy code'}
				>
					{#if copySuccess}
						<Check size={14} strokeWidth={2} />
					{:else}
						<Copy size={14} strokeWidth={2} />
					{/if}
				</button>
			</div>
		</header>
	{/if}

	<!-- Code content -->
	{#if !isCollapsed}
		<div class={styles.content} style:max-height={maxHeight}>
			{#if showLineNumbers}
				<div class={styles.lineNumbers}>
					{#each generateLineNumbers() as line}
						<span class={styles.lineNumber}>{line}</span>
					{/each}
				</div>
			{/if}

			<pre class={styles.pre}><code class={styles.code}>{code}</code></pre>
		</div>
	{:else}
		<div class={styles.collapsedPreview}>
			<span class={styles.lineCount}>{lineCount} lines</span>
			<span class={styles.ellipsis}>⋯</span>
		</div>
	{/if}
</div>
