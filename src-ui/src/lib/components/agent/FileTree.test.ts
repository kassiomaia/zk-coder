import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import FileTree from './FileTree.svelte';
import FileTreeTestHarness, {
	resetHarness,
	setHarnessTouched
} from './FileTree.testHarness.svelte';
import type { TouchedFile, WorkspaceEntry } from '$lib/types/agent';

const hoistedListDir = vi.hoisted(() => vi.fn());

vi.mock('$lib/api/workspace', () => ({
	listWorkspaceDir: hoistedListDir
}));

function entry(path: string, kind: 'file' | 'directory' = 'file'): WorkspaceEntry {
	return { path, kind };
}

function filesOnly(paths: string[]): WorkspaceEntry[] {
	return paths.map((p) => entry(p));
}

function touched(overrides: Partial<TouchedFile>): TouchedFile {
	return {
		path: 'file.ts',
		modified: true,
		action: 'write',
		...overrides
	};
}

function baseProps(overrides: Partial<Parameters<typeof FileTree>[1]> = {}) {
	return {
		workspaceRoot: '/tmp/ws',
		rootEntries: [] as WorkspaceEntry[],
		rootTruncated: false,
		touched: [] as TouchedFile[],
		...overrides
	};
}

describe('FileTree — lazy-load behavior', () => {
	beforeEach(() => {
		hoistedListDir.mockReset();
	});

	afterEach(() => {
		vi.useRealTimers();
	});

	it('renders empty state when the root has no entries', () => {
		render(FileTree, baseProps());

		expect(screen.getByText('No files to display')).toBeInTheDocument();
	});

	it('renders flat file entries from the root listing', () => {
		render(
			FileTree,
			baseProps({
				rootEntries: filesOnly(['README.md', 'a.ts'])
			})
		);

		expect(screen.getByText('README.md')).toBeInTheDocument();
		expect(screen.getByText('a.ts')).toBeInTheDocument();
	});

	it('renders the directory basename, not its full path', () => {
		render(
			FileTree,
			baseProps({
				rootEntries: [entry('src', 'directory')]
			})
		);

		expect(screen.getByText('src')).toBeInTheDocument();
	});

	it('does not render descendants of a collapsed directory', () => {
		render(
			FileTree,
			baseProps({
				rootEntries: [entry('src', 'directory')]
			})
		);

		expect(screen.queryByText('main.ts')).not.toBeInTheDocument();
	});

	it('fetches and reveals children when a directory is expanded', async () => {
		hoistedListDir.mockResolvedValue({
			entries: [entry('src/main.ts')],
			truncated: false
		});

		render(
			FileTree,
			baseProps({
				rootEntries: [entry('src', 'directory')]
			})
		);

		const srcBtn = screen.getByText('src').closest('button') as HTMLElement;
		expect(srcBtn).toHaveAttribute('aria-expanded', 'false');

		await fireEvent.click(srcBtn);

		expect(hoistedListDir).toHaveBeenCalledWith('/tmp/ws', 'src');
		expect(srcBtn).toHaveAttribute('aria-expanded', 'true');
		await waitFor(() => expect(screen.getByText('main.ts')).toBeInTheDocument());
	});

	it('does not re-fetch on a second expand of the same directory (cache hit)', async () => {
		hoistedListDir.mockResolvedValue({
			entries: [entry('src/main.ts')],
			truncated: false
		});

		render(
			FileTree,
			baseProps({
				rootEntries: [entry('src', 'directory')]
			})
		);

		const srcBtn = screen.getByText('src').closest('button') as HTMLElement;
		await fireEvent.click(srcBtn); // expand
		await waitFor(() => expect(screen.getByText('main.ts')).toBeInTheDocument());
		await fireEvent.click(srcBtn); // collapse
		expect(screen.queryByText('main.ts')).not.toBeInTheDocument();
		await fireEvent.click(srcBtn); // re-expand

		expect(hoistedListDir).toHaveBeenCalledTimes(1);
		await waitFor(() => expect(screen.getByText('main.ts')).toBeInTheDocument());
	});

	it('collapses a directory on a second click', async () => {
		hoistedListDir.mockResolvedValue({
			entries: [entry('src/main.ts')],
			truncated: false
		});

		render(
			FileTree,
			baseProps({
				rootEntries: [entry('src', 'directory')]
			})
		);

		const srcBtn = screen.getByText('src').closest('button') as HTMLElement;
		await fireEvent.click(srcBtn);
		await waitFor(() => expect(screen.getByText('main.ts')).toBeInTheDocument());
		await fireEvent.click(srcBtn);
		expect(screen.queryByText('main.ts')).not.toBeInTheDocument();
	});

	it('shows an inline error with a retry when the listing call fails', async () => {
		hoistedListDir.mockRejectedValueOnce(new Error('boom'));

		render(
			FileTree,
			baseProps({
				rootEntries: [entry('src', 'directory')]
			})
		);

		await fireEvent.click(screen.getByText('src').closest('button') as HTMLElement);

		const retry = await screen.findByRole('button', { name: /failed to load — retry/i });
		expect(retry).toBeInTheDocument();

		hoistedListDir.mockResolvedValueOnce({
			entries: [entry('src/main.ts')],
			truncated: false
		});
		await fireEvent.click(retry);

		await waitFor(() => expect(screen.getByText('main.ts')).toBeInTheDocument());
	});
});

describe('FileTree — touched-file overlay', () => {
	beforeEach(() => {
		hoistedListDir.mockReset();
	});

	it('shows "M" badge for a write+modified file at the root', () => {
		render(
			FileTree,
			baseProps({
				rootEntries: filesOnly(['a.ts']),
				touched: [touched({ path: 'a.ts', action: 'write', modified: true })]
			})
		);

		expect(screen.getByText('M')).toBeInTheDocument();
	});

	it('shows "A" badge for write+!modified', () => {
		render(
			FileTree,
			baseProps({
				rootEntries: filesOnly(['b.ts']),
				touched: [touched({ path: 'b.ts', action: 'write', modified: false })]
			})
		);

		expect(screen.getByText('A')).toBeInTheDocument();
	});

	it('shows "D" badge for delete action', () => {
		render(
			FileTree,
			baseProps({
				rootEntries: filesOnly(['c.ts']),
				touched: [touched({ path: 'c.ts', action: 'delete', modified: true })]
			})
		);

		expect(screen.getByText('D')).toBeInTheDocument();
	});

	it('shows no badge for read action', () => {
		render(
			FileTree,
			baseProps({
				rootEntries: filesOnly(['d.ts']),
				touched: [touched({ path: 'd.ts', action: 'read', modified: false })]
			})
		);

		expect(screen.queryByText('M')).not.toBeInTheDocument();
		expect(screen.queryByText('A')).not.toBeInTheDocument();
		expect(screen.queryByText('D')).not.toBeInTheDocument();
	});

	it('renders no badge when the touched path does not match any rendered entry', () => {
		render(
			FileTree,
			baseProps({
				rootEntries: filesOnly(['untouched.ts']),
				touched: []
			})
		);

		expect(screen.queryByText('M')).not.toBeInTheDocument();
	});
});

describe('FileTree — file click callback', () => {
	beforeEach(() => {
		hoistedListDir.mockReset();
	});

	it('invokes onfileclick with the full relative path', async () => {
		hoistedListDir.mockResolvedValue({
			entries: [entry('src/main.ts')],
			truncated: false
		});
		const onfileclick = vi.fn();

		render(
			FileTree,
			baseProps({
				rootEntries: [entry('src', 'directory')],
				onfileclick
			})
		);

		await fireEvent.click(screen.getByText('src').closest('button') as HTMLElement);
		await waitFor(() => expect(screen.getByText('main.ts')).toBeInTheDocument());
		await fireEvent.click(screen.getByText('main.ts').closest('button') as HTMLElement);

		expect(onfileclick).toHaveBeenCalledWith('src/main.ts');
	});

	it('applies the active style only to the file matching activePath', () => {
		render(
			FileTree,
			baseProps({
				rootEntries: filesOnly(['solo.ts']),
				activePath: 'solo.ts'
			})
		);

		const file = screen.getByText('solo.ts').closest('button') as HTMLElement;
		expect(file.className).toMatch(/isActive/);
	});
});

describe('FileTree — truncation', () => {
	it('renders per-folder truncation message at the root when rootTruncated=true', () => {
		render(
			FileTree,
			baseProps({
				rootEntries: filesOnly(['a.ts']),
				rootTruncated: true
			})
		);

		expect(screen.getByText(/Showing first 1000 of this folder/i)).toBeInTheDocument();
	});

	it('does not render a truncation message when the root is complete', () => {
		render(
			FileTree,
			baseProps({
				rootEntries: filesOnly(['a.ts']),
				rootTruncated: false
			})
		);

		expect(screen.queryByText(/Showing first/i)).not.toBeInTheDocument();
	});

	it('renders a truncation message inside an expanded sub-folder when the sub-listing was capped', async () => {
		// The truncation message is driven by the response flag, not the
		// actual entry count — a small fixture keeps this test fast.
		hoistedListDir.mockResolvedValue({
			entries: [entry('big/a.ts'), entry('big/b.ts')],
			truncated: true
		});

		render(
			FileTree,
			baseProps({
				rootEntries: [entry('big', 'directory')]
			})
		);

		await fireEvent.click(screen.getByText('big').closest('button') as HTMLElement);
		await waitFor(() =>
			expect(screen.getByText(/Showing first 1000 of this folder/i)).toBeInTheDocument()
		);
	});
});

describe('FileTree — accessibility', () => {
	it('has an accessible label on the outer container', () => {
		const { container } = render(FileTree, baseProps());
		const tree = container.querySelector('[aria-label="File tree"]');
		expect(tree).toBeInTheDocument();
	});

	it('does not expose a refresh control — the header owns it', () => {
		render(
			FileTree,
			baseProps({
				rootEntries: filesOnly(['a.ts'])
			})
		);
		expect(
			screen.queryByRole('button', { name: /refresh file tree/i })
		).not.toBeInTheDocument();
	});
});

describe('FileTree — touched-file auto-expand', () => {
	beforeEach(() => {
		hoistedListDir.mockReset();
		resetHarness();
		vi.useFakeTimers({ shouldAdvanceTime: true });
	});

	afterEach(() => {
		vi.useRealTimers();
	});

	it('expands ancestors of a touched file after the debounce window', async () => {
		hoistedListDir.mockResolvedValue({
			entries: [entry('src/main.ts')],
			truncated: false
		});

		render(FileTreeTestHarness, {
			workspaceRoot: '/tmp/ws',
			rootEntries: [entry('src', 'directory')],
			rootTruncated: false
		});

		expect(screen.getByText('src').closest('button') as HTMLElement).toHaveAttribute(
			'aria-expanded',
			'false'
		);

		setHarnessTouched([
			touched({ path: 'src/main.ts', action: 'write', modified: false })
		]);

		await vi.advanceTimersByTimeAsync(250);

		expect(hoistedListDir).toHaveBeenCalledWith('/tmp/ws', 'src');
		await waitFor(() =>
			expect(screen.getByText('src').closest('button') as HTMLElement).toHaveAttribute(
				'aria-expanded',
				'true'
			)
		);
	});

	it('does not auto-expand a directory the user has collapsed', async () => {
		hoistedListDir.mockResolvedValue({
			entries: [entry('src/main.ts')],
			truncated: false
		});

		render(FileTreeTestHarness, {
			workspaceRoot: '/tmp/ws',
			rootEntries: [entry('src', 'directory')],
			rootTruncated: false
		});

		let srcBtn = screen.getByText('src').closest('button') as HTMLElement;
		await fireEvent.click(srcBtn); // user expands
		await waitFor(() => expect(screen.getByText('main.ts')).toBeInTheDocument());
		await fireEvent.click(srcBtn); // user collapses — registers in userCollapsed
		expect(srcBtn).toHaveAttribute('aria-expanded', 'false');

		setHarnessTouched([
			touched({ path: 'src/main.ts', action: 'write', modified: true })
		]);

		await vi.advanceTimersByTimeAsync(250);

		srcBtn = screen.getByText('src').closest('button') as HTMLElement;
		expect(srcBtn).toHaveAttribute('aria-expanded', 'false');
		expect(screen.queryByText('main.ts')).not.toBeInTheDocument();
	});
});
