/**
 * Lock in the renderer-registry contract: every registered tool has the
 * three required fields, aliases point at the *same* instance as the
 * canonical name (reference equality), and unknown names fall back to the
 * default renderer.
 */
import { describe, expect, it } from 'vitest';
import { _registryForTests, defaultRenderer, getToolRenderer } from './registry';

describe('tool-renderers registry', () => {
	const registry = _registryForTests();

	it('returns the default renderer for unknown tool names', () => {
		expect(getToolRenderer('no_such_tool')).toBe(defaultRenderer);
		expect(getToolRenderer('')).toBe(defaultRenderer);
	});

	it('returns the same renderer instance for aliases', () => {
		expect(getToolRenderer('fs_read_file')).toBe(getToolRenderer('read'));
		expect(getToolRenderer('fs_read_file')).toBe(getToolRenderer('read_file'));
		expect(getToolRenderer('fs_write_file')).toBe(getToolRenderer('write'));
		expect(getToolRenderer('fs_glob')).toBe(getToolRenderer('glob'));
		expect(getToolRenderer('fs_grep')).toBe(getToolRenderer('grep'));
		expect(getToolRenderer('execute_command')).toBe(getToolRenderer('bash'));
		expect(getToolRenderer('web_request')).toBe(getToolRenderer('web_fetch'));
		expect(getToolRenderer('web_fetch')).toBe(getToolRenderer('webfetch'));
	});

	it('every registered renderer has an icon, getSummary, and getOutcome', () => {
		for (const [name, renderer] of Object.entries(registry)) {
			expect(renderer, `renderer missing for ${name}`).toBeTruthy();
			expect(renderer.icon, `${name} missing icon`).toBeDefined();
			expect(typeof renderer.getSummary, `${name} getSummary not a fn`).toBe('function');
			expect(typeof renderer.getOutcome, `${name} getOutcome not a fn`).toBe('function');
		}
	});

	it('default renderer uses the tool name as its title', () => {
		const summary = defaultRenderer.getSummary(undefined, 'mystery_tool');
		expect(summary.title).toBe('mystery_tool');
	});

	it('default renderer exposes the first arg value as subtitle when present', () => {
		const summary = defaultRenderer.getSummary({ foo: 'bar' } as unknown, 'other');
		expect(summary.subtitle).toBe('bar');
	});

	it('default renderer does not stringify array/object first values', () => {
		const arrArgs = { subtasks: [{ title: 'a' }, { title: 'b' }] };
		expect(
			defaultRenderer.getSummary(arrArgs as unknown, 'agent_plan_subtasks').subtitle
		).toBeUndefined();

		const objArgs = { config: { foo: 1 } };
		expect(defaultRenderer.getSummary(objArgs as unknown, 'x').subtitle).toBeUndefined();
	});

	it('agent_plan_subtasks renderer shows plan title and count-based subtitle', () => {
		const r = getToolRenderer('agent_plan_subtasks');
		expect(r).not.toBe(defaultRenderer);
		const s = r.getSummary(
			{ subtasks: [{ title: 'Analyze code' }, { title: 'Implement' }] } as unknown,
			'agent_plan_subtasks'
		);
		expect(s.title).toBe('Planned subtasks');
		expect(s.subtitle).toBe('Analyze code (+1 more)');
	});

	it('default renderer maps status to outcome phrase', () => {
		expect(defaultRenderer.getOutcome(undefined, 'pending')).toBe('Executing...');
		expect(defaultRenderer.getOutcome(undefined, 'error')).toBe('Action failed');
		expect(defaultRenderer.getOutcome(undefined, 'success')).toBe('Completed');
	});

	it('fs renderers tolerate undefined args without throwing', () => {
		const read = getToolRenderer('fs_read_file');
		const write = getToolRenderer('fs_write_file');
		const glob = getToolRenderer('fs_glob');
		const grep = getToolRenderer('fs_grep');
		expect(() => read.getSummary(undefined, 'fs_read_file')).not.toThrow();
		expect(() => write.getSummary(undefined, 'fs_write_file')).not.toThrow();
		expect(() => glob.getSummary(undefined, 'fs_glob')).not.toThrow();
		expect(() => grep.getSummary(undefined, 'fs_grep')).not.toThrow();
	});

	it('fs_read outcome includes a line-count derived from the content string', () => {
		const read = getToolRenderer('fs_read_file');
		const out = read.getOutcome({ content: 'line1\nline2\nline3' } as unknown, 'success');
		expect(out).toBe('Read 3 lines');
	});

	it('fs_glob outcome handles both Array and { paths } result shapes', () => {
		const glob = getToolRenderer('fs_glob');
		expect(glob.getOutcome(['a', 'b'] as unknown, 'success')).toBe('Found 2 matching files');
		expect(
			glob.getOutcome({ paths: ['a', 'b', 'c'] } as unknown, 'success')
		).toBe('Found 3 matching files');
	});

	it('fs_grep outcome handles both Array and object result shapes', () => {
		const grep = getToolRenderer('fs_grep');
		expect(grep.getOutcome([1, 2, 3] as unknown, 'success')).toBe(
			'Found matches in 3 files'
		);
		expect(grep.getOutcome({ a: 1, b: 2 } as unknown, 'success')).toBe(
			'Found matches in 2 files'
		);
	});

	it('bash outcome branches on exit_code', () => {
		const bash = getToolRenderer('bash');
		expect(bash.getOutcome({ exit_code: 0 } as unknown, 'success')).toBe(
			'Command finished successfully'
		);
		expect(bash.getOutcome({ exit_code: 1 } as unknown, 'success')).toBe(
			'Failed with exit code 1'
		);
	});

	it('renderer summary titles match the pre-refactor copy', () => {
		expect(getToolRenderer('fs_read_file').getSummary({ filePath: '/a' }, 'fs_read_file').title).toBe('Read file');
		expect(
			getToolRenderer('fs_write_file').getSummary({ filePath: '/a' }, 'fs_write_file').title
		).toBe('Modified file');
		expect(getToolRenderer('fs_glob').getSummary({ pattern: '*.ts' }, 'fs_glob').title).toBe(
			'Searched files'
		);
		expect(getToolRenderer('fs_grep').getSummary({ pattern: 'foo' }, 'fs_grep').title).toBe(
			'Searched content'
		);
		expect(getToolRenderer('bash').getSummary({ command: 'ls' }, 'bash').title).toBe('Ran command');
		expect(getToolRenderer('web_fetch').getSummary({ url: 'https://x' }, 'web_fetch').title).toBe(
			'Fetched URL'
		);
		expect(getToolRenderer('skill').getSummary({ name: 'foo' }, 'skill').title).toBe('Loaded skill');
		expect(
			getToolRenderer('task').getSummary({ description: 'd' }, 'task').title
		).toBe('Spawned agent');
	});
});
