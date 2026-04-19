<script lang="ts" module>
	/**
	 * Test-only module state. Exposes reactive `touched` and
	 * `refreshToken` that a plain `.ts` test file can mutate between
	 * assertions without @testing-library/svelte's `rerender`, which
	 * resets Svelte 5 runes inside the component under test.
	 */
	import type { TouchedFile } from '$lib/types/agent';

	export const harnessState: { touched: TouchedFile[]; refreshToken: number } = $state({
		touched: [],
		refreshToken: 0
	});

	export function setHarnessTouched(t: TouchedFile[]): void {
		harnessState.touched = t;
	}

	export function bumpHarnessRefresh(): void {
		harnessState.refreshToken += 1;
	}

	export function resetHarness(): void {
		harnessState.touched = [];
		harnessState.refreshToken = 0;
	}
</script>

<script lang="ts">
	import FileTree from './FileTree.svelte';
	import type { WorkspaceEntry } from '$lib/types/agent';

	interface Props {
		workspaceRoot: string;
		rootEntries: WorkspaceEntry[];
		rootTruncated?: boolean;
		onfileclick?: (path: string) => void;
		activePath?: string;
	}

	let {
		workspaceRoot,
		rootEntries,
		rootTruncated = false,
		onfileclick,
		activePath
	}: Props = $props();
</script>

<FileTree
	{workspaceRoot}
	{rootEntries}
	{rootTruncated}
	touched={harnessState.touched}
	refreshToken={harnessState.refreshToken}
	{onfileclick}
	{activePath}
/>
