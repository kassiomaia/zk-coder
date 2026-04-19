<script lang="ts">
	import styles from './InlineDiff.module.css';

	interface Props {
		diff: string;
		filePath: string;
		maxRows?: number;
		collapsible?: boolean;
		initiallyOpen?: boolean;
	}

	let {
		diff,
		filePath,
		maxRows = 80,
		collapsible = false,
		initiallyOpen = true
	}: Props = $props();

	type DiffRowType = 'hunk' | 'add' | 'del' | 'ctx';

	interface DiffRow {
		type: DiffRowType;
		oldLine: number | null;
		newLine: number | null;
		content: string;
	}

	interface HunkHeader {
		oldStart: number;
		newStart: number;
	}

	function parseHunkHeader(line: string): HunkHeader {
		const m = line.match(/@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/);
		if (m) return { oldStart: parseInt(m[1], 10), newStart: parseInt(m[2], 10) };
		return { oldStart: 1, newStart: 1 };
	}

	const rows = $derived.by((): DiffRow[] => {
		const lines = diff.split('\n');
		const result: DiffRow[] = [];
		let oldLine = 1;
		let newLine = 1;

		for (const line of lines) {
			if (line.startsWith('---') || line.startsWith('+++')) continue;

			if (line.startsWith('@@')) {
				const { oldStart, newStart } = parseHunkHeader(line);
				oldLine = oldStart;
				newLine = newStart;
				result.push({ type: 'hunk', oldLine: null, newLine: null, content: line });
				continue;
			}

			if (line.startsWith('-')) {
				result.push({ type: 'del', oldLine: oldLine++, newLine: null, content: line.slice(1) });
			} else if (line.startsWith('+')) {
				result.push({ type: 'add', oldLine: null, newLine: newLine++, content: line.slice(1) });
			} else {
				const content = line.startsWith(' ') ? line.slice(1) : line;
				result.push({ type: 'ctx', oldLine: oldLine++, newLine: newLine++, content });
			}
		}

		return result;
	});

	const addCount = $derived(rows.filter((r) => r.type === 'add').length);
	const delCount = $derived(rows.filter((r) => r.type === 'del').length);

	let showAll = $state(false);
	let open = $state(false);

	$effect(() => {
		open = initiallyOpen;
	});

	const visibleRows = $derived(showAll ? rows : rows.slice(0, maxRows));
	const isTruncated = $derived(!showAll && rows.length > maxRows);

	// Language dot color from extension
	const ext = $derived(
		(() => {
			const dot = filePath.lastIndexOf('.');
			return dot >= 0 ? filePath.slice(dot) : '';
		})()
	);

	const langColor = $derived(
		(() => {
			switch (ext) {
				case '.rs': return '#f74c00';
				case '.ts': case '.tsx': return '#3178c6';
				case '.svelte': return '#ff3e00';
				case '.js': case '.jsx': return '#f7df1e';
				case '.toml': case '.yaml': case '.yml': return '#8fb47a';
				case '.md': return '#a3a29a';
				case '.json': return '#c29a6e';
				default: return '#545350';
			}
		})()
	);

	// Split path for display
	const lastSlash = $derived(filePath.lastIndexOf('/'));
	const fileDir = $derived(lastSlash >= 0 ? filePath.slice(0, lastSlash + 1) : '');
	const fileName = $derived(lastSlash >= 0 ? filePath.slice(lastSlash + 1) : filePath);

	function toggleOpen() {
		open = !open;
	}

	function expandAll() {
		showAll = true;
	}
</script>

<div class={styles.wrap}>
	{#if collapsible}
		<div class={styles.fileHeader} onclick={toggleOpen} role="button" tabindex="0"
			onkeydown={(e) => e.key === 'Enter' && toggleOpen()}>
			<span class={[styles.fileChevron, open && styles.fileChevronOpen].filter(Boolean).join(' ')}>
				<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
					stroke-linecap="round" stroke-linejoin="round" width="10" height="10" aria-hidden="true">
					<polyline points="9 18 15 12 9 6"></polyline>
				</svg>
			</span>
			<div class={styles.fileIconWrap}>
				<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"
					width="12" height="12" aria-hidden="true">
					<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
					<polyline points="14 2 14 8 20 8"></polyline>
				</svg>
				<span class={styles.langDot} style="background: {langColor}"></span>
			</div>
			<span class={styles.filePathText}>
				<span class={styles.fileDir}>{fileDir}</span><span class={styles.fileName}>{fileName}</span>
			</span>
			<div class={styles.fileStats}>
				{#if addCount > 0}<span class={styles.statAdd}>+{addCount}</span>{/if}
				{#if delCount > 0}<span class={styles.statDel}>−{delCount}</span>{/if}
			</div>
		</div>
	{/if}

	{#if !collapsible || open}
		<div class={styles.diffWrap}>
			<table class={styles.diffTable}>
				<colgroup>
					<col class={styles.colLn} />
					<col class={styles.colLn} />
					<col class={styles.colGt} />
					<col />
				</colgroup>
				<tbody>
					{#each visibleRows as row (row)}
						<tr class={styles[row.type]}>
							<td class={[styles.cell, styles.ln].filter(Boolean).join(' ')}>
								{row.oldLine ?? ''}
							</td>
							<td class={[styles.cell, styles.ln].filter(Boolean).join(' ')}>
								{row.newLine ?? ''}
							</td>
							<td class={[styles.cell, styles.gt].filter(Boolean).join(' ')}>
								{row.type === 'add' ? '+' : row.type === 'del' ? '−' : ''}
							</td>
							<td class={[styles.cell, styles.code].filter(Boolean).join(' ')}>
								{row.content}
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>

		{#if isTruncated}
			<div class={styles.showAllWrap}>
				<button class={styles.showAllBtn} onclick={expandAll}>
					<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
						stroke-linecap="round" stroke-linejoin="round" width="10" height="10" aria-hidden="true">
						<line x1="12" y1="5" x2="12" y2="19"></line>
						<polyline points="19 12 12 19 5 12"></polyline>
					</svg>
					Show all {rows.length} lines
				</button>
			</div>
		{/if}
	{/if}
</div>
