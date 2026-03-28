<script lang="ts">
	import { onMount } from 'svelte';
	import { Copy, Check } from 'lucide-svelte';
	import styles from './CodeBlock.module.css';

	interface Props {
		code: string;
		language?: string;
		highlightedCode?: string;
	}

	let { code, language = '', highlightedCode = '' }: Props = $props();

	let copied = $state(false);
	let copyTimeout: ReturnType<typeof setTimeout>;

	async function handleCopy() {
		try {
			await navigator.clipboard.writeText(code);
			copied = true;
			clearTimeout(copyTimeout);
			copyTimeout = setTimeout(() => {
				copied = false;
			}, 2000);
		} catch (err) {
			console.error('Failed to copy code:', err);
		}
	}

	// Determine if this is a request or response (subtle indicator)
	// We can use language or simple heuristic for now, 
	// or let the caller pass a 'type' if we had one.
	// For now, let's look for common patterns or just use 'default'.
	const isRequest = $derived(
		language.toLowerCase().includes('http') || 
		code.trim().startsWith('GET ') || 
		code.trim().startsWith('POST ')
	);
	
	const isResponse = $derived(
		code.trim().startsWith('{') || 
		code.trim().startsWith('[') ||
		language.toLowerCase() === 'json'
	);

	const indicatorClass = $derived.by(() => {
		if (isRequest) return styles.indicatorRequest;
		if (isResponse) return styles.indicatorResponse;
		return '';
	});

	const displayLanguage = $derived(
		language && language !== 'auto' ? language : 'code'
	);
</script>

<div 
	class={[styles.container, indicatorClass].filter(Boolean).join(' ')}
	data-language={displayLanguage}
>
	<div class={styles.header}>
		<span class={styles.language}>{displayLanguage}</span>
		<button 
			class={styles.copyButton} 
			onclick={handleCopy}
			aria-label={copied ? 'Copied' : 'Copy code'}
			title={copied ? 'Copied' : 'Copy code'}
		>
			{#if copied}
				<Check size={14} strokeWidth={2.5} />
				<span class={styles.copyText}>Copied</span>
			{:else}
				<Copy size={14} strokeWidth={2.5} />
				<span class={styles.copyText}>Copy</span>
			{/if}
		</button>
	</div>
	<div class={styles.viewport}>
		<pre class={styles.pre}><code class="hljs">{@html highlightedCode || code}</code></pre>
	</div>
</div>
