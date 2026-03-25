import adapter from '@sveltejs/adapter-static';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	kit: {
		adapter: adapter({
			pages: 'build',
			assets: 'build',
			fallback: 'index.html',
			precompress: false,
			strict: true
		})
	},
	// Exclude test files from svelte-check
	service: {
		exclude: ['**/*.test.ts', '**/*.test.js', '**/*.spec.ts', '**/*.spec.js']
	}
};

export default config;
