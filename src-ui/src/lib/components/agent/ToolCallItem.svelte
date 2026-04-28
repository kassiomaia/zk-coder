<script lang="ts">
	import {
		AlertCircle,
		CheckCircle2,
		ChevronRight,
		Clock,
		Code,
		FilePenLine,
		FileSearch,
		FileText,
		Globe,
		Terminal,
		Search,
		Eye,
		PlayCircle,
		ExternalLink,
		ListChecks
	} from 'lucide-svelte';
	import type { ToolCallRecord } from '$lib/types/agent';
	import { slide } from 'svelte/transition';
	import hljs from '$lib/utils/hljs';
	import 'highlight.js/styles/github-dark.css';

	interface Props {
		record: ToolCallRecord;
	}

	let { record }: Props = $props();
	let isOpen = $state(false);
	let showRaw = $state(false);

	const toolIconMap: Record<string, any> = {
		fs_read_file: FileText,
		fs_write_file: FilePenLine,
		fs_glob: FileSearch,
		fs_grep: Search,
		execute_command: Terminal,
		web_request: Globe,
		llm_request: Code,
		read: FileText,
		write: FilePenLine,
		glob: FileSearch,
		grep: Search,
		bash: Terminal,
		webfetch: Globe,
		skill: PlayCircle,
		task: Eye,
		agent_plan_subtasks: ListChecks
	};

	const ToolIcon = $derived(toolIconMap[record.tool] || Code);

	function formatJSON(obj: any): string {
		try {
			const json = JSON.stringify(obj, null, 2);
			return hljs.highlight(json, { language: 'json' }).value;
		} catch (e) {
			return String(obj);
		}
	}

	// Safe subtitle fallback: only primitive arg values are rendered inline.
	// Arrays/objects are skipped — Svelte's implicit string coercion would
	// otherwise produce `[object Object],...`.
	function firstPrimitiveValue(args: Record<string, unknown>): string | undefined {
		if (!args || typeof args !== 'object') return undefined;
		const first = Object.values(args)[0];
		if (first === null || first === undefined) return undefined;
		const t = typeof first;
		if (t === 'string') return first as string;
		if (t === 'number' || t === 'boolean') return String(first);
		return undefined;
	}

	function getActionSummary(record: ToolCallRecord): { title: string; subtitle?: string } {
		const args = record.arguments;
		const tool = record.tool;

		switch (tool) {
			case 'fs_read_file':
			case 'read':
				return { title: 'Read file', subtitle: args.filePath || args.path };
			case 'fs_write_file':
			case 'write':
				return { title: 'Modified file', subtitle: args.filePath || args.path };
			case 'fs_glob':
			case 'glob':
				return { title: 'Searched files', subtitle: args.pattern };
			case 'fs_grep':
			case 'grep':
				return { title: 'Searched content', subtitle: args.pattern };
			case 'execute_command':
			case 'bash':
				return { title: 'Ran command', subtitle: args.command };
			case 'web_request':
			case 'webfetch':
				return { title: 'Fetched URL', subtitle: args.url };
			case 'skill':
				return { title: 'Loaded skill', subtitle: args.name };
			case 'task':
				return { title: 'Spawned agent', subtitle: args.description };
			case 'agent_plan_subtasks': {
				const subtasks = (args.subtasks as Array<{ title?: string }> | undefined) ?? [];
				const count = subtasks.length;
				const first = subtasks[0]?.title;
				let subtitle: string | undefined;
				if (count === 0) subtitle = undefined;
				else if (count === 1) subtitle = first;
				else subtitle = first ? `${first} (+${count - 1} more)` : `${count} subtasks`;
				return { title: 'Planned subtasks', subtitle };
			}
			default:
				return { title: tool, subtitle: firstPrimitiveValue(args) };
		}
	}

	function getOutcomeSummary(record: ToolCallRecord): string {
		if (record.isError) return 'Action failed';
		if (!record.result) return 'Executing...';

		const tool = record.tool;
		const res = record.result;

		switch (tool) {
			case 'fs_read_file':
			case 'read':
				return `Read ${res.content?.split('\n').length || 0} lines`;
			case 'fs_write_file':
			case 'write':
				return 'Changes applied successfully';
			case 'fs_glob':
			case 'glob':
				const count = Array.isArray(res) ? res.length : res.paths?.length || 0;
				return `Found ${count} matching files`;
			case 'fs_grep':
			case 'grep':
				const matchCount = Array.isArray(res) ? res.length : Object.keys(res).length || 0;
				return `Found matches in ${matchCount} files`;
			case 'execute_command':
			case 'bash':
				return res.exit_code === 0
					? 'Command finished successfully'
					: `Failed with exit code ${res.exit_code}`;
			case 'task':
				return `Agent started (ID: ${res.task_id?.slice(0, 8)})`;
			default:
				return 'Completed';
		}
	}

	const summary = $derived(getActionSummary(record));
	const outcome = $derived(getOutcomeSummary(record));

	function formatTime(isoString: string): string {
		return new Date(isoString).toLocaleTimeString('en-US', {
			hour12: false,
			hour: '2-digit',
			minute: '2-digit'
		});
	}
</script>

<div class="tool-call-item" class:is-open={isOpen} class:has-error={record.isError}>
	<button class="item-header" onclick={() => (isOpen = !isOpen)} type="button">
		<div class="header-main">
			<span class="chevron" class:rotated={isOpen}>
				<ChevronRight size={14} />
			</span>

			<div class="tool-badge" title={record.tool}>
				<ToolIcon size={12} />
			</div>

			<div class="action-info">
				<span class="action-title">{summary.title}</span>
				{#if summary.subtitle}
					<span class="action-subtitle">{summary.subtitle}</span>
				{/if}
			</div>
		</div>

		<div class="header-meta">
			<span class="outcome-text" class:error={record.isError}>{outcome}</span>

			{#if record.durationMs}
				<span class="latency">
					{record.durationMs}ms
				</span>
			{/if}

			<span
				class="status-indicator"
				class:error={record.isError}
				class:success={!record.isError && record.result}
			>
				{#if record.isError}
					<AlertCircle size={14} />
				{:else if record.result}
					<CheckCircle2 size={14} />
				{:else}
					<Clock size={14} />
				{/if}
			</span>
		</div>
	</button>

	{#if isOpen}
		<div class="item-details" transition:slide={{ duration: 200 }}>
			<div class="details-grid">
				<div class="detail-group">
					<div class="group-label">Parameters</div>
					<div class="parameters-list">
						{#each Object.entries(record.arguments) as [key, value]}
							{#if key !== 'path' && key !== 'filePath' && key !== 'pattern' && key !== 'command' && key !== 'url' && key !== 'name'}
								<div class="param-item">
									<span class="param-key">{key}:</span>
									<span class="param-value"
										>{typeof value === 'object' ? JSON.stringify(value) : value}</span
									>
								</div>
							{/if}
						{/each}
						{#if summary.subtitle}
							<div class="param-item">
								<span class="param-key">Target:</span>
								<span class="param-value">{summary.subtitle}</span>
							</div>
						{/if}
					</div>
				</div>

				{#if record.result}
					<div class="detail-group">
						<div class="group-label">Result Overview</div>
						<div class="result-summary">
							{#if record.isError}
								<div class="error-message">
									{record.result.error || record.result.message || 'An unknown error occurred'}
								</div>
							{:else}
								<div class="success-data">
									{outcome}
								</div>
							{/if}
						</div>
					</div>
				{/if}
			</div>

			<div class="raw-toggle-area">
				<button class="raw-toggle" onclick={() => (showRaw = !showRaw)}>
					<Code size={12} />
					{showRaw ? 'Hide raw data' : 'View raw JSON'}
				</button>
			</div>

			{#if showRaw}
				<div class="raw-view" transition:slide>
					<div class="detail-section">
						<div class="section-label">Arguments</div>
						<pre class="json-block">{@html formatJSON(record.arguments)}</pre>
					</div>

					{#if record.result}
						<div class="detail-section">
							<div class="section-label">Result</div>
							<pre class="json-block">{@html formatJSON(record.result)}</pre>
						</div>
					{/if}
				</div>
			{/if}

			<div class="timestamp-footer">
				Executed at {formatTime(record.timestamp)}
			</div>
		</div>
	{/if}
</div>

<style>
	.tool-call-item {
		margin-bottom: var(--space-px);
		background: transparent;
		border-radius: var(--radius-none);
		border: 1px solid transparent;
		overflow: hidden;
		transition: all var(--transition-fast);
	}

	.tool-call-item:hover {
		background: var(--color-interactive-hover);
		border-color: var(--color-border-subtle);
	}

	.tool-call-item.is-open {
		background: var(--color-bg-secondary);
		border-color: var(--color-border-subtle);
		margin-bottom: var(--space-px);
	}

	.item-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		width: 100%;
		padding: var(--space-1) var(--space-2);
		background: transparent;
		border: none;
		cursor: pointer;
		gap: var(--space-2);
		text-align: left;
	}

	.header-main {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		min-width: 0;
		flex: 1;
	}

	.chevron {
		display: flex;
		color: var(--color-text-muted);
		opacity: 0.5;
		transition: transform var(--transition-fast);
	}

	.chevron.rotated {
		transform: rotate(90deg);
		opacity: 0.8;
	}

	.tool-badge {
		display: flex;
		align-items: center;
		justify-content: center;
		width: var(--space-4-5);
		height: var(--space-4-5);
		border-radius: var(--radius-none);
		background: var(--color-bg-tertiary);
		color: var(--color-text-tertiary);
		flex-shrink: 0;
		transition: all var(--transition-fast);
	}

	.is-open .tool-badge,
	.tool-call-item:hover .tool-badge {
		background: var(--color-primary-subtle);
		color: var(--color-primary-500);
	}

	.action-info {
		display: flex;
		flex-direction: row;
		align-items: baseline;
		gap: var(--space-2);
		min-width: 0;
	}

	.action-title {
		font-size: var(--text-xs);
		font-weight: var(--font-medium);
		color: var(--color-text-secondary);
		white-space: nowrap;
	}

	.is-open .action-title,
	.tool-call-item:hover .action-title {
		color: var(--color-text-primary);
		font-weight: var(--font-semibold);
	}

	.action-subtitle {
		font-family: var(--font-mono);
		font-size: var(--text-2xs);
		color: var(--color-text-muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		opacity: 0.5;
	}

	.header-meta {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		flex-shrink: 0;
	}

	.outcome-text {
		font-size: var(--text-2xs);
		color: var(--color-text-muted);
		font-weight: var(--font-normal);
		display: none;
	}

	.is-open .outcome-text {
		display: block;
	}

	.latency {
		font-size: var(--text-2xs);
		color: var(--color-text-muted);
		font-family: var(--font-mono);
		opacity: 0.4;
	}

	.status-indicator {
		display: flex;
		color: var(--color-text-muted);
		opacity: 0.4;
	}

	.status-indicator.success {
		color: var(--color-success);
		opacity: 0.7;
	}

	.status-indicator.error {
		color: var(--color-error);
		opacity: 0.8;
	}

	.item-details {
		padding: var(--space-3);
		background: var(--color-bg-secondary);
	}

	.details-grid {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		margin-bottom: var(--space-3);
	}

	.detail-group {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}

	.group-label {
		font-size: var(--text-2xs);
		font-weight: var(--font-bold);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-text-muted);
		opacity: 0.8;
	}

	.parameters-list {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.param-item {
		display: flex;
		gap: var(--space-2);
		font-size: var(--text-2xs);
	}

	.param-key {
		color: var(--color-text-tertiary);
		font-weight: var(--font-medium);
		min-width: 60px;
	}

	.param-value {
		color: var(--color-text-secondary);
		font-family: var(--font-mono);
		word-break: break-all;
	}

	.result-summary {
		font-size: var(--text-2xs);
		line-height: var(--leading-tight);
		color: var(--color-text-secondary);
	}

	.error-message {
		color: var(--color-error);
		background: var(--color-error-light);
		padding: var(--space-1-5) var(--space-2);
		border-radius: var(--radius-none);
		border: 1px solid var(--color-error-border);
	}

	.raw-toggle-area {
		margin-top: var(--space-3);
		display: flex;
		justify-content: flex-start;
	}

	.raw-toggle {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		background: transparent;
		border: none;
		font-size: var(--text-2xs);
		color: var(--color-primary-500);
		cursor: pointer;
		padding: 2px var(--space-1-5);
		border-radius: var(--radius-none);
		transition: background var(--transition-fast);
	}

	.raw-toggle:hover {
		background: var(--color-primary-subtle);
	}

	.raw-view {
		margin-top: var(--space-2);
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		padding: var(--space-2);
		background: var(--color-bg-secondary);
		border-radius: var(--radius-none);
	}

	.section-label {
		font-size: var(--text-2xs);
		font-weight: var(--font-bold);
		color: var(--color-text-muted);
		margin-bottom: 2px;
	}

	.json-block {
		margin: 0;
		padding: var(--space-2);
		background: var(--color-bg-tertiary);
		border-radius: var(--radius-none);
		font-family: var(--font-mono);
		font-size: var(--text-2xs);
		line-height: var(--leading-tight);
		overflow-x: auto;
		border: 1px solid var(--color-border-subtle);
	}

	.timestamp-footer {
		margin-top: var(--space-3);
		font-size: var(--text-2xs);
		color: var(--color-text-muted);
		opacity: 0.4;
		text-align: right;
	}

	.tool-call-item:hover {
		background: var(--color-interactive-hover);
		border-color: var(--color-border-subtle);
	}

	.tool-call-item.is-open {
		background: var(--color-bg-secondary);
		border-color: var(--color-border-subtle);
		margin-bottom: var(--space-2);
	}

	.item-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		width: 100%;
		padding: var(--space-1) var(--space-2);
		background: transparent;
		border: none;
		cursor: pointer;
		gap: var(--space-2);
		text-align: left;
	}

	.header-main {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		min-width: 0;
		flex: 1;
	}

	.chevron {
		display: flex;
		color: var(--color-text-muted);
		opacity: 0.5;
		transition: transform var(--transition-fast);
	}

	.chevron.rotated {
		transform: rotate(90deg);
		opacity: 0.8;
	}

	.tool-badge {
		display: flex;
		align-items: center;
		justify-content: center;
		width: var(--space-4-5);
		height: var(--space-4-5);
		border-radius: var(--radius-sm);
		background: var(--color-bg-tertiary);
		color: var(--color-text-tertiary);
		flex-shrink: 0;
		transition: all var(--transition-fast);
	}

	.is-open .tool-badge,
	.tool-call-item:hover .tool-badge {
		background: var(--color-primary-subtle);
		color: var(--color-primary-500);
	}

	.action-info {
		display: flex;
		flex-direction: row;
		align-items: baseline;
		gap: var(--space-2);
		min-width: 0;
	}

	.action-title {
		font-size: var(--text-xs);
		font-weight: var(--font-medium);
		color: var(--color-text-secondary);
		white-space: nowrap;
	}

	.is-open .action-title,
	.tool-call-item:hover .action-title {
		color: var(--color-text-primary);
		font-weight: var(--font-semibold);
	}

	.action-subtitle {
		font-family: var(--font-mono);
		font-size: var(--text-2xs);
		color: var(--color-text-muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		opacity: 0.5;
	}

	.header-meta {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		flex-shrink: 0;
	}

	.outcome-text {
		font-size: var(--text-2xs);
		color: var(--color-text-muted);
		font-weight: var(--font-normal);
		display: none;
	}

	.is-open .outcome-text {
		display: block;
	}

	.latency {
		font-size: var(--text-2xs);
		color: var(--color-text-muted);
		font-family: var(--font-mono);
		opacity: 0.4;
	}

	.status-indicator {
		display: flex;
		color: var(--color-text-muted);
		opacity: 0.4;
	}

	.status-indicator.success {
		color: var(--color-success);
		opacity: 0.7;
	}

	.status-indicator.error {
		color: var(--color-error);
		opacity: 0.8;
	}

	.item-details {
		padding: var(--space-3);
		background: var(--color-bg-secondary);
	}

	.details-grid {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		margin-bottom: var(--space-3);
	}

	.detail-group {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}

	.group-label {
		font-size: var(--text-2xs);
		font-weight: var(--font-bold);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-text-muted);
		opacity: 0.8;
	}

	.parameters-list {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.param-item {
		display: flex;
		gap: var(--space-2);
		font-size: var(--text-2xs);
	}

	.param-key {
		color: var(--color-text-tertiary);
		font-weight: var(--font-medium);
		min-width: 60px;
	}

	.param-value {
		color: var(--color-text-secondary);
		font-family: var(--font-mono);
		word-break: break-all;
	}

	.result-summary {
		font-size: var(--text-2xs);
		line-height: var(--leading-tight);
		color: var(--color-text-secondary);
	}

	.error-message {
		color: var(--color-error);
		background: var(--color-error-light);
		padding: var(--space-1-5) var(--space-2);
		border-radius: var(--radius-sm);
		border: 1px solid var(--color-error-border);
	}

	.raw-toggle-area {
		margin-top: var(--space-3);
		display: flex;
		justify-content: flex-start;
	}

	.raw-toggle {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		background: transparent;
		border: none;
		font-size: var(--text-2xs);
		color: var(--color-primary-500);
		cursor: pointer;
		padding: 2px var(--space-1-5);
		border-radius: var(--radius-sm);
		transition: background var(--transition-fast);
	}

	.raw-toggle:hover {
		background: var(--color-primary-subtle);
	}

	.raw-view {
		margin-top: var(--space-2);
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		padding: var(--space-2);
		background: var(--color-bg-secondary);
		border-radius: var(--radius-md);
	}

	.section-label {
		font-size: var(--text-2xs);
		font-weight: var(--font-bold);
		color: var(--color-text-muted);
		margin-bottom: 2px;
	}

	.json-block {
		margin: 0;
		padding: var(--space-2);
		background: var(--color-bg-tertiary);
		border-radius: var(--radius-sm);
		font-family: var(--font-mono);
		font-size: var(--text-2xs);
		line-height: var(--leading-tight);
		overflow-x: auto;
		border: 1px solid var(--color-border-subtle);
	}

	.timestamp-footer {
		margin-top: var(--space-3);
		font-size: var(--text-2xs);
		color: var(--color-text-muted);
		opacity: 0.4;
		text-align: right;
	}

	.has-error {
		border-left: 2px solid var(--color-error);
	}

	@media (max-width: 640px) {
		.details-grid {
			grid-template-columns: 1fr;
		}
	}
</style>
