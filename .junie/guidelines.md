# Coder Development Guidelines

This document provides essential information for developers working on the Coder project.

## 1. Build/Configuration Instructions

### Prerequisites
- **Rust**: Latest stable version (installed via rustup).
- **Node.js**: LTS version (18+ recommended).
- **npm**: Installed with Node.js.

### Project Setup
1. Clone the repository.
2. Install frontend dependencies:
   ```bash
   cd src-ui && npm install
   ```
3. (Optional) Install Tauri CLI if not using `cargo tauri`:
   ```bash
   npm install -g @tauri-apps/cli
   ```

### Development
To run the application in development mode with hot-reloading:
```bash
cargo tauri dev
```
Alternatively, for frontend-only development:
```bash
cd src-ui && npm run dev
```

### Production Build
To build the production-ready application for your current platform:
```bash
cargo tauri build
```

---

## 2. Testing Information

### Backend (Rust)
Tests are located within the source files (unit tests) or in the `tests/` directory (integration tests).
- **Run all backend tests:**
  ```bash
  cargo test --manifest-path src-tauri/Cargo.toml
  ```

### Frontend (TypeScript/Svelte)
Tests use **Vitest** and are co-located with the source files using the `*.test.ts` naming convention.
- **Run all frontend tests:**
  ```bash
  cd src-ui && npm run test
  ```

### Adding New Tests
- **Rust**: Add a `#[cfg(test)] mod tests { ... }` block at the bottom of your module.
- **Frontend**: Create a file named `filename.test.ts` next to `filename.ts`.

### Demonstration Process
To verify the testing setup, a simple test was created and executed on both sides:

**Backend Test (`src-tauri/src/utils/demo.rs`):**
```rust
pub fn add(a: i32, b: i32) -> i32 { a + b }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_add_works() { assert_eq!(add(2, 2), 4); }
}
```

**Frontend Test (`src-ui/src/lib/utils/demo.test.ts`):**
```typescript
import { it, expect } from 'vitest';
import { multiply } from './demo';

it('multiplies two numbers', () => {
    expect(multiply(2, 3)).toBe(6);
});
```

**Execution Results:**
- Backend: `test result: ok. 5 passed; 0 failed;`
- Frontend: `Test Files 2 passed (2). Tests 4 passed (4).`

---

## 3. Additional Development Information

### Architecture & Tech Stack
- **Framework**: Tauri v2 (Rust backend + Svelte/TypeScript frontend).
- **Backend**: Rust, Tokio (async), Serde (serialization), Anyhow (error handling).
- **Frontend**: SvelteKit, TypeScript, Vite.
- **Styling**: Component-scoped CSS modules, Global CSS for theming.

### Coding Style
- **Rust**: 
  - Follow standard Rust naming conventions (`snake_case` for functions/variables, `PascalCase` for types).
  - Use `anyhow::Result` for error handling; avoid `unwrap()` or `expect()`.
  - Run `cargo clippy` and `cargo fmt` regularly.
- **TypeScript/Svelte**:
  - Components: `PascalCase` (e.g., `ChatMessage.svelte`).
  - Utility functions: `camelCase` (e.g., `formatTimestamp.ts`).
  - Strict mode enabled in TypeScript.
- **File Size Limits**:
  - Soft limit: 500 lines of code.
  - Hard limit: 800 lines of code.
  - Files exceeding these limits must be refactored into smaller, focused modules.

### Design System
Before making any styling changes, consult `src-ui/src/lib/styles/design-system.css`.
- Use CSS variables for colors, spacing, and typography.
- Avoid generic fonts (Arial, Helvetica); use the fonts specified in the design system.
- Prioritize a unique and visually appealing "Desktop First" design.

### Directory Structure
- `src-tauri/`: Rust backend code.
- `src-ui/`: Svelte frontend code.
  - `src/lib/components/`: Reusable UI components.
  - `src/lib/utils/`: TypeScript utility functions.
  - `src/lib/styles/`: Global styles and design system.
- `docs/`: Project documentation.
