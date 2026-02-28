# Contributing to Coder

Thank you for your interest in contributing to Coder! This document provides guidelines and instructions for contributing.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Development Setup](#development-setup)
- [Development Process](#development-process)
- [Code Style](#code-style)
- [Commit Guidelines](#commit-guidelines)
- [Pull Request Process](#pull-request-process)

## Code of Conduct

Be respectful and inclusive. We welcome contributions from everyone.

## Development Setup

### Prerequisites

- Rust 1.77.2 or later
- Node.js 18 or later
- npm 9 or later

### Getting Started

1. Clone the repository
2. Install frontend dependencies:
   ```bash
   cd src-ui && npm install
   ```
3. Build the Rust backend:
   ```bash
   cargo build --manifest-path src-tauri/Cargo.toml
   ```
4. Run in development mode:
   ```bash
   cargo tauri dev
   ```

## Development Process

1. Create a feature branch from `master`
2. Make your changes
3. Ensure tests pass and code is linted
4. Submit a pull request

### Branching Strategy

- `master` - Main branch for releases
- `feature/*` - New features
- `bugfix/*` - Bug fixes
- `docs/*` - Documentation updates

## Code Style

### Rust

- Follow standard Rust naming conventions
- Run `cargo clippy` before committing
- Write unit tests for new functionality
- Document public APIs with doc comments

### TypeScript/Svelte

- Use TypeScript strict mode
- Follow ESLint and Prettier configurations
- One component per file
- Use CSS modules for component styles

### Running Linters

```bash
# Rust
cargo clippy --manifest-path src-tauri/Cargo.toml

# Frontend
cd src-ui && npm run lint
```

## Commit Guidelines

- Use clear, descriptive commit messages
- Reference issues when applicable
- Keep commits focused and atomic

### Commit Message Format

```
type(scope): description

[optional body]

[optional footer]
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`

## Pull Request Process

1. Ensure all tests pass
2. Update documentation if needed
3. Update CHANGELOG.md with your changes
4. Request review from maintainers
5. Address review feedback

### PR Checklist

- [ ] Code compiles without errors
- [ ] All tests pass
- [ ] Linters pass without warnings
- [ ] Documentation updated if needed
- [ ] CHANGELOG.md updated
- [ ] PR description is clear and complete
