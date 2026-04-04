<script lang="ts">
	import { Activity, CheckCircle2, Clock, Cpu, Database, Zap } from 'lucide-svelte';
	import type { TableRow } from './traceTable';
	import shell from './AgentTraceShell.module.css';
	import table from './AgentTraceTable.module.css';

	type ColumnWidths = Record<string, number>;

	let {
		rows,
		columnWidths,
		selectedTraceId,
		onSelect,
		onResizeStart
	}: {
		rows: TableRow[];
		columnWidths: ColumnWidths;
		selectedTraceId: string | null;
		onSelect: (id: string) => void;
		onResizeStart: (event: MouseEvent, column: string) => void;
	} = $props();
</script>

<table class={table.table}>
	<thead>
		<tr>
			<th style:width="{columnWidths.time}px">
				<div class={shell.headerCell}>
					<Clock size={12} />
					Time
				</div>
				<button
					class={table.colResizer}
					type="button"
					aria-label="Resize time column"
					onmousedown={(event) => onResizeStart(event, 'time')}
				></button>
			</th>
			<th style:width="{columnWidths.delta}px">
				<div class={shell.headerCell}>
					<Zap size={12} />
					Delta
				</div>
				<button
					class={table.colResizer}
					type="button"
					aria-label="Resize delta column"
					onmousedown={(event) => onResizeStart(event, 'delta')}
				></button>
			</th>
			<th style:width="{columnWidths.step}px">
				<div class={shell.headerCell}>Step</div>
				<button
					class={table.colResizer}
					type="button"
					aria-label="Resize step column"
					onmousedown={(event) => onResizeStart(event, 'step')}
				></button>
			</th>
			<th style:width="{columnWidths.phase}px">
				<div class={shell.headerCell}>
					<Activity size={12} />
					Phase
				</div>
				<button
					class={table.colResizer}
					type="button"
					aria-label="Resize phase column"
					onmousedown={(event) => onResizeStart(event, 'phase')}
				></button>
			</th>
			<th style:width="{columnWidths.action}px">
				<div class={shell.headerCell}>Action</div>
				<button
					class={table.colResizer}
					type="button"
					aria-label="Resize action column"
					onmousedown={(event) => onResizeStart(event, 'action')}
				></button>
			</th>
			<th style:width="{columnWidths.actor}px">
				<div class={shell.headerCell}>
					<Cpu size={12} />
					Actor
				</div>
				<button
					class={table.colResizer}
					type="button"
					aria-label="Resize actor column"
					onmousedown={(event) => onResizeStart(event, 'actor')}
				></button>
			</th>
			<th style:width="{columnWidths.status}px">
				<div class={shell.headerCell}>
					<CheckCircle2 size={12} />
					Status
				</div>
				<button
					class={table.colResizer}
					type="button"
					aria-label="Resize status column"
					onmousedown={(event) => onResizeStart(event, 'status')}
				></button>
			</th>
			<th style:width="{columnWidths.duration}px">
				<div class={shell.headerCell}>
					<Clock size={12} />
					Duration
				</div>
				<button
					class={table.colResizer}
					type="button"
					aria-label="Resize duration column"
					onmousedown={(event) => onResizeStart(event, 'duration')}
				></button>
			</th>
			<th style:width="{columnWidths.tokens}px">
				<div class={shell.headerCell}>
					<Database size={12} />
					Tokens
				</div>
				<button
					class={table.colResizer}
					type="button"
					aria-label="Resize tokens column"
					onmousedown={(event) => onResizeStart(event, 'tokens')}
				></button>
			</th>
			<th style:width="{columnWidths.tool}px">
				<div class={shell.headerCell}>Tool</div>
				<button
					class={table.colResizer}
					type="button"
					aria-label="Resize tool column"
					onmousedown={(event) => onResizeStart(event, 'tool')}
				></button>
			</th>
			<th style:width="{columnWidths.summary}px">
				<div class={shell.headerCell}>Summary</div>
				<button
					class={table.colResizer}
					type="button"
					aria-label="Resize summary column"
					onmousedown={(event) => onResizeStart(event, 'summary')}
				></button>
			</th>
		</tr>
	</thead>
	<tbody>
		{#each rows as row (row.id)}
			<tr
				class={`${table.row} ${row.id === selectedTraceId ? table.rowSelected : ''}`}
				onclick={() => onSelect(row.id)}
			>
				<td class={table.rowCode}>{row.time}</td>
				<td class={table.deltaCell}>
					{row.deltaMs ? `+${row.deltaMs}ms` : '-'}
				</td>
				<td>{row.step}</td>
				<td>
					<span class={`${table.pill} ${table.pillNeutral}`}>{row.phase}</span>
				</td>
				<td class={table.rowAction} title={row.action}>{row.action}</td>
				<td>
					<span
						class={`${table.pill} ${row.actor === 'LLM' ? table.pillPrimary : row.actor === 'Tools' ? table.pillSuccess : table.pillNeutral}`}
					>
						{row.actor}
					</span>
				</td>
				<td>
					<span
						class={`${table.pill} ${
							row.status === 'completed'
								? table.pillSuccess
								: row.status === 'failed'
									? table.pillError
									: row.status === 'cancelled'
										? table.pillWarning
										: table.pillPrimary
						}`}
					>
						{row.status}
					</span>
				</td>
				<td>{row.duration}</td>
				<td class={table.rowCode}>{row.tokens}</td>
				<td title={row.tool}>{row.tool}</td>
				<td class={table.summaryCell} title={row.summary}>{row.summary}</td>
			</tr>
		{/each}
	</tbody>
</table>
