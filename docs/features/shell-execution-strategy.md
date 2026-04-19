# Shell Execution Strategy

**Status**: ACTIVE
**Last reviewed**: 2026-04-19
**Purpose**: Document how the agent's `execute_command` tool selects a shell per platform, how user shell configuration is optionally sourced, and what the known limitations are.

## Summary

When the agent's `execute_command` tool runs a shell-mode command (`SpawnInvocation::Shell`), the actual shell process that is launched depends on the host operating system and on the `CODER_SHELL_MODE` environment variable.

| Platform | Default (Plain) mode | User mode (`CODER_SHELL_MODE=user`) |
|----------|----------------------|-------------------------------------|
| Linux    | `sh -c <cmd>`        | **Same as plain** — `CODER_SHELL_MODE` is ignored on Linux. |
| macOS    | `/bin/sh -c <cmd>`   | User's `$SHELL` with dot-sourced rc file (`.zshrc` / `.bashrc` / fish config). |
| Windows  | `powershell.exe -NoProfile -NonInteractive -EncodedCommand <b64>` | `powershell.exe -NonInteractive -EncodedCommand <b64>` (loads `$PROFILE`) |

## Why the distinction exists

The agent runs spawned processes with the parent process's environment. On Linux and macOS this means the default `sh -c` invocation is a non-interactive, non-login shell, so none of the user's shell configuration (aliases, shell functions, `PATH` modifications made in `.bashrc` / `.zshrc`, version managers like nvm/pyenv/asdf) is visible to the command. On Windows, `sh` does not exist at all, so a shell-mode spawn fails at process creation.

The `ShellStrategy` abstraction (in `src-tauri/src/infrastructure/agent/runtime/shell.rs`) exists to:
1. Fix Windows — use PowerShell instead of the nonexistent `sh`.
2. Offer opt-in sourcing of user shell configuration on macOS and Windows, so commands can resolve user-customized binaries and aliases when desired.
3. Keep plain mode as a safe, deterministic baseline that does not depend on user configuration.

Linux is intentionally excluded from the user-sourcing feature. It remains on the legacy `sh -c` path — no strategy abstraction is used on Linux, and `CODER_SHELL_MODE` is never read.

## Opt-in: `CODER_SHELL_MODE=user`

To enable user-configuration sourcing on macOS or Windows, set the environment variable before launching the Coder app:

```sh
# macOS
export CODER_SHELL_MODE=user
open -a Coder
```

```powershell
# Windows
$env:CODER_SHELL_MODE = "user"
& "C:\Path\To\Coder.exe"
```

Any other value (including unset, empty, or garbage) resolves to plain mode. The variable is read once when the spawn engine constructs its strategy; changes while the app is running are picked up per-command because the strategy is resolved per invocation.

## macOS behavior in user mode

The strategy detects the user's shell by inspecting `$SHELL`. The following shell families are recognized:

| `$SHELL` basename | Invocation |
|-------------------|------------|
| `zsh`             | `<shell> -c '. "$HOME/.zshrc" 2>/dev/null; <cmd>'` |
| `bash`            | `<shell> -c '. "$HOME/.bashrc" 2>/dev/null; <cmd>'` |
| `fish`            | `<shell> -c 'source "$HOME/.config/fish/config.fish" 2>/dev/null; <cmd>'` |
| any other, or `$SHELL` unset | Falls back to plain (`/bin/sh -c <cmd>`) |

The dot-source pattern is used deliberately instead of the `-i` (interactive) flag. Interactive mode can emit PS1 prompts to stdout, change SIGINT handling in ways that affect cancellation, and alter process-group semantics. Dot-sourcing avoids all of these while still loading user configuration.

## Windows behavior

The strategy always uses `powershell.exe` with `-EncodedCommand` (base64 of UTF-16LE bytes). This is done to sidestep PowerShell's fragile inline-quoting rules for `-Command`.

- In **plain mode**, `-NoProfile` is passed to prevent `$PROFILE` from loading.
- In **user mode**, `-NoProfile` is omitted, so `$PROFILE` is sourced by PowerShell before the command runs.

PowerShell 7+ (`pwsh.exe`) is not auto-detected in this release; only `powershell.exe` (Windows PowerShell 5.1) is used.

## Known limitations

- **Linux user-profile sourcing is not supported.** This is an explicit non-goal of the current release. Linux keeps `sh -c` unconditionally. See `docs/code-review/FIX_PLAN.md` for the deferral rationale.
- **PowerShell execution policy.** If the machine's policy restricts profile loading, `$PROFILE` will be skipped silently even in user mode; the command itself still runs. Coder does **not** pass `-ExecutionPolicy Bypass` — that decision belongs to the user and their organization.
- **Stdout pollution in user mode.** A `.zshrc` or `$PROFILE` that prints to stdout during sourcing will pollute the captured command output. This is why user mode is opt-in.
- **Shell-config cost per command.** Heavy configurations (oh-my-zsh, plugin managers, nvm, pyenv, asdf) add measurable latency to every invocation in user mode.
- **Unknown shell families on macOS.** `nu`, `xonsh`, `elvish`, and other non-POSIX shells are not classified; the strategy downgrades to plain mode rather than guessing rc-file syntax.
- **PowerShell command length.** The `-EncodedCommand` argument has a practical limit near 30 KB. Commands exceeding this are currently accepted but may fail at process creation; a future improvement would fall back to a temp `.ps1` file.

## Telemetry

Each `SpawnStarted` runtime trace includes a `shellStrategy` field with one of these values:

- `linux-sh`
- `macos-plain`, `macos-user-zsh`, `macos-user-bash`, `macos-user-fish`, `macos-plain-fallback`
- `windows-powershell-plain`, `windows-powershell-user`

Use this field when diagnosing command failures to confirm which strategy was in effect.

## Related files

- `src-tauri/src/infrastructure/agent/runtime/shell.rs` — strategy trait and platform implementations (macOS + Windows only; not compiled on Linux)
- `src-tauri/src/infrastructure/agent/runtime/spawn.rs` — `build_shell_command` delegation and `shell_strategy_label` telemetry helper
- `docs/code-review/FIX_PLAN.md` — architectural plan and scope rationale
