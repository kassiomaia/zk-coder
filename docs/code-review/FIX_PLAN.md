# Fix Plan

Generated from the architecture audit of shell-config visibility in the agent `execute_command` tool (see `src-tauri/src/infrastructure/agent/runtime/spawn.rs:811`). This plan is deliberately scoped per user direction:

- **macOS and Windows**: new shell strategies implemented (plain + opt-in user-profile sourcing).
- **Linux**: behavior remains byte-identical to today — `sh -c` only. **Linux does NOT use user-profile sourcing under any circumstances**; `CODER_SHELL_MODE=user` is silently ignored on Linux because the env var is never read on that target. This is intentional: Linux user-profile sourcing is deferred until it can be validated on non-Linux hardware first, and because no Linux test surface currently guarantees that `.bashrc`/`.zshrc` sourcing won't regress the existing shell invocation.

New code is gated so it does not compile on Linux, and tests are gated so they do not run on Linux CI.

## Findings Selected for Fix Planning

| # | Severity | Confidence | Finding | Why Selected |
|---|----------|------------|---------|--------------|
| 1 | Critical | High | `build_command()` in `spawn.rs` hardcodes `Command::new("sh")`, which is broken by construction on Windows and cannot surface `.zshrc` customization on macOS. | User-directed scope: make mac + windows work without touching Linux. |

## Findings Deferred

| Finding | Reason Deferred |
|---------|-----------------|
| Linux `PosixUserShell` (opt-in `.bashrc`/`.zshrc` sourcing on Linux) | **Explicitly deferred.** Linux stays on `sh -c` with no user-profile sourcing. `CODER_SHELL_MODE=user` has no effect on Linux — the env var is not read on this target. Decision: ship mac/windows first, validate on real hardware, then (optionally) layer Linux support in a later PR. |
| UI/DB surface for per-user shell preference | Deferred to a later phase; the first delivery uses a process-level env var (`CODER_SHELL_MODE`) as the opt-in. |
| `pwsh.exe` (PowerShell 7+) auto-detection with fallback to `powershell.exe` | Out of scope for first PR; `powershell.exe` is universally present on supported Windows versions. Can layer detection later. |

---

## Fix 1: Cross-platform shell execution (macOS + Windows only)

**Source Finding:** Critical from architecture audit
**Finding Confidence:** High
**Location:** `src-tauri/src/infrastructure/agent/runtime/spawn.rs:811-838` (`SpawnEngine::build_command`)
**Execution Status:** Done
**Deviations:** `SpawnPolicy` was not extended with a `shell_strategy` field. Instead, a private `build_shell_command()` helper `cfg`-branches on target and delegates to `shell::default_strategy()` on macOS/Windows. Rationale is documented in the Resolution section of `FINDINGS.md`. The `shellStrategy` telemetry field is emitted via a `shell_strategy_label()` helper rather than reading from policy. All other plan details implemented as written.

### Root Cause Analysis

`build_command` constructs a `tokio::process::Command` directly:

```rust
SpawnMode::Shell => {
    let mut command = Command::new("sh");
    command.arg("-c").arg(shell_command);
    command
}
```

Two problems:

1. **Windows**: `sh` does not exist on vanilla Windows. Every shell-mode invocation from the agent fails at process-creation time (`SpawnFailed`). The audit confirmed **zero** `cfg(target_os)` branching anywhere in `spawn.rs` — no Windows codepath exists at all.
2. **macOS**: `sh` technically works but resolves to a minimal POSIX shell. The user's preferred shell (zsh since Catalina) and its `~/.zshrc` customizations (aliases, PATH modifications from brew, version managers like asdf/nvm/pyenv) are invisible to the spawned command. Agents cannot discover binaries the user has interactively.

The root cause is a missing abstraction boundary. `SpawnEngine` mixes two responsibilities:
- Process lifecycle (concurrency, streaming, cancellation, telemetry) — well-structured
- Shell-command construction — hardcoded, monolithic, platform-unaware

Without isolating the second responsibility, any platform fix accumulates `if cfg!(...)` branches inside a method that should not know about shell syntax.

### Proposed Change

Introduce a `ShellStrategy` trait as the platform seam. Provide **macOS** and **Windows** implementations only. On Linux, `build_command` keeps its current code path unchanged (byte-identical). All new code is gated with `#[cfg(any(target_os = "macos", target_os = "windows"))]` so Linux builds do not compile the new strategies at all.

#### File additions / modifications

| File | Change |
|------|--------|
| `src-tauri/src/infrastructure/agent/runtime/shell.rs` | **NEW.** Defines `ShellStrategy` trait, `ShellMode` enum, `detect_mode()` helper, and the macOS/Windows implementations. Entire file gated with `#[cfg(any(target_os = "macos", target_os = "windows"))]`. |
| `src-tauri/src/infrastructure/agent/runtime/mod.rs` | Add conditional `pub mod shell;` with the same `cfg` gate. |
| `src-tauri/src/infrastructure/agent/runtime/spawn.rs` | In `build_command`, branch on `cfg(target_os)`: Linux path identical to today; macOS and Windows delegate to `shell::default_strategy().build(cmd, cwd)`. |
| `src-tauri/src/infrastructure/tools/system/execute_command_tests.rs` | Add platform-gated test modules: mac tests under `#[cfg(target_os = "macos")]`, Windows tests under `#[cfg(target_os = "windows")]`. |
| `docs/features/shell-execution-strategy.md` | **NEW.** Documents behavior per platform, the `CODER_SHELL_MODE` env-var opt-in, known limitations (PowerShell execution policy, `$PROFILE` load failures). |

#### `ShellStrategy` trait (pseudocode)

```rust
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub trait ShellStrategy: Send + Sync + std::fmt::Debug {
    /// Construct a Command that, when spawned, executes `command` inside `cwd`.
    fn build(&self, command: &str, cwd: &Path) -> tokio::process::Command;
    /// Stable identifier for telemetry (e.g., "macos-plain", "windows-powershell-user").
    fn describe(&self) -> &'static str;
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellMode {
    /// Minimal, deterministic — does NOT source user config files. Default.
    Plain,
    /// Sources user shell config (zsh .zshrc / PowerShell $PROFILE). Opt-in.
    User,
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub fn detect_mode() -> ShellMode {
    match std::env::var("CODER_SHELL_MODE").ok().as_deref() {
        Some("user") => ShellMode::User,
        _ => ShellMode::Plain,
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub fn default_strategy() -> Arc<dyn ShellStrategy> {
    let mode = detect_mode();
    #[cfg(target_os = "macos")]
    { Arc::new(MacOsShell::new(mode)) }
    #[cfg(target_os = "windows")]
    { Arc::new(WindowsShell::new(mode)) }
}
```

#### macOS strategy (`MacOsShell`)

- **Plain mode**: `Command::new("/bin/sh").arg("-c").arg(command)` — matches the legacy behavior so the plain path is a known-safe baseline.
- **User mode**: Detect user shell from `$SHELL` by basename.
  - `zsh` → `Command::new(shell_path).arg("-c").arg(format!(". \"$HOME/.zshrc\" 2>/dev/null; {}", command))`
  - `bash` → same pattern with `~/.bashrc`
  - `fish` → `Command::new(shell_path).arg("-c").arg(format!("source \"$HOME/.config/fish/config.fish\" 2>/dev/null; {}", command))`
  - Unknown / unset `$SHELL` → fall through to plain mode invocation (logged via observer).
- **Command string construction**: the user-mode branch concatenates `command` after a trailing semicolon inside single-quoted `-c` string. This is safe because the inner command is passed as a **single argv element** to the shell's `-c` flag; no outer shell does argv splitting. Quoting only matters for the rc-file path, which is controlled.
- **No use of `-i`**: interactive mode causes PS1 output, SIGINT handling changes, and process-group surprises. Dot-sourcing is the explicit alternative and is what the user asked for per the risk analysis.

#### Windows strategy (`WindowsShell`)

- **Plain mode**: `Command::new("powershell.exe").args(["-NoProfile", "-NonInteractive", "-Command", command])`
- **User mode**: `Command::new("powershell.exe").args(["-NonInteractive", "-Command", command])` — omits `-NoProfile` so `$PROFILE` loads.
- **Quoting**: PowerShell `-Command` with an inline string has fragile escaping rules for embedded quotes and `$`. To avoid an entire class of bugs, the implementation will use **`-EncodedCommand`** with UTF-16LE + base64 encoded command strings. This is the documented, escape-free path.
  - Pseudocode:
    ```rust
    let utf16: Vec<u16> = command.encode_utf16().collect();
    let bytes: Vec<u8> = utf16.iter().flat_map(|c| c.to_le_bytes()).collect();
    let encoded = base64_standard_encode(&bytes);
    Command::new("powershell.exe")
        .arg("-NoProfile")
        .arg("-NonInteractive")
        .arg("-EncodedCommand")
        .arg(encoded)
    ```
  - User mode uses the same encoding but drops `-NoProfile`.
- **Base64 helper**: Implement locally (tiny function) rather than adding a `base64` crate dependency. Keeps the Cargo graph unchanged.
- **Execution policy**: Intentionally **not** set to `Bypass`. If a restricted policy blocks `$PROFILE` in user mode, `-Command` still executes; the user simply does not see profile customizations. Document this as a known limitation rather than silently overriding user security policy.

#### `build_command` modification (`spawn.rs`)

```rust
fn build_command(&self, launch: &ResolvedLaunch, cwd: &Path) -> Result<Command, SpawnError> {
    let mut command = match launch.mode {
        SpawnMode::Program => {
            let program = launch.program.as_ref().ok_or(...)?;
            let mut c = Command::new(program);
            c.args(&launch.args);
            c
        }
        SpawnMode::Shell => {
            let shell_command = launch.command.as_ref().ok_or(...)?;

            #[cfg(target_os = "linux")]
            {
                // Linux intentionally does NOT honor CODER_SHELL_MODE and does NOT
                // source any user profile (.bashrc / .zshrc). This path is byte-identical
                // to the pre-fix behavior. Linux user-profile sourcing is an explicit
                // non-goal of this PR — see "Findings Deferred" above.
                let mut c = Command::new("sh");
                c.arg("-c").arg(shell_command);
                c
            }

            #[cfg(any(target_os = "macos", target_os = "windows"))]
            {
                crate::infrastructure::agent::runtime::shell::default_strategy()
                    .build(shell_command, cwd)
            }

            #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
            {
                let mut c = Command::new("sh");
                c.arg("-c").arg(shell_command);
                c
            }
        }
    };

    command
        .current_dir(cwd)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    Ok(command)
}
```

The Linux branch is character-for-character identical to today. The `not(any(...))` fallback preserves pre-existing behavior for any other Unix target (BSD, etc.) that may inherit via `cfg(unix)` elsewhere.

#### Telemetry

In the `SpawnStarted` trace payload (currently built at `spawn.rs:349`), add a `shellStrategy` string field. On Linux, emit the constant `"linux-sh"`. On mac/windows, emit `strategy.describe()`. This enables failure correlation without changing the event shape.

### Affected Code Paths

- `SpawnEngine::build_command` — only shell mode branch touched; program mode untouched.
- `SpawnEngine::execute` — trace emission gains one field; nothing else changes.
- `ExecuteCommandTool::execute` (in `infrastructure/tools/system/execute_command.rs`) — **unchanged**.
- `AgentServiceImpl::spawn_command` (in `application/agent/service/spawn.rs`) — **unchanged**; it continues to delegate to `SpawnEngine::shared()`.
- Integration tests in `execute_command_tests.rs` — existing Linux-oriented tests gated by `#[cfg(all(test, feature = "integration-test"))]` continue to run only on Linux/CI where shell commands work; new platform-specific blocks added.

No callers depend on specific argv shape of the spawned `Command` — output is captured via streaming readers, exit code via `child.wait()`. The `SpawnOutcome` struct shape stays identical.

### Scope gating — the "profile" requested

The user's request for a "profile to ensure we will run only when we build on mac and window" is translated to **Rust's built-in target-triple conditional compilation**. Concretely:

- All new code (trait, enums, strategies, factory) is attributed with `#[cfg(any(target_os = "macos", target_os = "windows"))]`.
- When running `cargo build` or `cargo test` on Linux, the new module is not compiled; the Linux branch in `build_command` uses the pre-existing hardcoded `sh -c` path.
- When running `cargo test` on macOS, only macOS tests compile and run (`#[cfg(target_os = "macos")]`).
- When running `cargo test` on Windows, only Windows tests compile and run (`#[cfg(target_os = "windows")]`).

No Cargo feature flag is introduced. Reasoning:
- Cargo `[profile.X]` controls compilation profiles (dev / release / test / bench) and is the wrong primitive for platform gating.
- A Cargo `[features]` entry (e.g., `shell-multiplatform`) would be redundant with `cfg(target_os)` and would add a second, easy-to-forget switch.
- `cfg(target_os)` is the idiomatic Rust mechanism; CI matrices naturally respect it.

The `CODER_SHELL_MODE=user` env var is the **runtime** opt-in for config sourcing — independent of the compile-time platform gate.

### Edge Cases

| Case | Handling |
|------|----------|
| `CODER_SHELL_MODE=user` set on Linux | **Ignored.** The env var is never read on Linux. `build_command` uses the unchanged `sh -c` path regardless of the env value. No warning is logged — this is not an error, it is the documented behavior. |
| `$SHELL` unset on macOS | User mode silently downgrades to plain; observer trace records the downgrade (`shellStrategy: "macos-plain-fallback"`). |
| `$SHELL` points to non-existent binary on macOS | `Command::spawn()` returns `io::Error`; propagated as `SpawnError::SpawnFailed` per existing path. Acceptable — surfaces the broken config to the user. |
| Unknown shell family on macOS (e.g., `nu`, `xonsh`) | Downgrade to plain with logged trace. Do not attempt to guess dot-source syntax. |
| `.zshrc` contains `cd ~/someplace` | Working directory is overridden post-sourcing by the explicit `command`. We do **not** add `cd "$CWD" &&` — `tokio::process::Command::current_dir` already sets cwd before the child starts, and `.zshrc` cd'ing would rarely override that at invocation-time in non-interactive mode. Document as a known behavior, not a bug. |
| `.zshrc` writes to stdout | Output is captured in the command's stdout and returned to the agent. This is the stdout-pollution risk documented earlier; user-mode is opt-in for this reason. |
| PowerShell `$PROFILE` missing | `-Command` still runs; no error. |
| PowerShell execution policy blocks `$PROFILE` | Same — command runs, profile skipped silently. Documented limitation. |
| Command string contains null bytes | UTF-16LE encoding handles null bytes cleanly in `-EncodedCommand`; no interpolation issues. |
| Command string > 32 KB | PowerShell `-EncodedCommand` has a command-line length limit (~32 KB on Windows). If exceeded, fall back to a temp `.ps1` file + `-File`. **Flag as follow-up** — first implementation returns `SpawnError::InvalidArguments` with a clear message if encoded length exceeds 32000 bytes. |
| Non-ASCII command text on Windows | UTF-16LE encoding is required; handled by the encoding step. |
| `$SHELL` is `/bin/sh` on macOS | Plain mode (no config file to source for a POSIX sh). |

### Test Plan

#### Existing tests that cover this area
- `src-tauri/src/infrastructure/tools/system/execute_command_tests.rs` — integration-gated tests that hit real `sh -c`. These run on Linux and the `integration-test` feature. They stay unchanged and will continue to pass on Linux because the Linux branch in `build_command` is byte-identical.
- `src-tauri/src/infrastructure/agent/runtime/spawn.rs` unit tests (`mod tests` at line ~851) — exercise the policy struct and a recording observer. They do not touch `build_command` directly; unchanged.

#### New tests

All new tests must compile and run **only** on their target platform.

**Module:** `src-tauri/src/infrastructure/agent/runtime/shell_tests.rs` (co-located module, included from `shell.rs` with `#[cfg(test)]`).

Pure unit tests that construct `Command` objects and inspect argv — no process spawn required:

| Test | Gate | Assertion |
|------|------|-----------|
| `macos_plain_uses_sh_dash_c` | `#[cfg(target_os = "macos")]` | argv is `["-c", "<cmd>"]`, program is `/bin/sh` |
| `macos_user_zsh_dot_sources_zshrc` | `#[cfg(target_os = "macos")]` | With `$SHELL=/bin/zsh` and `CODER_SHELL_MODE=user`, argv inner string contains `. "$HOME/.zshrc"` prefix and the user command suffix |
| `macos_user_bash_dot_sources_bashrc` | `#[cfg(target_os = "macos")]` | With `$SHELL=/bin/bash` and user mode, argv contains `. "$HOME/.bashrc"` |
| `macos_user_fish_sources_config_fish` | `#[cfg(target_os = "macos")]` | With `$SHELL=/usr/local/bin/fish` and user mode, argv uses `source` keyword |
| `macos_user_unknown_shell_falls_back_to_plain` | `#[cfg(target_os = "macos")]` | With `$SHELL=/opt/xonsh` (unknown family) and user mode, resolves to plain strategy |
| `macos_user_shell_unset_falls_back_to_plain` | `#[cfg(target_os = "macos")]` | With `$SHELL` removed from env, user mode resolves to plain |
| `windows_plain_uses_powershell_noprofile` | `#[cfg(target_os = "windows")]` | argv contains `-NoProfile`, `-NonInteractive`, `-EncodedCommand`, and a valid base64 blob |
| `windows_user_omits_noprofile` | `#[cfg(target_os = "windows")]` | argv contains `-NonInteractive`, `-EncodedCommand`, no `-NoProfile` |
| `windows_encoded_command_decodes_to_utf16le` | `#[cfg(target_os = "windows")]` | Decode the base64 argv element → UTF-16LE → original string matches input |
| `windows_encoded_command_length_limit` | `#[cfg(target_os = "windows")]` | Command > 32000 bytes after encoding returns error (or acceptable fallback — verify implementation matches plan) |
| `detect_mode_returns_plain_by_default` | `#[cfg(any(target_os = "macos", target_os = "windows"))]` | With `CODER_SHELL_MODE` unset → `ShellMode::Plain` |
| `detect_mode_returns_user_for_user_value` | `#[cfg(any(target_os = "macos", target_os = "windows"))]` | With `CODER_SHELL_MODE=user` → `ShellMode::User` |
| `detect_mode_unknown_value_returns_plain` | `#[cfg(any(target_os = "macos", target_os = "windows"))]` | With `CODER_SHELL_MODE=garbage` → `ShellMode::Plain` |
| `linux_ignores_coder_shell_mode` | `#[cfg(target_os = "linux")]` (added to `spawn.rs` tests module) | With `CODER_SHELL_MODE=user` set, `build_command` in shell mode produces argv `["-c", "<cmd>"]` and program `sh` — byte-identical to the env-unset case. Confirms Linux path ignores the env var. |

**Test isolation warning:** tests that manipulate `$SHELL` or `CODER_SHELL_MODE` must use a serial-test lock or avoid mutating process env (prefer a `detect_mode_with(env: &dyn EnvLookup)` seam injected in tests). Plan uses the injection seam — no `serial_test` dependency added.

**Integration tests** (gated by `#[cfg(all(test, feature = "integration-test", any(target_os = "macos", target_os = "windows")))]` in `execute_command_tests.rs`):

- `macos_user_mode_sees_zshrc_alias` — write a temp `$HOME/.zshrc` with `alias sentinel="echo CODER_OK"`, set `CODER_SHELL_MODE=user`, run `sentinel`, assert stdout contains `CODER_OK`. Requires temp HOME override.
- `windows_user_mode_sees_profile_variable` — write a temp `$PROFILE` that sets `$env:CODER_SENTINEL = "OK"`, set `CODER_SHELL_MODE=user`, run `echo $env:CODER_SENTINEL`, assert stdout contains `OK`.
- Plain-mode counterparts for each, asserting sentinel is **not** visible (confirms `-NoProfile` / lack of dot-sourcing works).

#### Regression risks to watch for

- Linux integration tests must still pass — `build_command` Linux branch is untouched, but verify with `cargo test --features integration-test` on Linux before merging.
- `SpawnOutcome` JSON shape — the `shellStrategy` trace field is additive, but downstream trace consumers should be surveyed. Run `grep -rn "SpawnStarted" src-tauri/` to confirm no destructuring assumes a fixed key set.
- The `detect_mode` env-var read happens at strategy construction. If `default_strategy()` is called per-command (it currently is), env var changes mid-session are honored — this is intentional but document it.

### Risk Assessment

- **Fix confidence:** High — root cause is unambiguous, the trait boundary is minimal, and the Linux path is preserved verbatim.
- **Blast radius:** Small — one new file, one modified function body, one new test file. No public API changes outside the new `shell` module. No DB, no UI, no dependency graph changes.
- **Breaking change:** No — Linux behavior byte-identical; Windows goes from broken to working; macOS plain mode byte-identical to current behavior (user mode is opt-in via env var).
- **Missing context:**
  - **Unverified on real macOS hardware**: the dot-source behavior of `zsh -c '. ~/.zshrc; <cmd>'` under non-interactive mode should be confirmed on a live macOS system. In particular, some `.zshrc` files check `$PS1` or `$-` and skip customizations when not interactive. If widely problematic, the fallback is to switch to `zsh -l -c` (login shell) — flagged but not adopted by default because login shells can change cwd via `.zprofile`.
  - **Unverified on real Windows hardware**: `-EncodedCommand` length limit (documented ~32 KB) should be empirically confirmed; the 32000-byte guard is conservative.
  - **Unverified with oh-my-zsh**: heavy zsh plugin managers may write to stdout during sourcing even in non-interactive mode. If this pollutes command output in practice, mitigation is to redirect sourcing stderr/stdout into `/dev/null` inside the dot-source wrapper — flagged as a potential follow-up, not included in the first implementation because it would mask legitimate errors in `.zshrc`.

### Out of Scope (explicit non-goals for this PR)

- **Linux user-profile sourcing** — Linux keeps `sh -c` unconditionally; `CODER_SHELL_MODE=user` is not honored on Linux; no `.bashrc` / `.zshrc` dot-source; no shell-family detection. (User-directed non-goal.)
- UI settings panel for shell preference.
- SQLite schema for per-workspace shell preference.
- `pwsh.exe` (PowerShell 7+) autodetection.
- Automatic shell-family detection for obscure shells (nu, xonsh, elvish) — all fall back to plain.
- Command-line-length fallback using a temp `.ps1` file on Windows.

---

## Implementation Sequencing

One PR, three commits:

1. **Add `shell.rs` and unit tests** — platform-gated trait, enum, factory, macOS + Windows strategies, unit tests. No changes to `spawn.rs` yet. Compiles on Linux (empty module under cfg), macOS, and Windows.
2. **Wire `build_command` delegation** — add the `cfg(target_os)` branch and the `shellStrategy` telemetry field. Linux branch unchanged.
3. **Add integration tests and docs** — platform-gated integration tests + `docs/features/shell-execution-strategy.md`.

Each commit compiles and tests pass on Linux (baseline preserved). Commits 1–3 additionally compile and tests pass on macOS and Windows when those targets are available in CI.
