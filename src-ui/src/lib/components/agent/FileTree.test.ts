import { fireEvent, render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import FileTree from './FileTree.svelte';
import type { TouchedFile, WorkspaceEntry } from '$lib/types/agent';

function touched(overrides: Partial<TouchedFile>): TouchedFile {
	return {
		path: 'file.ts',
		modified: true,
		action: 'write',
		...overrides
	};
}

function entry(path: string, kind: 'file' | 'directory' = 'file'): WorkspaceEntry {
	return { path, kind };
}

function filesOnly(paths: string[]): WorkspaceEntry[] {
	return paths.map((p) => entry(p));
}

describe('FileTree — baseline + extended behavior', () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	it('renders empty state when workspace is empty and nothing touched', () => {
		render(FileTree, { allFiles: [], touched: [] });

		expect(screen.getByText('No files to display')).toBeInTheDocument();
	});

	it('renders a flat file entry with its basename', () => {
		render(FileTree, {
			allFiles: filesOnly(['README.md']),
			touched: []
		});

		expect(screen.getByText('README.md')).toBeInTheDocument();
	});

	it('renders workspace files WITHOUT a badge when not touched', () => {
		render(FileTree, {
			allFiles: filesOnly(['untouched.ts']),
			touched: []
		});

		expect(screen.getByText('untouched.ts')).toBeInTheDocument();
		expect(screen.queryByText('M')).not.toBeInTheDocument();
		expect(screen.queryByText('A')).not.toBeInTheDocument();
		expect(screen.queryByText('D')).not.toBeInTheDocument();
	});

	it('shows "M" badge when a workspace file is also in touched with write+modified', () => {
		render(FileTree, {
			allFiles: filesOnly(['a.ts']),
			touched: [touched({ path: 'a.ts', action: 'write', modified: true })]
		});

		expect(screen.getByText('M')).toBeInTheDocument();
	});

	it('shows "A" badge for write+!modified touched entries', () => {
		render(FileTree, {
			allFiles: filesOnly(['b.ts']),
			touched: [touched({ path: 'b.ts', action: 'write', modified: false })]
		});

		expect(screen.getByText('A')).toBeInTheDocument();
	});

	it('shows "D" badge for delete action', () => {
		render(FileTree, {
			allFiles: filesOnly(['c.ts']),
			touched: [touched({ path: 'c.ts', action: 'delete', modified: true })]
		});

		expect(screen.getByText('D')).toBeInTheDocument();
	});

	it('shows no badge for read action', () => {
		render(FileTree, {
			allFiles: filesOnly(['d.ts']),
			touched: [touched({ path: 'd.ts', action: 'read', modified: false })]
		});

		expect(screen.getByText('d.ts')).toBeInTheDocument();
		expect(screen.queryByText('M')).not.toBeInTheDocument();
	});

	it('builds nested directory structure from slash-separated paths', () => {
		render(FileTree, {
			allFiles: [
				entry('src', 'directory'),
				entry('src/lib', 'directory'),
				entry('src/lib/foo.ts'),
				entry('src/lib/bar.ts'),
				entry('src/main.ts')
			],
			touched: []
		});

		expect(screen.getByText('src')).toBeInTheDocument();
		expect(screen.queryByText('foo.ts')).not.toBeInTheDocument();
		expect(screen.queryByText('lib')).not.toBeInTheDocument();
	});

	it('expands a directory on click to reveal children', async () => {
		render(FileTree, {
			allFiles: [entry('src', 'directory'), entry('src/main.ts')],
			touched: []
		});

		const srcDir = screen.getByText('src').closest('button') as HTMLElement;
		expect(srcDir).toHaveAttribute('aria-expanded', 'false');

		await fireEvent.click(srcDir);

		expect(srcDir).toHaveAttribute('aria-expanded', 'true');
		expect(screen.getByText('main.ts')).toBeInTheDocument();
	});

	it('collapses a directory on second click', async () => {
		render(FileTree, {
			allFiles: [entry('src', 'directory'), entry('src/main.ts')],
			touched: []
		});

		const srcDir = screen.getByText('src').closest('button') as HTMLElement;
		await fireEvent.click(srcDir);
		expect(screen.getByText('main.ts')).toBeInTheDocument();

		await fireEvent.click(srcDir);
		expect(screen.queryByText('main.ts')).not.toBeInTheDocument();
	});

	it('invokes onfileclick with the full path when a file is clicked', async () => {
		const onfileclick = vi.fn();
		render(FileTree, {
			allFiles: [entry('src', 'directory'), entry('src/main.ts')],
			touched: [],
			onfileclick
		});

		await fireEvent.click(screen.getByText('src').closest('button') as HTMLElement);
		await fireEvent.click(screen.getByText('main.ts').closest('button') as HTMLElement);

		expect(onfileclick).toHaveBeenCalledWith('src/main.ts');
	});

	it('applies the active style only to the file matching activePath', () => {
		render(FileTree, {
			allFiles: filesOnly(['solo.ts']),
			touched: [],
			activePath: 'solo.ts'
		});

		const file = screen.getByText('solo.ts').closest('button') as HTMLElement;
		expect(file.className).toMatch(/isActive/);
	});

	it('has an accessible label on the outer container', () => {
		const { container } = render(FileTree, { allFiles: [], touched: [] });

		const tree = container.querySelector('[aria-label="File tree"]');
		expect(tree).toBeInTheDocument();
	});

	it('synthetic-merges a touched path that is missing from allFiles', () => {
		render(FileTree, {
			allFiles: [entry('existing.ts')],
			touched: [touched({ path: 'brand/new.ts', action: 'write', modified: false })]
		});

		// Directory auto-created
		expect(screen.getByText('brand')).toBeInTheDocument();
	});

	it('keeps the synthetic touched file under its expanded directory with badge', async () => {
		render(FileTree, {
			allFiles: [],
			touched: [touched({ path: 'brand/new.ts', action: 'write', modified: false })]
		});

		await fireEvent.click(screen.getByText('brand').closest('button') as HTMLElement);
		expect(screen.getByText('new.ts')).toBeInTheDocument();
		expect(screen.getByText('A')).toBeInTheDocument();
	});

	it('renders the truncation footer when truncated=true', () => {
		render(FileTree, {
			allFiles: filesOnly(['a.ts']),
			touched: [],
			truncated: true
		});

		expect(screen.getByText(/Showing first 10,000 entries/i)).toBeInTheDocument();
	});

	it('does not render truncation footer when truncated=false', () => {
		render(FileTree, {
			allFiles: filesOnly(['a.ts']),
			touched: [],
			truncated: false
		});

		expect(screen.queryByText(/Showing first/i)).not.toBeInTheDocument();
	});

	it('does not expose any refresh control — the header owns it', () => {
		render(FileTree, {
			allFiles: filesOnly(['a.ts']),
			touched: []
		});

		expect(screen.queryByRole('button', { name: /refresh file tree/i })).not.toBeInTheDocument();
	});

	it('renders directories before files at the same level', () => {
		const { container } = render(FileTree, {
			allFiles: [
				entry('z.ts'),
				entry('alpha', 'directory'),
				entry('b.ts'),
				entry('zeta', 'directory')
			],
			touched: []
		});

		const names = Array.from(
			container.querySelectorAll('[aria-label="File tree"] button [class*="nodeName"]')
		).map((n) => n.textContent?.trim());
		// directories first (alphabetical), then files (alphabetical)
		expect(names).toEqual(['alpha', 'zeta', 'b.ts', 'z.ts']);
	});

	it('sorts files alphabetically (case-insensitive, natural order)', () => {
		const { container } = render(FileTree, {
			allFiles: [entry('File10.ts'), entry('file2.ts'), entry('file1.ts'), entry('apple.ts')],
			touched: []
		});

		const names = Array.from(
			container.querySelectorAll('[aria-label="File tree"] button [class*="nodeName"]')
		).map((n) => n.textContent?.trim());
		expect(names).toEqual(['apple.ts', 'file1.ts', 'file2.ts', 'File10.ts']);
	});

	it('sorts nested directory contents recursively', async () => {
		const { container } = render(FileTree, {
			allFiles: [
				entry('src', 'directory'),
				entry('src/z.ts'),
				entry('src/components', 'directory'),
				entry('src/a.ts'),
				entry('src/components/Button.ts'),
				entry('src/components/Atom.ts')
			],
			touched: []
		});

		await fireEvent.click(screen.getByText('src').closest('button') as HTMLElement);
		await fireEvent.click(screen.getByText('components').closest('button') as HTMLElement);

		const names = Array.from(
			container.querySelectorAll('[aria-label="File tree"] button [class*="nodeName"]')
		).map((n) => n.textContent?.trim());
		// src (dir) > components (dir) > Atom.ts, Button.ts (files) > a.ts, z.ts (files of src)
		expect(names).toEqual(['src', 'components', 'Atom.ts', 'Button.ts', 'a.ts', 'z.ts']);
	});

	it('renders empty directories (directory entries with no file children) as expandable', async () => {
		render(FileTree, {
			allFiles: [entry('empty-dir', 'directory')],
			touched: []
		});

		const btn = screen.getByText('empty-dir').closest('button') as HTMLElement;
		expect(btn).toHaveAttribute('aria-expanded', 'false');
		await fireEvent.click(btn);
		expect(btn).toHaveAttribute('aria-expanded', 'true');
	});
});
