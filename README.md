# Coder - AI Assistant Desktop Application

> A modern desktop application built with Rust and Tauri designed to help developers automate activities and write software more efficiently using AI agents.

## Quick Start

### Prerequisites

- **Rust** 1.77.2 or later
- **Node.js** 18 or later
- **npm** 9 or later

### Installation

```bash
# Clone the repository
git clone https://github.com/your-org/coder.git
cd coder

# Install frontend dependencies
cd src-ui && npm install && cd ..

# Run in development mode
cargo tauri dev
```

### Building for Production

```bash
cargo tauri build

**Module documentation pointers:**
- Backend layered modules (`core`, `application`, `infrastructure`, `commands`): `docs/architecture.md`
- Thread locking module behavior and UX contract: `docs/features/thread-locking.md`
- SQLite database files, schema, and debugging workflow: `docs/database-reference.md`

## Key Features

- **Threaded Conversations**: Chat organized by conversations and threads with resolve/archive controls
- **Agent Sessions with Approvals**: Long-running agent execution with user approval checkpoints
- **Safe File Changes and Rollback**: Session-level snapshots and rollback commands for edited files
- **Streaming LLM Responses**: Token streaming, cancel support, and provider/model selection flows
- **Document Artifacts**: Save, list, download, and manage generated artifacts per session
- **Multi-Provider LLM Integration**: OpenAI and Anthropic provider infrastructure with extensible adapters

## Development

### Backend (Rust)

```bash
# From project root
cargo build              # Build the project
cargo test               # Run tests
cargo clippy             # Run linter
```

### Frontend (Svelte)

```bash
# From src-ui/
npm run dev              # Development server
npm run build            # Production build
npm run check            # Type checking
npm run lint             # ESLint
```

### Design System

The application uses a custom design system with CSS variables. See `src-ui/src/lib/styles/design-system.css` for tokens.

**Key Principles:**
- Desktop-first design
- Dark mode by default
- Custom CSS modules for component-scoped styles
- Inter + JetBrains Mono typography

## Documentation

- [Architecture Overview](docs/architecture.md) - System architecture and module descriptions
- [Contributing Guide](docs/CONTRIBUTING.md) - Contribution guidelines
- [Design System](docs/design-system-synthesis.md) - Design tokens and component specifications

## Tech Stack

| Layer | Technology |
|-------|-----------|
| **Backend** | Rust, Tauri v2, tokio, rusqlite |
| **Frontend** | Svelte 5, SvelteKit, TypeScript, Vite |
| **Styling** | Custom CSS modules with design tokens |
| **LLM** | OpenAI, Anthropic (extensible) |

## License

MIT

## Authors

Coder Team
