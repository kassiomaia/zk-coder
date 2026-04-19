import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import { createRawSnippet } from 'svelte';
import AgentPanelFilesSection from './AgentPanelFilesSection.svelte';

function makeChildren() {
	return createRawSnippet(() => ({
		render: () => '<div data-testid="mock-child">tree</div>'
	}));
}

function props(overrides: {
	touchedCount?: number;
	totalCount?: number;
	onRefresh?: () => void;
} = {}) {
	return {
		expanded: true,
		onToggle: vi.fn(),
		touchedCount: overrides.touchedCount ?? 0,
		totalCount: overrides.totalCount ?? 0,
		onRefresh: overrides.onRefresh,
		children: makeChildren()
	};
}

describe('AgentPanelFilesSection badge', () => {
	it('renders no badge when nothing is touched and workspace has no entries', () => {
		render(AgentPanelFilesSection, props());

		expect(screen.queryByText('0/0')).not.toBeInTheDocument();
		expect(screen.queryByText('0')).not.toBeInTheDocument();
	});

	it('renders touched/total when workspace listing is present', () => {
		render(AgentPanelFilesSection, props({ touchedCount: 2, totalCount: 42 }));

		expect(screen.getByText('2/42')).toBeInTheDocument();
	});

	it('falls back to touched count alone when workspace listing is empty', () => {
		render(AgentPanelFilesSection, props({ touchedCount: 3, totalCount: 0 }));

		expect(screen.getByText('3')).toBeInTheDocument();
		expect(screen.queryByText('3/0')).not.toBeInTheDocument();
	});

	it('shows 0/N while agent has not touched anything but tree is loaded', () => {
		render(AgentPanelFilesSection, props({ touchedCount: 0, totalCount: 25 }));

		expect(screen.getByText('0/25')).toBeInTheDocument();
	});

	it('renders the refresh action in the header when onRefresh is provided', () => {
		render(AgentPanelFilesSection, props({ onRefresh: vi.fn() }));

		expect(screen.getByRole('button', { name: /refresh file tree/i })).toBeInTheDocument();
	});

	it('omits the refresh action when onRefresh is not provided', () => {
		render(AgentPanelFilesSection, props());

		expect(screen.queryByRole('button', { name: /refresh file tree/i })).not.toBeInTheDocument();
	});

	it('invokes onRefresh and does NOT toggle the section when clicked', async () => {
		const onRefresh = vi.fn();
		const onToggle = vi.fn();
		render(AgentPanelFilesSection, {
			expanded: true,
			onToggle,
			touchedCount: 0,
			totalCount: 10,
			onRefresh,
			children: makeChildren()
		});

		await fireEvent.click(screen.getByRole('button', { name: /refresh file tree/i }));

		expect(onRefresh).toHaveBeenCalledTimes(1);
		expect(onToggle).not.toHaveBeenCalled();
	});
});
