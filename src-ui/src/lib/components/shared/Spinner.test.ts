import { render, screen } from '@testing-library/svelte';
import { describe, it, expect } from 'vitest';
import Spinner from './Spinner.svelte';

describe('Spinner', () => {
	it('renders with default size of 16px', () => {
		render(Spinner);
		const svg = document.querySelector('svg');
		expect(svg).toBeInTheDocument();
		expect(svg).toHaveStyle({ width: '16px' });
		expect(svg).toHaveStyle({ height: '16px' });
	});

	it('renders with custom size', () => {
		render(Spinner, { size: 24 });
		const svg = document.querySelector('svg');
		expect(svg).toHaveStyle({ width: '24px' });
		expect(svg).toHaveStyle({ height: '24px' });
	});

	it('applies custom class name', () => {
		render(Spinner, { class: 'custom-spinner' });
		const svg = document.querySelector('svg');
		expect(svg).toHaveClass('custom-spinner');
	});

	it('has all spinner lines (8 lines for proper animation)', () => {
		render(Spinner);
		const lines = document.querySelectorAll('line');
		expect(lines).toHaveLength(8);
	});

	it('has animation applied', () => {
		render(Spinner);
		const svg = document.querySelector('svg');
		const styles = window.getComputedStyle(svg);
		expect(styles.animation).toContain('spin');
	});
});
