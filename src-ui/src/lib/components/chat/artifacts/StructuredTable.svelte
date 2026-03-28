<script lang="ts">
	/**
	 * StructuredTable Component
	 *
	 * Productized table component with strong visual hierarchy.
	 * Designed for displaying structured data from agent outputs.
	 *
	 * Features:
	 * - Clear header row with styling distinction
	 * - Hover states for rows
	 * - Responsive design with horizontal scroll
	 * - Optional sorting indicators
	 * - Column alignment control
	 */
	import type { Snippet } from 'svelte';
	import styles from './StructuredTable.module.css';
	import { ArrowUp, ArrowDown, Minus } from 'lucide-svelte';

	export type ColumnAlign = 'left' | 'center' | 'right';
	export type SortDirection = 'asc' | 'desc' | 'none';

	export interface TableColumn {
		key: string;
		label: string;
		align?: ColumnAlign;
		sortable?: boolean;
		width?: string;
	}

	export interface TableRow {
		id: string;
		cells: Record<string, Snippet | string>;
		highlighted?: boolean;
	}

	interface Props {
		/** Table columns definition */
		columns: TableColumn[];
		/** Table rows data */
		rows: TableRow[];
		/** Currently sorted column */
		sortColumn?: string;
		/** Sort direction */
		sortDirection?: SortDirection;
		/** Sort handler */
		onSort?: (column: string, direction: SortDirection) => void;
		/** Empty state message */
		emptyMessage?: string;
		/** Maximum height for scrollable table */
		maxHeight?: string;
		/** Compact variant */
		compact?: boolean;
		/** Enable row highlighting on hover */
		highlightRows?: boolean;
	}

	let {
		columns,
		rows,
		sortColumn,
		sortDirection = 'none',
		onSort,
		emptyMessage = 'No data available',
		maxHeight,
		compact = false,
		highlightRows = true
	}: Props = $props();

	// Handle column header click for sorting
	function handleHeaderClick(column: TableColumn) {
		if (!column.sortable || !onSort) return;

		if (sortColumn === column.key) {
			// Cycle through: none -> asc -> desc -> none
			if (sortDirection === 'none') {
				onSort(column.key, 'asc');
			} else if (sortDirection === 'asc') {
				onSort(column.key, 'desc');
			} else {
				onSort(column.key, 'none');
			}
		} else {
			onSort(column.key, 'asc');
		}
	}

	// Get sort icon for column header
	function getSortIcon(column: TableColumn): typeof Minus | typeof ArrowUp | typeof ArrowDown | null {
		if (sortColumn !== column.key || sortDirection === 'none') return Minus;
		return sortDirection === 'asc' ? ArrowUp : ArrowDown;
	}

	// Get alignment class for column
	function getAlignClass(align?: ColumnAlign): string {
		switch (align) {
			case 'center':
				return styles.alignCenter;
			case 'right':
				return styles.alignRight;
			default:
				return styles.alignLeft;
		}
	}

	// Check if table has data
	let hasData = $derived(rows.length > 0);

	// Calculate column width style
	function getColumnStyle(column: TableColumn): string {
		return column.width ? `width: ${column.width}` : '';
	}
</script>

<div
	class={[styles.tableWrapper, compact && styles.compact].filter(Boolean).join(' ')}
	style:max-height={maxHeight}
>
	<table class={styles.table}>
		<thead class={styles.thead}>
			<tr class={styles.headerRow}>
				{#each columns as column}
					<th
						class={[
							styles.headerCell,
							getAlignClass(column.align),
							column.sortable && styles.sortable
						].filter(Boolean).join(' ')}
						style={getColumnStyle(column)}
					>
						{#if column.sortable}
							<button
								class={styles.headerButton}
								onclick={() => handleHeaderClick(column)}
								type="button"
							>
								<span class={styles.headerLabel}>{column.label}</span>
								<span class={styles.sortIcon}>
									{@const SortIcon = getSortIcon(column)}
									{#if SortIcon}
										<SortIcon size={12} strokeWidth={2} />
									{/if}
								</span>
							</button>
						{:else}
							<span class={styles.headerLabel}>{column.label}</span>
						{/if}
					</th>
				{/each}
			</tr>
		</thead>

		<tbody class={styles.tbody}>
			{#if hasData}
				{#each rows as row, index}
					<tr
						class={[
							styles.row,
							highlightRows && styles.highlightable,
							row.highlighted && styles.rowHighlighted
						].filter(Boolean).join(' ')}
						style:animation-delay={`${index * 30}ms`}
					>
						{#each columns as column}
							<td
								class={[styles.cell, getAlignClass(column.align)].join(' ')}
							>
								{#if typeof row.cells[column.key] === 'string'}
									<span class={styles.cellContent}>{row.cells[column.key]}</span>
								{:else if typeof row.cells[column.key] === 'function'}
									<span class={styles.cellContent}>{@render row.cells[column.key]()}</span>
								{/if}
							</td>
						{/each}
					</tr>
				{/each}
			{:else}
				<tr class={styles.emptyRow}>
					<td colspan={columns.length} class={styles.emptyCell}>
						<div class={styles.emptyState}>
							<p class={styles.emptyMessage}>{emptyMessage}</p>
						</div>
					</td>
				</tr>
			{/if}
		</tbody>
	</table>
</div>
