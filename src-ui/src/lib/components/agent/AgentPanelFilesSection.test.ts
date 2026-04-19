import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import { createRawSnippet } from 'svelte';
import AgentPanelFilesSection from './AgentPanelFilesSection.svelte';

function makeChildren() {
	return createRawSnippet(() => ({
		render: () => '<div data-testid="mock-child">tree</div>'
	}));
}

function props(
	overrides: {
		touchedCount?: number;
		onRefresh?: () => void;
	} = {}
) {
	return {
		expanded: true,
		onToggle: vi.fn(),
		touchedCount: overrides.touchedCount ?? 0,
		onRefresh: overrides.onRefresh,
		children: makeChildren()
	};
}

describe('AgentPanelFilesSection badge', () => {
	it('renders no badge when nothing is touched', () => {
		render(AgentPanelFilesSection, props());

		expect(screen.queryByText('0')).not.toBeInTheDocument();
	});

	it('renders the touched count when non-zero', () => {
		render(AgentPanelFilesSection, props({ touchedCount: 3 }));

		expect(screen.getByText('3')).toBeInTheDocument();
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
			onRefresh,
			children: makeChildren()
		});

		await fireEvent.click(screen.getByRole('button', { name: /refresh file tree/i }));

		expect(onRefresh).toHaveBeenCalledTimes(1);
		expect(onToggle).not.toHaveBeenCalled();
	});
});
