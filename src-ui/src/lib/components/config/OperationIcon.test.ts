import {render} from '@testing-library/svelte';
import {describe, expect, it} from 'vitest';
import OperationIcon from './OperationIcon.svelte';
import type {OperationType} from '$lib/types/config';

describe('OperationIcon', () => {
	const iconTypes: OperationType[] = ['analysis', 'code', 'vision', 'audio', 'tips', 'review'];

	describe.each(iconTypes)('rendering %s icon', (type) => {
		it(`renders ${type} icon as SVG`, () => {
			const { container } = render(OperationIcon, { type });
			const svg = container.querySelector('svg');
			expect(svg).toBeInTheDocument();
		});

		it(`has correct stroke attributes for ${type}`, () => {
			const { container } = render(OperationIcon, { type });
			const svg = container.querySelector('svg');
			expect(svg).toHaveAttribute('fill', 'none');
			expect(svg).toHaveAttribute('stroke-width', '2');
			expect(svg).toHaveAttribute('stroke-linecap', 'round');
			expect(svg).toHaveAttribute('stroke-linejoin', 'round');
		});
	});

	it('renders analysis icon with circle and line', () => {
		const { container } = render(OperationIcon, { type: 'analysis' });
		const circle = container.querySelector('circle');
		const line = container.querySelector('line');
		expect(circle).toBeInTheDocument();
		expect(line).toBeInTheDocument();
	});

	it('renders code icon with two polylines', () => {
		const { container } = render(OperationIcon, { type: 'code' });
		const polylines = container.querySelectorAll('polyline');
		expect(polylines).toHaveLength(2);
	});

	it('renders vision icon with path and circle', () => {
		const { container } = render(OperationIcon, { type: 'vision' });
		const paths = container.querySelectorAll('path');
		const circle = container.querySelector('circle');
		expect(paths).toHaveLength(1);
		expect(circle).toBeInTheDocument();
	});

	it('renders audio icon with paths and lines', () => {
		const { container } = render(OperationIcon, { type: 'audio' });
		const paths = container.querySelectorAll('path');
		const lines = container.querySelectorAll('line');
		expect(paths.length).toBeGreaterThan(0);
		expect(lines.length).toBeGreaterThan(0);
	});

	it('renders tips icon with paths', () => {
		const { container } = render(OperationIcon, { type: 'tips' });
		const paths = container.querySelectorAll('path');
		expect(paths.length).toBeGreaterThan(0);
	});

	it('renders review icon with path and polyline', () => {
		const { container } = render(OperationIcon, { type: 'review' });
		const paths = container.querySelectorAll('path');
		const polyline = container.querySelector('polyline');
		expect(paths.length).toBeGreaterThan(0);
		expect(polyline).toBeInTheDocument();
	});

	it('renders with custom size', () => {
		const { container } = render(OperationIcon, { type: 'analysis', size: 24 });
		const svg = container.querySelector('svg') as SVGElement;
		expect(svg.getAttribute('width')).toBe('24');
		expect(svg.getAttribute('height')).toBe('24');
	});

	it('applies custom color via style', () => {
		const { container } = render(OperationIcon, {
			type: 'code',
			color: '#ff0000'
		});
		const svg = container.querySelector('svg') as SVGElement;
		// Color is set via style:color attribute which gets applied as CSS color
		expect(svg.style.color).toBe('rgb(255, 0, 0)');
	});
});
