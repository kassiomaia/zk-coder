import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { fileURLToPath } from 'url';
import path from 'path';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
	plugins: [svelte({ hot: false })],
	test: {
		environment: 'happy-dom',
		include: ['src/**/*.{test,spec}.{js,ts}'],
		setupFiles: ['./vitest-setup.ts'],
		coverage: {
			provider: 'v8',
			reporter: ['text', 'json', 'html']
		}
	},
	resolve: {
		alias: {
			$lib: path.resolve(__dirname, './src/lib')
		}
	}
});
