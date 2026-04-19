# Findings

> Source: architecture audit (2026-04-19) of shell-config visibility in the agent
> `execute_command` tool. This is not a full code review — a single architectural
> finding was raised and is tracked here for resolution traceability.

## Finding 1 — Hardcoded `sh -c` in `SpawnEngine::build_command`

**Severity:** Critical
**Finding confidence:** High
**Location:** `src-tauri/src/infrastructure/agent/runtime/spawn.rs:811-838`

### Description

`SpawnEngine::build_command` constructs shell-mode commands by calling `Command::new("sh").arg("-c").arg(...)` unconditionally. There is no platform branching anywhere in the spawn stack (`rg 'cfg(target_os' src-tauri/` confirms zero usage outside file-permission helpers).

Consequences:

1. **Windows**: `sh` does not exist. Every shell-mode spawn fails at process creation.
2. **macOS**: `sh` is a minimal POSIX shell. The user's preferred shell (zsh since Catalina) and its `~/.zshrc` customizations — including PATH modifications from Homebrew and version managers (asdf, nvm, pyenv) — are invisible to spawned commands.
3. **Linux**: `sh -c` works, but the same lack of profile sourcing hides user customizations from the agent. (Left out of scope for this change per user direction.)

### Resolution

**Status:** Fixed (macOS + Windows only; Linux intentionally deferred)
**Confidence verified:** Yes — finding confirmed by direct code inspection and by Explore-agent sweep of `cfg(target_os)` usage.
**Commit:** Introduces `ShellStrategy` trait in `src-tauri/src/infrastructure/agent/runtime/shell.rs` with `MacOsShell` and `WindowsShell` implementations. `SpawnEngine::build_command` delegates to a platform-branching `build_shell_command` helper. Linux path is byte-identical to pre-fix behavior. `CODER_SHELL_MODE=user` opts into user-profile sourcing on macOS/Windows only.
**Tests added:**
- `shell::tests::detect_mode_returns_plain_by_default`
- `shell::tests::detect_mode_returns_user_for_user_value`
- `shell::tests::detect_mode_unknown_value_returns_plain`
- `shell::tests::macos::plain_uses_sh_dash_c` (cfg-gated)
- `shell::tests::macos::user_zsh_dot_sources_zshrc` (cfg-gated)
- `shell::tests::macos::user_bash_dot_sources_bashrc` (cfg-gated)
- `shell::tests::macos::user_fish_sources_config_fish` (cfg-gated)
- `shell::tests::macos::user_unknown_shell_falls_back_to_plain` (cfg-gated)
- `shell::tests::macos::user_shell_unset_falls_back_to_plain` (cfg-gated)
- `shell::tests::macos::family_classification_from_path` (cfg-gated)
- `shell::tests::windows::plain_uses_powershell_noprofile` (cfg-gated)
- `shell::tests::windows::user_omits_noprofile` (cfg-gated)
- `shell::tests::windows::encoded_command_decodes_to_utf16le_roundtrip` (cfg-gated)
- `shell::tests::windows::encoded_command_handles_ascii` (cfg-gated)
- `shell::tests::windows::encoded_command_handles_empty` (cfg-gated)
- `shell::tests::windows::base64_encode_padding` (cfg-gated)
- `spawn::tests::linux_ignores_coder_shell_mode` (Linux regression — confirms `CODER_SHELL_MODE` has no effect on Linux)
**Deviations from plan:**
- Plan proposed adding `shell_strategy: Arc<dyn ShellStrategy>` to `SpawnPolicy`. Implementation instead uses a private `build_shell_command()` helper function that `cfg`-branches on target. Rationale: `SpawnPolicy` is shared by all platforms (including Linux), and adding a field that would be `None` or an unused baseline on Linux would widen public surface without benefit. The helper keeps the platform branching contained to one function while preserving the strategy abstraction on mac/windows.
- Plan listed a `~/.zshrc` integration test. Not implemented in this first pass — these tests require real macOS/Windows CI runners, which are not currently available. The unit tests fully cover argv construction. Integration tests are listed as a follow-up in the plan's "missing context" section.
