# Configuration Quality Guidelines

**Status**: ACTIVE
**Last reviewed**: 2026-04-04
**Purpose**: Mandatory rules for dependency manifests, toolchain settings, runtime configuration, and repository-level configuration files.

## Scope

This guide applies to files such as:

- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`
- `src-ui/package.json`
- `src-ui/tsconfig.json`
- `src-ui/eslint.config.js`
- `src-ui/vite.config.ts`
- `src-ui/vitest.config.ts`
- Tauri capabilities, build config, and similar repository settings

## Configuration principles

Configuration must be:

- minimal
- explicit
- documented
- reproducible
- safe by default

Do not use configuration as a shortcut for avoiding proper code design.

## Dependency management rules

Before adding a dependency:

1. check whether the repository already has a suitable library
2. justify why the new dependency is necessary
3. prefer well-maintained, mainstream packages/crates with clear ownership
4. consider size, security, API stability, and long-term maintenance cost
5. remove obsolete dependencies when they stop being needed

Do not:

- add overlapping libraries for the same job without a migration plan
- add dependencies only for tiny helpers that can be implemented clearly in-house
- leave unused dependencies in manifests after refactors

## Source-of-truth rules

- keep one clear source of truth for each setting
- do not duplicate environment, build, lint, or test settings across multiple files unless the tool requires it
- when duplication is unavoidable, document why the duplication exists
- keep naming and semantics aligned across backend and frontend config

## Safety and security rules

- never hardcode secrets, tokens, or private endpoints in tracked configuration
- do not silently weaken security-sensitive settings
- any exception to secure defaults must be explicit, documented, and reviewed
- do not disable linting, tests, type checking, or CSP-like protections just to make a change pass quickly
- if a temporary bypass is unavoidable, annotate it clearly and create a follow-up task immediately

## Build and tooling rules

- keep build commands deterministic and aligned with the actual workspace layout
- prefer small targeted config changes over broad rewrites
- when changing formatting/linting/type-check/test config, verify the impact on the current codebase before committing
- do not introduce config churn that rewrites large unrelated parts of the repository

## Repository-specific expectations

### Rust / Tauri configuration

- keep Rust linting aligned with the existing Clippy and Rust lint posture
- preserve the layered backend architecture; do not add configuration that encourages bypassing it
- Tauri configuration changes must be intentional, tested, and reviewed for packaging/runtime impact

### Frontend toolchain configuration

- keep TypeScript strictness intact
- frontend linting, formatting, and test commands must stay runnable and understandable from `src-ui/`
- do not change aliasing, module resolution, or test environment settings without verifying affected imports and tests

## Refactor process for configuration changes

1. identify the exact problem the config change solves
2. confirm it cannot be solved more safely in code or existing config
3. make the smallest config change that solves it
4. update docs/comments if the behavior changed materially
5. validate the affected commands before finalizing

## Validation expectations

Run the relevant validation for the files you changed. Typical examples from this repository include:

- `cargo build`
- `cargo test`
- `cargo clippy`
- `npm run check`
- `npm run lint`
- `npm run test`
- `npm run build`

Only claim a configuration change is safe after verifying the affected path.

## Remove unimportant references

Do not keep:

- obsolete script entries
- unused dependencies
- stale comments that describe old commands or paths
- dead feature flags and fallback config branches
- duplicate aliases, ignore patterns, or overrides that no longer serve a purpose

## Configuration review checklist

Before considering a configuration change complete, verify:

- the change has one clear purpose
- no duplicate source of truth was introduced
- no security posture was silently weakened
- unused dependencies/settings were not left behind
- affected commands were validated
- related documentation was updated when the workflow changed

Any exception must be explicit and justified.
