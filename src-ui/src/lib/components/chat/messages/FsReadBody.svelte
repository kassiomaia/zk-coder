<script lang="ts">
	import styles from './FsReadBody.module.css';

	type ToolCallStatus = 'pending' | 'success' | 'error';

	interface Props {
		args: { filePath?: string; path?: string; startLine?: number; endLine?: number } | undefined;
		result: { content?: string } | undefined;
		status: ToolCallStatus;
	}

	let { args, result, status }: Props = $props();

	const resolvedPath = $derived(args?.filePath ?? args?.path ?? '');
	const lastSlash = $derived(resolvedPath.lastIndexOf('/'));
	const dirPart = $derived(lastSlash >= 0 ? resolvedPath.slice(0, lastSlash + 1) : '');
	const namePart = $derived(lastSlash >= 0 ? resolvedPath.slice(lastSlash + 1) : resolvedPath);

	const ext = $derived(
		(() => {
			const dot = namePart.lastIndexOf('.');
			return dot >= 0 ? namePart.slice(dot) : '';
		})()
	);

	const langColor = $derived(
		(() => {
			switch (ext) {
				case '.rs':
					return '#f74c00';
				case '.ts':
				case '.tsx':
					return '#3178c6';
				case '.svelte':
					return '#ff3e00';
				case '.js':
				case '.jsx':
					return '#f7df1e';
				case '.toml':
				case '.yaml':
				case '.yml':
					return '#8fb47a';
				case '.md':
					return '#a3a29a';
				case '.json':
					return '#c29a6e';
				default:
					return 'var(--color-text-muted)';
			}
		})()
	);

	const lineRange = $derived(
		(() => {
			if (args?.startLine !== undefined && args?.endLine !== undefined) {
				return `:${args.startLine}–${args.endLine}`;
			}
			if (args?.startLine !== undefined) {
				return `:${args.startLine}`;
			}
			return '';
		})()
	);

	const lineCount = $derived(
		result?.content !== undefined ? result.content.split('\n').length : undefined
	);
</script>

{#if resolvedPath}
	<ul class={styles.fileList}>
		<li class={styles.fileRow}>
			<div class={styles.fileIcon}>
				<svg
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="1.5"
					width="13"
					height="13"
					aria-hidden="true"
				>
					<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
					<polyline points="14 2 14 8 20 8"></polyline>
				</svg>
				<span class={styles.langDot} style="background: {langColor}"></span>
			</div>

			<div class={styles.filePath}>
				{#if dirPart}
					<span class={styles.pathDir}>{dirPart}</span>
				{/if}
				<span class={styles.pathName}>{namePart}</span>
				{#if lineRange}
					<span class={styles.lineRange}>{lineRange}</span>
				{/if}
			</div>

			{#if lineCount !== undefined}
				<span class={styles.lineCount}>{lineCount} lines</span>
			{/if}

			<div class={styles.rowArrow}>&#8250;</div>
		</li>
	</ul>
{/if}
