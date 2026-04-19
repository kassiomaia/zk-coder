# Code Review Summary

> Source: architecture audit (2026-04-19) of shell-config visibility in the
> agent `execute_command` tool. Single finding, single fix.

## Scope

One architectural finding: `SpawnEngine::build_command` hardcodes `sh -c`,
which is broken on Windows and hides user shell customizations on macOS.

## Resolution Summary

- Fixes implemented: 1 of 1 planned (scoped to macOS + Windows; Linux
  intentionally deferred per user direction)
- Fixes deferred: 0 (Linux user-profile sourcing is an explicit non-goal of
  this release; it is not a deferred fix, it is out of scope)
- False positives found: 0
- Tests added: 17 unit tests (3 cross-platform, 7 macOS-gated, 6 Windows-gated,
  1 Linux regression)
- Frontend changes required: No

### Fix Log

| # | Finding | Status | Confidence Verified | Tests Added | Notes |
|---|---------|--------|---------------------|-------------|-------|
| 1 | Hardcoded `sh -c` in `SpawnEngine::build_command` | Fixed (mac/windows only) | Yes | 17 | Introduces `ShellStrategy` trait in platform-gated `shell.rs`; Linux path byte-identical. Integration tests requiring real mac/windows CI are a documented follow-up. |

### Verification

- `cargo build --lib` — passes (43 pre-existing warnings, 0 new warnings introduced)
- `cargo test --lib infrastructure::agent::runtime::spawn` — 9/9 pass including the new `linux_ignores_coder_shell_mode` regression test
- Full `cargo test --lib` — 1929/1930 pass; one pre-existing unrelated failure in `infrastructure::sqlite::migrations::chat::tests::test_derive_thread_title_long` confirmed not caused by this change (reproduced on the pre-change baseline)
- `cargo clippy --lib` — one pedantic `missing_const_for_fn` suggestion on `shell_strategy_label()`; not an error under the project's `warn`-level lint policy

### Out of Scope (Explicit Non-Goals)

- Linux user-profile sourcing (`CODER_SHELL_MODE=user` ignored on Linux)
- UI settings surface for shell preference
- `pwsh.exe` (PowerShell 7+) autodetection on Windows
- Per-workspace shell configuration persistence in SQLite
