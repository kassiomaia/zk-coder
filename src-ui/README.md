# Coder UI - Frontend Application

> Svelte 5 + TypeScript frontend for the Coder AI Assistant desktop application.

## Development

```bash
# Install dependencies
npm install

# Start development server (runs on http://localhost:5173)
npm run dev

# Type checking
npm run check

# Linting
npm run lint

# Format code
npm run format

# Run tests
npm run test
```

## Building

The frontend uses Vite for building and SvelteKit with the static adapter for Tauri integration.

```bash
# Production build
npm run build

# Preview production build
npm run preview
```

## Project Structure

```
src-ui/
├── src/
│   ├── lib/
│   │   ├── components/         # Svelte components
│   │   │   ├── chat/           # Chat-related components
│   │   │   ├── config/         # Configuration UI
│   │   │   ├── layout/         # Layout components
│   │   │   └── shared/         # Shared/reusable components
│   │   ├── stores/             # Svelte stores (state management)
│   │   ├── styles/             # Design system and global styles
│   │   │   ├── design-system.css   # Design tokens
│   │   │   └── global.css          # Global styles
│   │   ├── utils/              # Utility functions
│   │   │   ├── scrollController.ts  # Scroll system
│   │   │   └── ...
│   │   ├── icons.ts            # Icon library (lucide-svelte)
│   │   └── types.ts            # Shared TypeScript types
│   └── routes/                 # SvelteKit routes
├── static/                     # Static assets
├── package.json
├── svelte.config.js
├── tsconfig.json
└── vite.config.ts
```

## Component Conventions

### File Naming
- **Components**: PascalCase (e.g., `ChatMessage.svelte`)
- **CSS modules**: Match component name (e.g., `ChatMessage.module.css`)
- **Utilities**: camelCase (e.g., `formatTimestamp.ts`)

### Component Structure

```svelte
<script lang="ts">
  // Imports
  // Props/exports
  // Reactive state
  // Effects
</script>

<!-- Template -->

<style module>
  /* Component-scoped styles */
</style>
```

## Design System

The application uses a custom design system with CSS variables defined in `src/lib/styles/design-system.css`.

### Key Tokens

```css
/* Colors */
--color-primary-500: #6366f1;
--color-bg-primary: #111827;
--color-text-primary: #f9fafb;

/* Typography */
--font-sans: 'Inter', sans-serif;
--font-mono: 'JetBrains Mono', monospace;
--text-base: 1rem;

/* Spacing */
--space-1: 0.25rem;
--space-2: 0.5rem;
/* ... */

/* Effects */
--radius-md: 0.375rem;
--transition-fast: 150ms;
```

### Styling Guidelines

1. **Always use design tokens** - Never hardcode colors, spacing, or typography
2. **CSS modules for components** - Keep styles scoped to components
3. **Desktop-first** - Design for desktop, ensure responsive for smaller screens
4. **Avoid generic fonts** - Use Inter and JetBrains Mono as defined in design system

See [Design System Synthesis](../docs/design-system-synthesis.md) for complete specifications.

## State Management

State is managed using Svelte 5 runes (`$state`, `$derived`, `$effect`) and Svelte stores.

### Global Stores

Located in `src/lib/stores/`:
- `chat.ts` - Chat state (conversations, threads, messages)
- `config.ts` - Configuration state (providers, models, API keys)
- `agent.ts` - Agent session state
- `ui.ts` - UI state (sidebar, modals, notifications)

## Tauri Integration

The frontend communicates with the Rust backend via Tauri commands:

```typescript
import { invoke } from '@tauri-apps/api/core';

// Invoke backend command
const result = await invoke('command_name', { arg1: value1 });
```

Available commands are defined in `src-tauri/src/commands/`.

## Testing

```bash
# Run tests once
npm run test

# Watch mode
npm run test:watch
```

Tests use Vitest with Testing Library for Svelte.

## Linting and Formatting

```bash
# Check linting
npm run lint

# Auto-fix issues
npm run format
```

- **ESLint** - Code quality and type checking
- **Prettier** - Code formatting

## Key Dependencies

| Package | Purpose |
|---------|---------|
| `@tauri-apps/api` | Tauri API for backend communication |
| `svelte` | UI framework |
| `@sveltejs/kit` | Full-stack framework (using static adapter) |
| `lucide-svelte` | Icon library |
| `marked` | Markdown parsing |
| `highlight.js` | Syntax highlighting |
| `dompurify` | XSS sanitization |
| `diff2html` | Diff visualization |
