<script lang="ts">
	type BadgeVariant = 'success' | 'warning' | 'error' | 'info' | 'default' | 'mono' | 'tag';

	interface Props {
		/** Badge variant */
		variant?: BadgeVariant;
		/** Badge text */
		text: string;
		/** Optional size */
		size?: 'sm' | 'md';
		/** Additional class */
		class?: string;
	}

	let { variant = 'default', text, size = 'md', class: className = '' }: Props = $props();
</script>

<span class="status-badge {variant} {size} {className}">{text}</span>

<style>
	.status-badge {
		display: inline-flex;
		align-items: center;
		padding: var(--space-0-5) var(--space-2);
		border-radius: var(--radius-sm);
		font-size: var(--text-xs);
		font-weight: 500;
		white-space: nowrap;
	}

	.sm {
		padding: var(--space-0-5) var(--space-1-5);
		font-size: 10px;
	}

	.default {
		background: var(--color-bg-tertiary);
		color: var(--color-text-secondary);
		border: 1px solid var(--color-border-subtle);
	}

	.success {
		background: var(--color-success-subtle);
		color: var(--color-success-dark);
		border: 1px solid var(--color-success-border);
	}

	.warning {
		background: var(--color-warning-light);
		color: var(--color-warning-dark);
		border: 1px solid var(--color-warning-border);
	}

	.error {
		background: var(--color-error-light);
		color: var(--color-error-dark);
		border: 1px solid var(--color-error-border);
	}

	.info {
		background: var(--color-primary-subtle);
		color: var(--color-primary-600);
		border: 1px solid var(--color-primary-200);
	}

	.mono {
		background: var(--color-bg-secondary);
		border-color: var(--color-border-primary);
		color: var(--color-text-tertiary);
		font-family: var(--font-mono);
	}

	.tag {
		background: var(--color-bg-tertiary);
		border-color: var(--color-border-subtle);
		color: var(--color-text-secondary);
		font-family: monospace;
	}
</style>
