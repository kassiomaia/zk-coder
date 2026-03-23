<script lang="ts">
	/**
	 * DiffViewer Component
	 * Renders unified diff output with syntax-colored lines.
	 * Added lines are highlighted green, removed lines red, and context lines neutral.
	 */

	interface Props {
		/** Unified diff content (e.g., from `similar` crate output) */
		diff: string;
		/** Maximum height in pixels before scrolling */
		maxHeight?: number;
	}

	let { diff, maxHeight = 300 }: Props = $props();

	interface DiffLine {
		type: 'add' | 'remove' | 'context' | 'header';
		content: string;
		oldLineNum?: number;
		newLineNum?: number;
	}

	let lines = $derived(parseDiff(diff));

	function parseDiff(raw: string): DiffLine[] {
		const result: DiffLine[] = [];
		let oldLine = 0;
		let newLine = 0;

		for (const line of raw.split('\n')) {
			if (line.startsWith('@@')) {
				// Parse hunk header: @@ -old,count +new,count @@
				const match = line.match(/@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/);
				if (match) {
					oldLine = parseInt(match[1], 10);
					newLine = parseInt(match[2], 10);
				}
				result.push({ type: 'header', content: line });
			} else if (line.startsWith('---') || line.startsWith('+++')) {
				result.push({ type: 'header', content: line });
			} else if (line.startsWith('+')) {
				result.push({
					type: 'add',
					content: line.slice(1),
					newLineNum: newLine
				});
				newLine++;
			} else if (line.startsWith('-')) {
				result.push({
					type: 'remove',
					content: line.slice(1),
					oldLineNum: oldLine
				});
				oldLine++;
			} else if (line.length > 0) {
				// Context line (starts with space or no prefix)
				const content = line.startsWith(' ') ? line.slice(1) : line;
				result.push({
					type: 'context',
					content,
					oldLineNum: oldLine,
					newLineNum: newLine
				});
				oldLine++;
				newLine++;
			}
		}

		return result;
	}
</script>

<div class="diff-viewer" style:max-height="{maxHeight}px">
	<table class="diff-table">
		<tbody>
			{#each lines as line, i (i)}
				<tr class="diff-line {line.type}">
					<td class="line-num old">
						{#if line.type === 'remove' || line.type === 'context'}
							{line.oldLineNum ?? ''}
						{/if}
					</td>
					<td class="line-num new">
						{#if line.type === 'add' || line.type === 'context'}
							{line.newLineNum ?? ''}
						{/if}
					</td>
					<td class="line-marker">
						{#if line.type === 'add'}+{:else if line.type === 'remove'}-{:else if line.type === 'context'}&nbsp;{/if}
					</td>
					<td class="line-content">
						<pre>{line.content}</pre>
					</td>
				</tr>
			{/each}
		</tbody>
	</table>
</div>

<style>
	.diff-viewer {
		overflow: auto;
		background: var(--color-neutral-950);
		border-radius: var(--radius-sm);
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		line-height: var(--leading-normal);
	}

	.diff-table {
		width: 100%;
		border-collapse: collapse;
		table-layout: fixed;
	}

	.diff-line {
		border: none;
	}

	.diff-line.add {
		background: var(--color-success-subtle);
	}

	.diff-line.remove {
		background: var(--color-error-light);
	}

	.diff-line.header {
		background: var(--color-surface);
		color: var(--color-text-muted);
	}

	.diff-line.context {
		background: transparent;
	}

	.line-num {
		width: 40px;
		min-width: 40px;
		padding: 0 var(--space-2);
		text-align: right;
		color: var(--color-text-muted);
		user-select: none;
		white-space: nowrap;
		vertical-align: top;
		opacity: 0.6;
	}

	.line-marker {
		width: 16px;
		min-width: 16px;
		padding: 0;
		text-align: center;
		user-select: none;
		vertical-align: top;
	}

	.diff-line.add .line-marker {
		color: var(--color-success);
	}

	.diff-line.remove .line-marker {
		color: var(--color-error);
	}

	.line-content {
		padding: 0 var(--space-2);
		white-space: pre;
		color: var(--color-text-primary);
	}

	.line-content pre {
		margin: 0;
		font: inherit;
		white-space: pre;
	}

	/* Scrollbar */
	.diff-viewer::-webkit-scrollbar {
		width: 6px;
		height: 6px;
	}

	.diff-viewer::-webkit-scrollbar-track {
		background: transparent;
	}

	.diff-viewer::-webkit-scrollbar-thumb {
		background: var(--color-neutral-700);
		border-radius: var(--radius-full);
	}
</style>
