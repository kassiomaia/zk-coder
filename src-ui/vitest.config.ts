import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { fileURLToPath } from 'url';
import path from 'path';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
	plugins: [
		svelte({
			hot: false
		})
	],
	test: {
		environment: 'jsdom',
		include: ['src/**/*.{test,spec}.{js,ts}'],
		setupFiles: ['./vitest-setup.ts'],
		coverage: {
			provider: 'v8',
			reporter: ['text', 'json', 'html']
		},
		// Exclude type-checking of test files from vitest
		// We'll rely on svelte-check for type checking
		typecheck: {
			enabled: false
		}
	},
	resolve: {
		alias: {
			$lib: path.resolve(__dirname, './src/lib')
		},
		// Force browser conditions for Svelte imports
		conditions: ['browser', 'import', 'default']
	},
	// Optimize dependencies to ensure Svelte is processed correctly
	optimizeDeps: {
		include: ['svelte', '@testing-library/svelte']
	}
});
