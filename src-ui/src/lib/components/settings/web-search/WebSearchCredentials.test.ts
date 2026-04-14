import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import WebSearchCredentials from './WebSearchCredentials.svelte';
import {
	deleteToolCredentials,
	getToolCredentials,
	hasToolCredentials,
	setToolCredentials,
	testWebSearchCredentials
} from '$lib/api/toolCredentials';

vi.mock('$lib/api/toolCredentials', async () => {
	const actual =
		await vi.importActual<typeof import('$lib/api/toolCredentials')>(
			'$lib/api/toolCredentials'
		);
	return {
		...actual,
		hasToolCredentials: vi.fn(),
		getToolCredentials: vi.fn(),
		setToolCredentials: vi.fn(),
		deleteToolCredentials: vi.fn(),
		testWebSearchCredentials: vi.fn()
	};
});

const mockHas = vi.mocked(hasToolCredentials);
const mockGet = vi.mocked(getToolCredentials);
const mockSet = vi.mocked(setToolCredentials);
const mockDelete = vi.mocked(deleteToolCredentials);
const mockTest = vi.mocked(testWebSearchCredentials);

describe('WebSearchCredentials', () => {
	beforeEach(() => {
		vi.clearAllMocks();
		mockHas.mockResolvedValue(false);
		mockGet.mockResolvedValue(null);
		mockSet.mockResolvedValue(undefined);
		mockDelete.mockResolvedValue(undefined);
		mockTest.mockResolvedValue({ valid: true, message: 'ok' });
	});

	async function renderClean() {
		const view = render(WebSearchCredentials);
		// Wait for the initial load effect to settle.
		await waitFor(() => expect(mockHas).toHaveBeenCalled());
		return view;
	}

	it('renders an empty form when no credentials are stored', async () => {
		await renderClean();
		const apiKey = screen.getByTestId('api-key-input') as HTMLInputElement;
		const cx = screen.getByTestId('cx-input') as HTMLInputElement;
		expect(apiKey.value).toBe('');
		expect(cx.value).toBe('');
	});

	it('prefills a masked placeholder when credentials exist', async () => {
		mockHas.mockResolvedValue(true);
		mockGet.mockResolvedValue({ apiKey: 'sk-real-secret', cx: 'cx-1' });
		render(WebSearchCredentials);
		await waitFor(() => {
			const apiKey = screen.getByTestId('api-key-input') as HTMLInputElement;
			// Masked placeholder — the real key must not leak into the DOM.
			expect(apiKey.value).not.toContain('sk-real-secret');
			expect(apiKey.value.length).toBeGreaterThan(0);
			expect(apiKey.type).toBe('password');
		});
	});

	it('toggles key visibility on reveal click', async () => {
		await renderClean();
		const apiKey = screen.getByTestId('api-key-input') as HTMLInputElement;
		const toggle = screen.getByTestId('reveal-toggle');
		expect(apiKey.type).toBe('password');
		await fireEvent.click(toggle);
		expect(apiKey.type).toBe('text');
		await fireEvent.click(toggle);
		expect(apiKey.type).toBe('password');
	});

	it('disables the Save button until both fields have values', async () => {
		await renderClean();
		const save = screen.getByRole('button', { name: /^Save$/ });
		expect(save).toBeDisabled();

		const apiKey = screen.getByTestId('api-key-input');
		const cx = screen.getByTestId('cx-input');
		await fireEvent.input(apiKey, { target: { value: 'sk-abc' } });
		await fireEvent.input(cx, { target: { value: 'cx-1' } });
		expect(save).not.toBeDisabled();
	});

	it('calls set_tool_credentials on save with the typed values', async () => {
		await renderClean();
		await fireEvent.input(screen.getByTestId('api-key-input'), {
			target: { value: 'sk-abc' }
		});
		await fireEvent.input(screen.getByTestId('cx-input'), {
			target: { value: 'cx-1' }
		});
		await fireEvent.click(screen.getByRole('button', { name: /^Save$/ }));
		await waitFor(() => {
			expect(mockSet).toHaveBeenCalledWith('search_web_search', {
				apiKey: 'sk-abc',
				cx: 'cx-1'
			});
		});
	});

	it('disables Test while the request is in flight', async () => {
		mockHas.mockResolvedValue(true);
		mockGet.mockResolvedValue({ apiKey: 'sk', cx: 'cx' });
		let resolveTest: (v: { valid: boolean; message: string }) => void = () => {};
		mockTest.mockImplementation(
			() =>
				new Promise((res) => {
					resolveTest = res;
				})
		);
		render(WebSearchCredentials);
		await waitFor(() => {
			const btn = screen.getByRole('button', { name: /^Test$/ });
			expect(btn).not.toBeDisabled();
		});

		const testBtn = screen.getByRole('button', { name: /^Test$/ });
		await fireEvent.click(testBtn);
		await waitFor(() => expect(testBtn).toBeDisabled());
		resolveTest({ valid: true, message: 'Credentials verified.' });
		await waitFor(() => expect(testBtn).not.toBeDisabled());
	});

	it('renders a verified badge after a successful test', async () => {
		mockHas.mockResolvedValue(true);
		mockGet.mockResolvedValue({ apiKey: 'sk', cx: 'cx' });
		mockTest.mockResolvedValue({ valid: true, message: 'Credentials verified.' });
		render(WebSearchCredentials);
		await waitFor(() => {
			const btn = screen.getByRole('button', { name: /^Test$/ });
			expect(btn).not.toBeDisabled();
		});
		await fireEvent.click(screen.getByRole('button', { name: /^Test$/ }));
		await waitFor(() => {
			expect(screen.getByTestId('verified-badge')).toBeInTheDocument();
			expect(screen.getByTestId('test-success').textContent).toContain('verified');
		});
	});

	it('renders an error message when the test fails', async () => {
		mockHas.mockResolvedValue(true);
		mockGet.mockResolvedValue({ apiKey: 'sk', cx: 'cx' });
		mockTest.mockResolvedValue({
			valid: false,
			message: 'Google rejected the credentials.'
		});
		render(WebSearchCredentials);
		await waitFor(() => {
			const btn = screen.getByRole('button', { name: /^Test$/ });
			expect(btn).not.toBeDisabled();
		});
		await fireEvent.click(screen.getByRole('button', { name: /^Test$/ }));
		await waitFor(() => {
			expect(screen.getByTestId('test-failure').textContent).toContain('rejected');
			expect(screen.getByTestId('failed-badge')).toBeInTheDocument();
		});
	});
});
