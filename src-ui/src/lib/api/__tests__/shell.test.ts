/**
 * Tests for the shell URL wrapper. The real `openUrl` goes through a Tauri
 * plugin; we mock the plugin and verify the wrapper's validation.
 */
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { isOpenableUrl, openUrl } from '../shell';
import { openUrl as openUrlPlugin } from '@tauri-apps/plugin-opener';

vi.mock('@tauri-apps/plugin-opener', () => ({
	openUrl: vi.fn()
}));

const mockOpen = vi.mocked(openUrlPlugin);

describe('shell.openUrl', () => {
	beforeEach(() => {
		vi.clearAllMocks();
		mockOpen.mockResolvedValue(undefined);
	});

	it('invokes the plugin for https URLs', async () => {
		await openUrl('https://svelte.dev/docs');
		expect(mockOpen).toHaveBeenCalledWith('https://svelte.dev/docs');
	});

	it('invokes the plugin for http URLs', async () => {
		await openUrl('http://example.com/');
		expect(mockOpen).toHaveBeenCalledWith('http://example.com/');
	});

	it('rejects a javascript scheme without invoking the plugin', async () => {
		await expect(openUrl('javascript:alert(1)')).rejects.toThrow(/scheme/);
		expect(mockOpen).not.toHaveBeenCalled();
	});

	it('rejects a file scheme', async () => {
		await expect(openUrl('file:///etc/passwd')).rejects.toThrow(/scheme/);
		expect(mockOpen).not.toHaveBeenCalled();
	});

	it('rejects a malformed URL', async () => {
		await expect(openUrl('not a url')).rejects.toThrow(/valid absolute URL/);
		expect(mockOpen).not.toHaveBeenCalled();
	});
});

describe('isOpenableUrl', () => {
	it('accepts http and https', () => {
		expect(isOpenableUrl('https://x.com')).toBe(true);
		expect(isOpenableUrl('http://x.com')).toBe(true);
	});

	it('rejects other schemes', () => {
		expect(isOpenableUrl('ftp://x.com')).toBe(false);
		expect(isOpenableUrl('javascript:alert(1)')).toBe(false);
	});

	it('rejects malformed URLs', () => {
		expect(isOpenableUrl('not a url')).toBe(false);
		expect(isOpenableUrl('')).toBe(false);
	});
});
