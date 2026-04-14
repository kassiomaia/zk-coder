import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [sveltekit()],
	optimizeDeps: {
		include: ['marked', 'marked-highlight', 'highlight.js']
	},
	server: {
		// Pre-warm every Svelte component at dev-server start. vite-plugin-svelte attaches
		// the compiled CSS to each module's meta during the JS transform; if the client
		// requests the virtual `?svelte&type=style&lang.css` URL before that transform
		// completes, the plugin logs `failed to load virtual css module`. Warming the
		// JS first guarantees the cache is populated before any CSS URL lands.
		warmup: {
			clientFiles: ['./src/**/*.svelte']
		}
	}
});
