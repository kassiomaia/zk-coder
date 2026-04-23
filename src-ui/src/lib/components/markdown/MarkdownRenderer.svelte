<script lang="ts">
	/**
	 * MarkdownRenderer Component
	 *
	 * Renders markdown content with syntax highlighting, tables, task lists,
	 * and image support. Validates and sanitizes input for XSS protection.
	 *
	 * Performance notes:
	 * - Uses `renderMarkdownWithBlocks` to split markdown into html parts and
	 *   extracted code blocks via sentinel comments. No DOM round-trip.
	 * - Memoizes the rendered result by input content so streaming tokens
	 *   don't re-run marked + hljs on every reactive update for stable messages.
	 */
	import { browser } from '$app/environment';
	import {
		escapeHtml,
		renderMarkdownWithBlocks,
		splitOnSentinels,
		type CodeBlockData,
		type RenderedMarkdown,
		type RenderedPart
	} from '$lib/utils/markdown';
	import CodeBlock from './code/CodeBlock.svelte';
	import {
		type ContentValidationConfig,
		defaultValidationConfig,
		mightNeedSanitization,
		validateContent
	} from '$lib/utils/contentValidation';
	import styles from './MarkdownRenderer.module.css';

	interface Props {
		/** Raw markdown content */
		content: string;
		/** Additional CSS class */
		class?: string;
		/**
		 * Validation configuration options.
		 * Set to null to disable validation (use only for trusted content).
		 */
		validationConfig?: ContentValidationConfig | null;
		/** Whether to skip validation for trusted content (default: false) */
		skipValidation?: boolean;
	}

	let {
		content,
		class: className = '',
		validationConfig = {},
		skipValidation = false
	}: Props = $props();

	let validationResult = $derived.by(() => {
		if (skipValidation || validationConfig === null) {
			return {
				isValid: true,
				sanitizedContent: content ?? '',
				error: undefined,
				warnings: []
			};
		}

		if (!mightNeedSanitization(content)) {
			return {
				isValid: true,
				sanitizedContent: content ?? '',
				error: undefined,
				warnings: []
			};
		}

		const mergedConfig = { ...defaultValidationConfig, ...validationConfig };
		return validateContent(content, mergedConfig);
	});

	// Per-instance memoization. Streaming only mutates the trailing message's
	// `content`, so non-streaming messages hit this cache and skip marked + hljs
	// entirely on re-renders caused by surrounding reactivity.
	let lastInput: string | null = null;
	let lastRendered: RenderedMarkdown | null = null;

	let rendered = $derived.by<RenderedMarkdown>(() => {
		if (!validationResult.isValid) {
			const errMsg = escapeHtml(validationResult.error ?? 'Content validation failed');
			return {
				html: `<div class="${styles.validationError}" role="alert"><span class="${styles.errorIcon}">!</span><span class="${styles.errorMessage}">${errMsg}</span></div>`,
				codeBlocks: []
			};
		}

		const sanitized = validationResult.sanitizedContent;

		if (!browser) {
			return {
				html: escapeHtml(sanitized).replace(/\n/g, '<br>'),
				codeBlocks: []
			};
		}

		if (lastInput !== null && lastInput === sanitized && lastRendered) {
			return lastRendered;
		}

		const result = renderMarkdownWithBlocks(sanitized);
		lastInput = sanitized;
		lastRendered = result;
		return result;
	});

	let parts = $derived<RenderedPart[]>(splitOnSentinels(rendered.html, rendered.codeBlocks));
</script>

<div class={`${styles.markdown} ${className}`}>
	{#if !browser}
		{@html rendered.html}
	{:else}
		{#each parts as part, i (i)}
			{#if part.type === 'code'}
				{@const data = part.data as CodeBlockData}
				<CodeBlock
					code={data.code}
					language={data.language}
					highlightedCode={data.highlightedHtml}
				/>
			{:else}
				{@html part.html}
			{/if}
		{/each}
	{/if}
</div>
