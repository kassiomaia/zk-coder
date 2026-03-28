<script lang="ts">
	/**
	 * ArtifactCard Component
	 *
	 * Dedicated visual container for structured agent outputs.
	 * Distinguishes code, tables, diagrams, and other artifacts from inline message content.
	 *
	 * Design Philosophy:
	 * - Artifacts feel like meaningful outputs, not inline content
	 * - Subtle grouping with optional headers
	 * - Contextual actions integrated cleanly
	 * - Consistent with workspace aesthetic
	 */

	import type { Snippet } from 'svelte';

	interface Props {
		/** Type of artifact for appropriate styling */
		type?: 'code' | 'json' | 'table' | 'diagram' | 'terminal' | 'diff';
		/** Optional title/header for the artifact */
		title?: string;
		/** Optional language badge (for code artifacts) */
		language?: string;
		/** Whether to show copy button */
		showCopy?: boolean;
		/** Whether to show expand/collapse */
		collapsible?: boolean;
		/** Initial collapsed state */
		initiallyCollapsed?: boolean;
		/** Artifact content */
		children: Snippet;
		/** Copy callback */
		onCopy?: () => Promise<void>;
		/** Delete/remove callback */
		onRemove?: () => void;
	}

	let {
		type = 'code',
		title,
		language,
		showCopy = true,
		collapsible = false,
		initiallyCollapsed = false,
		children,
		onCopy,
		onRemove
	}: Props = $props();

	let isCollapsed = $state(initiallyCollapsed);
	let isCopied = $state(false);

	async function handleCopy() {
		if (onCopy) {
			await onCopy();
		}
		isCopied = true;
		setTimeout(() => (isCopied = false), 2000);
	}

	function handleToggle() {
		if (collapsible) {
			isCollapsed = !isCollapsed;
		}
	}

	// Icon for artifact type
	const typeIcon = $derived(() => {
		switch (type) {
			case 'terminal':
				return `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="4 17 10 11 4 5"/><line x1="12" y1="19" x2="20" y2="19"/></svg>`;
			case 'json':
			case 'code':
				return `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="16 18 22 12 16 6"/><polyline points="8 6 2 12 8 18"/></svg>`;
			case 'table':
				return `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="3" width="18" height="18" rx="2"/><line x1="3" y1="9" x2="21" y2="9"/><line x1="3" y1="15" x2="21" y2="15"/><line x1="9" y1="3" x2="9" y2="21"/><line x1="15" y1="3" x2="15" y2="21"/></svg>`;
			case 'diff':
				return `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="9" y1="15" x2="15" y2="15"/><line x1="12" y1="12" x2="12" y2="18"/></svg>`;
			default:
				return `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="8.5" cy="8.5" r="1.5"/><polyline points="21 15 16 10 5 21"/></svg>`;
		}
	});
</script>

<div class="artifact-card" class:collapsed={isCollapsed} class:collapsible>
	<!-- Header -->
	{#if title || language || showCopy || collapsible}
		<div class="artifact-header">
			<div class="artifact-header-left">
				{#if type}
					<span class="artifact-type-icon">{@html typeIcon()}</span>
				{/if}
				{#if title}
					<span class="artifact-title">{title}</span>
				{/if}
				{#if language}
					<span class="artifact-language">{language}</span>
				{/if}
			</div>
			<div class="artifact-header-right">
				{#if showCopy}
					<button
						class="artifact-action"
						class:copied={isCopied}
						onclick={handleCopy}
						title={isCopied ? 'Copied!' : 'Copy'}
						type="button"
					>
						{#if isCopied}
							<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
								<polyline points="20 6 9 17 4 12"/>
							</svg>
						{:else}
							<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
								<rect x="9" y="9" width="13" height="13" rx="2" ry="2"/>
								<path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
							</svg>
						{/if}
					</button>
				{/if}
				{#if onRemove}
					<button
						class="artifact-action artifact-action-remove"
						onclick={onRemove}
						title="Remove"
						type="button"
					>
						<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
							<line x1="18" y1="6" x2="6" y2="18"/>
							<line x1="6" y1="6" x2="18" y2="18"/>
						</svg>
					</button>
				{/if}
				{#if collapsible}
					<button
						class="artifact-action"
						class:expanded={!isCollapsed}
						onclick={handleToggle}
						title={isCollapsed ? 'Expand' : 'Collapse'}
						type="button"
					>
						<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
							<polyline points="6 9 12 15 18 9"/>
						</svg>
					</button>
				{/if}
			</div>
		</div>
	{/if}

	<!-- Content -->
	<div class="artifact-content" class:collapsed={isCollapsed}>
		{@render children()}
	</div>
</div>

<style>
	.artifact-card {
		--artifact-bg: var(--color-bg-tertiary);
		--artifact-border: var(--color-border-subtle);
		--artifact-hover: var(--color-interactive-hover);

		background: var(--artifact-bg);
		border: 1px solid var(--artifact-border);
		border-radius: var(--radius-lg);
		overflow: hidden;
		transition:
			border-color var(--transition-fast),
			box-shadow var(--transition-fast);
		margin-top: var(--space-2);
		margin-bottom: var(--space-3);
	}

	.artifact-card:hover {
		border-color: var(--color-border-primary);
		box-shadow: var(--shadow-xs);
	}

	/* =====================
	   Header
	   ===================== */

	.artifact-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-3);
		padding: var(--space-2) var(--space-3);
		background: var(--color-bg-secondary);
		border-bottom: 1px solid var(--artifact-border);
		min-height: 36px;
		user-select: none;
	}

	.artifact-header-left {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		flex: 1;
		min-width: 0;
	}

	.artifact-type-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--color-text-tertiary);
		flex-shrink: 0;
	}

	.artifact-title {
		font-size: var(--text-xs);
		font-weight: var(--font-medium);
		color: var(--color-text-secondary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.artifact-language {
		font-size: var(--text-2xs);
		font-weight: var(--font-semibold);
		color: var(--color-text-tertiary);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		background: var(--color-interactive-selected);
		padding: 2px 6px;
		border-radius: var(--radius-sm);
	}

	.artifact-header-right {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		flex-shrink: 0;
	}

	/* =====================
	   Actions
	   ===================== */

	.artifact-action {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 28px;
		height: 28px;
		padding: 0;
		background: transparent;
		border: none;
		border-radius: var(--radius-sm);
		color: var(--color-text-muted);
		cursor: pointer;
		transition: all var(--transition-fast);
	}

	.artifact-action:hover {
		background: var(--artifact-hover);
		color: var(--color-text-secondary);
	}

	.artifact-action:focus-visible {
		outline: 2px solid var(--color-accent-primary);
		outline-offset: 1px;
	}

	.artifact-action.copied {
		color: var(--color-success);
	}

	.artifact-action.expanded svg {
		transform: rotate(180deg);
	}

	.artifact-action-remove:hover {
		color: var(--color-error);
		background: var(--color-error-light);
	}

	/* =====================
	   Content
	   ===================== */

	.artifact-content {
		padding: var(--space-3);
		overflow-x: auto;
		transition: opacity var(--transition-normal) var(--ease-out);
	}

	.artifact-content.collapsed {
		display: none;
	}

	/* =====================
	   Collapsed State
	   ===================== */

	.artifact-card.collapsed .artifact-header {
		border-bottom-color: transparent;
	}

	.artifact-card.collapsed .artifact-header::after {
		content: '';
		position: absolute;
		bottom: -1px;
		left: 0;
		right: 0;
		height: 1px;
		background: var(--artifact-border);
	}

	/* =====================
	   Type-Specific Styling
	   ===================== */

	/* Terminal artifacts get monospace treatment */
	.artifact-card:has(.artifact-language:contains("Terminal")) {
		font-family: var(--font-mono);
	}

	/* =====================
	   Reduced Motion
	   ===================== */

	@media (prefers-reduced-motion: reduce) {
		.artifact-card,
		.artifact-action,
		.artifact-content {
			transition: none;
		}

		.artifact-action.expanded svg {
			transform: rotate(0deg);
		}
	}
</style>
