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
```

## Project Structure

```
coder/
├── src-tauri/           # Rust backend
│   ├── src/
│   │   ├── agents/      # AI agent orchestration
│   │   ├── commands/    # Tauri commands (API layer)
│   │   ├── core/        # Core domain entities
│   │   ├── llm/         # LLM provider integrations
│   │   ├── nlp/         # Natural language processing
│   │   ├── storage/     # Database layer
│   │   └── tools/       # Agent tool implementations
│   └── Cargo.toml
├── src-ui/              # Svelte frontend
│   └── src/
│       ├── lib/
│       │   ├── components/    # UI components
│       │   ├── stores/        # Svelte stores
│       │   ├── styles/        # Design system
│       │   └── utils/         # Utilities
│       └── routes/            # SvelteKit routes
├── docs/                # Documentation
├── CLAUDE.md            # Project instructions
└── CHANGELOG.md         # Version history
```

## Key Features

- **AI Agent Integration**: Seamlessly integrate AI agents for code generation and task automation
- **Multi-Provider Support**: Works with OpenAI, Anthropic, and custom LLM providers
- **Workspace Management**: Manage multiple workspaces with file tracking
- **Agent Orchestration**: Plan, execute, and track complex multi-step tasks
- **Thread-Based Chat**: Organize conversations into threads with resolution states
- **File Operations**: Safe file editing with rollback capabilities

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
- [Scroll System](docs/scroll-system-architecture.md) - Auto-scroll implementation details
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
