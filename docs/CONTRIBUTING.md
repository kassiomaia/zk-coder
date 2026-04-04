# Contributing to Coder

**Status**: ACTIVE
**Last reviewed**: 2026-04-04
**Purpose**: Contributor workflow, setup, and review expectations for this repository

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
   cd src-ui && npm install && cd ..
   ```
3. Run in development mode:
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

### File Size Limits

- **Soft limit**: 500 lines per file
- **Hard limit**: 800 lines per file
- When files exceed the soft limit, refactor into smaller, focused modules

### Running Linters

```bash
# Rust
cargo clippy

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

**Types:** `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`

**Examples:**
```
feat(chat): add message editing capability
fix(scroll): prevent auto-scroll during user interaction
docs(readme): update installation instructions
```

## Pull Request Process

1. Ensure all tests pass
2. Update documentation if needed
3. Update CHANGELOG.md with your changes
4. Request review from maintainers
5. Address review feedback

### PR Checklist

- [ ] Code compiles without errors
- [ ] All tests pass (`cargo test` and `npm test`)
- [ ] Linters pass without warnings (`cargo clippy` and `npm run lint`)
- [ ] Documentation updated if needed
- [ ] CHANGELOG.md updated
- [ ] PR description is clear and complete

### PR Description Template

```markdown
## Summary
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing
How was this tested?

## Checklist
- [ ] Tests pass
- [ ] Linting passes
- [ ] Documentation updated
```
