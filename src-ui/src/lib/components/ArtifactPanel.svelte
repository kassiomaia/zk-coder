<script lang="ts">
	/**
	 * ArtifactPanel Component
	 * Sidebar panel for browsing and managing document artifacts.
	 * Shows all artifacts with filtering and selection capabilities.
	 */
	import styles from './ArtifactPanel.module.css';
	import ArtifactCard from './ArtifactCard.svelte';
	import type { ArtifactRef, ArtifactType } from './ArtifactCard.svelte';

	/** Local type alias for consistency */
	type Artifact = ArtifactRef;

	/** Filter options for artifacts */
	export type FilterType = 'all' | ArtifactType;

	interface Props {
		/** All artifacts to display */
		artifacts: Artifact[];
		/** Currently selected artifact ID */
		selectedArtifactId?: string | null;
		/** Current filter */
		filter?: FilterType;
		/** Handler when filter changes */
		onFilterChange?: (filter: FilterType) => void;
		/** Handler when artifact is selected */
		onSelect?: (artifact: Artifact) => void;
		/** Handler when artifact is downloaded */
		onDownload?: (artifact: Artifact) => void | Promise<void>;
		/** Handler when artifact is deleted */
		onDelete?: (artifact: Artifact) => void | Promise<void>;
		/** Whether the panel is empty (no artifacts) */
		empty?: boolean;
		/** Optional empty state message */
		emptyMessage?: string;
	}

	let {
		artifacts,
		selectedArtifactId = null,
		filter = 'all',
		onFilterChange,
		onSelect,
		onDownload,
		onDelete
	}: Props = $props();

	// Available filter types
	const filterTypes: FilterType[] = ['all', 'Docx', 'Pptx', 'Pdf'];

	// Current filter state - controlled by parent prop, uses 'all' as fallback
	let currentFilter = $derived.by(() => filter ?? 'all');

	// Filter artifacts by type (derived from currentFilter and artifacts)
	let filteredArtifacts = $derived.by(() => {
		if (currentFilter === 'all') {
			return artifacts;
		}
		return artifacts.filter((a) => a.artifactType === currentFilter);
	});

	// Group artifacts by type (derived from filteredArtifacts)
	let groupedArtifacts = $derived.by(() => {
		const groups: Record<ArtifactType, Artifact[]> = {
			Docx: [],
			Pptx: [],
			Pdf: []
		};

		for (const artifact of filteredArtifacts) {
			const type = artifact.artifactType;
			if (type in groups) {
				groups[type].push(artifact);
			}
		}

		return groups;
	});

	// Get artifact count for a type
	function getCount(type: ArtifactType | 'all'): number {
		if (type === 'all') {
			return artifacts.length;
		}
		return artifacts.filter((a) => a.artifactType === type).length;
	}

	// Handle filter button click
	function handleFilterClick(newFilter: FilterType) {
		onFilterChange?.(newFilter);
	}

	// Handle artifact selection
	function handleSelect(artifact: Artifact) {
		onSelect?.(artifact);
	}

	// Get filter label for display
	function getFilterLabel(filterType: FilterType): string {
		const labels: Record<FilterType, string> = {
			all: 'All Documents',
			Docx: 'Word',
			Pptx: 'PowerPoint',
			Pdf: 'PDF'
		};
		return labels[filterType];
	}

	// Get icon for filter button
	function getFilterIcon(filterType: FilterType): string {
		const icons: Record<FilterType, string> = {
			all: '📚',
			Docx: '📄',
			Pptx: '📊',
			Pdf: '📕'
		};
		return icons[filterType];
	}

	// Get group label for type
	function getGroupLabel(type: ArtifactType): string {
		const labels: Record<ArtifactType, string> = {
			Docx: 'Word Documents',
			Pptx: 'PowerPoint Presentations',
			Pdf: 'PDF Documents'
		};
		return `${type === 'Docx' ? '📄' : type === 'Pptx' ? '📊' : '📕'} ${labels[type]}`;
	}

	// Get icon for artifact type
	function getArtifactIcon(type: ArtifactType): string {
		const icons: Record<ArtifactType, string> = {
			Docx: '📄',
			Pptx: '📊',
			Pdf: '📕'
		};
		return icons[type];
	}

	// Format date for display
	function formatDate(dateString: string): string {
		const date = new Date(dateString);
		const now = new Date();
		const diffMs = now.getTime() - date.getTime();
		const diffMins = Math.floor(diffMs / 60000);
		const diffHours = Math.floor(diffMs / 3600000);
		const diffDays = Math.floor(diffMs / 86400000);

		if (diffMins < 1) return 'Just now';
		if (diffMins < 60) return `${diffMins}m ago`;
		if (diffHours < 24) return `${diffHours}h ago`;
		if (diffDays < 7) return `${diffDays}d ago`;
		return date.toLocaleDateString();
	}

	// Format file size
	function formatSize(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1048576) return `${Math.round(bytes / 1024)} KB`;
		return `${Math.round(bytes / 1048576)} MB`;
	}
</script>

<div class={styles.artifactPanel}>
	<!-- Header with title and filter buttons -->
	<div class={styles.header}>
		<h2 class={styles.title}>Documents</h2>
		<div class={styles.count}>{artifacts.length} documents</div>
	</div>

	<!-- Filter buttons -->
	<nav class={styles.filters} aria-label="Filter documents">
		{#each filterTypes as filterType}
			{@const count = getCount(filterType)}
			<button
				class={`${styles.filterButton} ${currentFilter === filterType ? styles.active : ''}`}
				onclick={() => handleFilterClick(filterType)}
				type="button"
				aria-pressed={currentFilter === filterType}
				aria-label={`${getFilterLabel(filterType)} (${count})`}
			>
				<span class={styles.filterIcon}>{getFilterIcon(filterType)}</span>
				<span class={styles.filterLabel}>{getFilterLabel(filterType)}</span>
				<span class={styles.filterCount}>({count})</span>
			</button>
		{/each}
	</nav>

	<!-- Artifact list -->
	<div class={styles.artifactList}>
		{#if filteredArtifacts.length === 0}
			<div class={styles.emptyState}>
				<span class={styles.emptyIcon}>📭</span>
				<p class={styles.emptyMessage}>
					No {currentFilter === 'all' ? '' : getFilterLabel(currentFilter).toLowerCase()} documents yet
				</p>
			</div>
		{:else}
			<!-- Show grouped by type when 'all' filter is selected -->
			{#if currentFilter === 'all'}
				{#each Object.entries(groupedArtifacts) as [type, typeArtifacts]}
					{#if typeArtifacts.length > 0}
						{@const artifactType = type as ArtifactType}
						<div class={styles.artifactGroup}>
							<h3 class={styles.groupTitle}>
								{getGroupLabel(artifactType)}
								<span class={styles.groupCount}>({typeArtifacts.length})</span>
							</h3>
							<div class={styles.artifactGrid}>
								{#each typeArtifacts as artifact}
									{@const isSelected = selectedArtifactId === artifact.id}
									<div
										class={`${styles.artifactItem} ${isSelected ? styles.selected : ''}`}
										onclick={() => handleSelect(artifact)}
										onkeydown={(e) => e.key === 'Enter' && handleSelect(artifact)}
										role="button"
										tabindex="0"
										aria-pressed={isSelected}
									>
										<div class={styles.artifactHeader}>
											<span class={styles.artifactIcon}
												>{getArtifactIcon(artifact.artifactType)}</span
											>
											<span class={styles.artifactFilename}>{artifact.filename}</span>
											<span class={styles.artifactSize}>{formatSize(artifact.sizeBytes)}</span>
										</div>
										<span class={styles.artifactDate}>{formatDate(artifact.createdAt)}</span>
									</div>
								{/each}
							</div>
						</div>
					{/if}
				{/each}
			{:else}
				<!-- Show simple list when specific filter is selected -->
				<div class={styles.artifactGrid}>
					{#each filteredArtifacts as artifact (artifact.id)}
						{@const isSelected = selectedArtifactId === artifact.id}
						<div
							class={`${styles.artifactItem} ${isSelected ? styles.selected : ''}`}
							onclick={() => handleSelect(artifact)}
							onkeydown={(e) => e.key === 'Enter' && handleSelect(artifact)}
							role="button"
							tabindex="0"
							aria-pressed={isSelected}
						>
							<div class={styles.artifactHeader}>
								<span class={styles.artifactIcon}>{getArtifactIcon(artifact.artifactType)}</span>
								<span class={styles.artifactFilename}>{artifact.filename}</span>
								<span class={styles.artifactSize}>{formatSize(artifact.sizeBytes)}</span>
							</div>
							<span class={styles.artifactDate}>{formatDate(artifact.createdAt)}</span>
						</div>
					{/each}
				</div>
			{/if}
		{/if}
	</div>
</div>
