import { describe, it, expect } from 'vitest';
import { readdirSync, readFileSync, statSync } from 'fs';
import { join, relative } from 'path';

const SRC_ROOT = join(__dirname, '..', '..');
const REPO_SRC_ROOT = join(__dirname, '..', '..', '..');

function walk(dir: string, out: string[] = []): string[] {
	for (const entry of readdirSync(dir)) {
		if (entry === 'node_modules' || entry === '.svelte-kit' || entry === 'build') continue;
		const full = join(dir, entry);
		const s = statSync(full);
		if (s.isDirectory()) walk(full, out);
		else out.push(full);
	}
	return out;
}

const ALL_FILES = walk(SRC_ROOT);
const CSS_FILES = ALL_FILES.filter((f) => f.endsWith('.css'));
const SVELTE_FILES = ALL_FILES.filter((f) => f.endsWith('.svelte'));

function extractSvelteStyle(source: string): string {
	const blocks: string[] = [];
	const re = /<style(?:\s[^>]*)?>([\s\S]*?)<\/style>/g;
	let m: RegExpExecArray | null;
	while ((m = re.exec(source)) !== null) blocks.push(m[1]);
	return blocks.join('\n');
}

function stripComments(css: string): string {
	return css.replace(/\/\*[\s\S]*?\*\//g, '');
}

function collectDefinedVars(): Set<string> {
	const defined = new Set<string>();
	const re = /(--[a-zA-Z0-9_-]+)\s*:/g;
	for (const file of [...CSS_FILES, ...SVELTE_FILES]) {
		const src = readFileSync(file, 'utf-8');
		const css = file.endsWith('.svelte') ? extractSvelteStyle(src) : src;
		const clean = stripComments(css);
		let m: RegExpExecArray | null;
		while ((m = re.exec(clean)) !== null) defined.add(m[1]);
	}
	return defined;
}

interface VarUsage {
	file: string;
	line: number;
	name: string;
	hasFallback: boolean;
}

function collectVarUsages(): VarUsage[] {
	const usages: VarUsage[] = [];
	const re = /var\(\s*(--[a-zA-Z0-9_-]+)\s*(,)?/g;
	for (const file of [...CSS_FILES, ...SVELTE_FILES]) {
		const src = readFileSync(file, 'utf-8');
		const css = file.endsWith('.svelte') ? extractSvelteStyle(src) : src;
		const clean = stripComments(css);
		// Compute line numbers against the raw source's style block position
		const lines = clean.split('\n');
		for (let i = 0; i < lines.length; i++) {
			const lineRe = /var\(\s*(--[a-zA-Z0-9_-]+)\s*(,)?/g;
			let m: RegExpExecArray | null;
			while ((m = lineRe.exec(lines[i])) !== null) {
				usages.push({
					file: relative(REPO_SRC_ROOT, file),
					line: i + 1,
					name: m[1],
					hasFallback: m[2] === ','
				});
			}
		}
	}
	return usages;
}

interface EmptyRuleset {
	file: string;
	selector: string;
}

function findEmptyRulesets(): EmptyRuleset[] {
	// Per RULES.md: a comment-only body (e.g. `{ /* inherited */ }`) is acceptable —
	// it avoids Vite's CSS module loader failure on literal `{}`. Only flag bodies
	// that are whitespace-only.
	const results: EmptyRuleset[] = [];
	const re = /([^{};\/\n][^{};\/]*?)\{([^{}]*)\}/g;
	for (const file of [...CSS_FILES, ...SVELTE_FILES]) {
		const src = readFileSync(file, 'utf-8');
		const css = file.endsWith('.svelte') ? extractSvelteStyle(src) : src;
		let m: RegExpExecArray | null;
		while ((m = re.exec(css)) !== null) {
			const selector = m[1].trim();
			const body = m[2];
			if (!selector) continue;
			if (selector.startsWith('@')) continue;
			if (!/[.#:&*\[][\w-]*|[a-zA-Z][\w-]*\s*(?:[.#:&\[,>\s]|$)/.test(selector)) continue;
			if (body.trim().length > 0) continue;
			results.push({ file: relative(REPO_SRC_ROOT, file), selector });
		}
	}
	return results;
}

// Pseudo-classes that look like CSS but aren't — they're jQuery/pre-selector-4 extensions.
// Svelte's CSS parser rejects these and marks the virtual stylesheet invalid, which
// cascades into `failed to load virtual css module` warnings for every file that
// imports the broken component. Flag them explicitly so the next offender is caught
// at test time instead of at runtime.
const INVALID_PSEUDO_CLASSES = [':contains(', ':parent(', ':eq(', ':gt(', ':lt('];

interface InvalidSelector {
	file: string;
	line: number;
	snippet: string;
	pseudo: string;
}

function findInvalidSelectors(): InvalidSelector[] {
	const results: InvalidSelector[] = [];
	for (const file of [...CSS_FILES, ...SVELTE_FILES]) {
		const src = readFileSync(file, 'utf-8');
		const css = file.endsWith('.svelte') ? extractSvelteStyle(src) : src;
		const lines = css.split('\n');
		for (let i = 0; i < lines.length; i++) {
			for (const pseudo of INVALID_PSEUDO_CLASSES) {
				if (lines[i].includes(pseudo)) {
					results.push({
						file: relative(REPO_SRC_ROOT, file),
						line: i + 1,
						snippet: lines[i].trim(),
						pseudo
					});
				}
			}
		}
	}
	return results;
}

describe('CSS integrity', () => {
	it('every var(--token) references a defined token (or provides a fallback)', () => {
		const defined = collectDefinedVars();
		const usages = collectVarUsages();
		const missing = usages.filter((u) => !u.hasFallback && !defined.has(u.name));
		const grouped = new Map<string, VarUsage[]>();
		for (const u of missing) {
			const key = u.name;
			if (!grouped.has(key)) grouped.set(key, []);
			grouped.get(key)!.push(u);
		}
		const report = Array.from(grouped.entries())
			.map(
				([name, items]) =>
					`${name}\n  ${items.map((i) => `${i.file}:${i.line}`).join('\n  ')}`
			)
			.join('\n');
		expect(
			missing,
			`Undefined CSS variables (add to design-system.css or supply a fallback):\n${report}`
		).toEqual([]);
	});

	it('contains no empty CSS rulesets', () => {
		const empties = findEmptyRulesets();
		const report = empties.map((e) => `${e.file}: ${e.selector} {}`).join('\n');
		expect(empties, `Empty CSS rulesets found:\n${report}`).toEqual([]);
	});

	it('uses no invalid pseudo-class selectors (jQuery extensions are not CSS)', () => {
		const invalid = findInvalidSelectors();
		const report = invalid
			.map((i) => `${i.file}:${i.line}  ${i.pseudo}...  →  ${i.snippet}`)
			.join('\n');
		expect(
			invalid,
			`Invalid pseudo-class selectors found (these break Svelte's CSS parser and produce "failed to load virtual css module" warnings for every dependent component):\n${report}`
		).toEqual([]);
	});
});
