//! Platform-specific shell strategies for command execution.
//!
//! This module exists only on macOS and Windows. On Linux, `SpawnEngine::build_command`
//! continues to use `Command::new("sh").arg("-c")` directly — Linux user-profile sourcing
//! is an explicit non-goal of the current work (see `docs/code-review/FIX_PLAN.md`).
//!
//! The `ShellStrategy` trait is the seam through which platform-specific shell invocation
//! is expressed. Two strategies exist per platform: plain (minimal, deterministic) and
//! user (sources user shell configuration — `.zshrc` on macOS, `$PROFILE` on Windows).
//!
//! User mode is opt-in via the `CODER_SHELL_MODE=user` environment variable. Any other
//! value (or an unset variable) resolves to plain mode.

#![cfg(any(target_os = "macos", target_os = "windows"))]

use std::path::Path;
use std::sync::Arc;

use tokio::process::Command;

/// A platform-specific strategy for constructing shell commands.
pub trait ShellStrategy: Send + Sync + std::fmt::Debug {
    /// Build a `Command` that, when spawned, executes `command` inside `cwd`.
    ///
    /// The caller is responsible for configuring stdio and working directory on
    /// the returned `Command` before spawning. This method only decides which
    /// executable and arguments are used.
    fn build(&self, command: &str, cwd: &Path) -> Command;

    /// Stable identifier for telemetry, e.g. `"macos-plain"`, `"windows-powershell-user"`.
    fn describe(&self) -> &'static str;
}

/// Whether the strategy should source the user's shell configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellMode {
    /// Minimal, deterministic: do not source user configuration files. Default.
    Plain,
    /// Source user configuration files (opt-in via `CODER_SHELL_MODE=user`).
    User,
}

/// Abstraction over environment-variable lookups to keep strategy construction testable
/// without mutating process-wide environment state.
pub trait EnvLookup {
    /// Return the value of `key` from the environment, or `None` if unset.
    fn get(&self, key: &str) -> Option<String>;
}

/// Default implementation that reads the current process environment.
#[derive(Debug, Default)]
pub struct ProcessEnv;

impl EnvLookup for ProcessEnv {
    fn get(&self, key: &str) -> Option<String> {
        std::env::var(key).ok()
    }
}

/// Resolve `ShellMode` from an `EnvLookup`. Any value other than `"user"` — including
/// unset — yields `ShellMode::Plain`.
pub fn detect_mode_with(env: &dyn EnvLookup) -> ShellMode {
    match env.get("CODER_SHELL_MODE").as_deref() {
        Some("user") => ShellMode::User,
        _ => ShellMode::Plain,
    }
}

/// Resolve `ShellMode` from the process environment.
pub fn detect_mode() -> ShellMode {
    detect_mode_with(&ProcessEnv)
}

/// Construct the default strategy for the current platform using the process environment.
#[cfg(target_os = "macos")]
pub fn default_strategy() -> Arc<dyn ShellStrategy> {
    Arc::new(MacOsShell::from_env(&ProcessEnv))
}

/// Construct the default strategy for the current platform using the process environment.
#[cfg(target_os = "windows")]
pub fn default_strategy() -> Arc<dyn ShellStrategy> {
    Arc::new(WindowsShell::new(detect_mode()))
}

/// Return a stable label for the current default strategy without constructing the
/// strategy object. Useful for telemetry that fires before `build_command`.
pub fn current_label() -> &'static str {
    default_strategy().describe()
}

// =============================================================================
// macOS
// =============================================================================

/// Recognized shell families on macOS. Used to choose the correct rc file and
/// sourcing syntax. Unknown shells fall back to plain `/bin/sh -c` invocation.
#[cfg(target_os = "macos")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellFamily {
    /// zsh — the macOS default since Catalina. Sources `~/.zshrc`.
    Zsh,
    /// bash. Sources `~/.bashrc`.
    Bash,
    /// fish. Uses `source` (not dot-source) with `~/.config/fish/config.fish`.
    Fish,
    /// Shell family could not be determined from `$SHELL`; treat as plain.
    Unknown,
}

#[cfg(target_os = "macos")]
impl ShellFamily {
    /// Classify a shell binary path (e.g. `/bin/zsh`) by its basename.
    pub fn from_path(path: &str) -> Self {
        let basename = std::path::Path::new(path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        match basename {
            "zsh" => Self::Zsh,
            "bash" => Self::Bash,
            "fish" => Self::Fish,
            _ => Self::Unknown,
        }
    }
}

/// macOS shell strategy. In plain mode, invokes `/bin/sh -c <cmd>` (byte-identical
/// to the historical Linux path). In user mode, invokes the user's preferred shell
/// with an explicit dot-source of their rc file before the command runs.
#[cfg(target_os = "macos")]
#[derive(Clone, Debug)]
pub struct MacOsShell {
    mode: ShellMode,
    shell_path: Option<String>,
    shell_family: ShellFamily,
}

#[cfg(target_os = "macos")]
impl MacOsShell {
    /// Construct a strategy from an `EnvLookup`. Reads `$SHELL` (for family detection)
    /// and `CODER_SHELL_MODE` (for mode selection).
    pub fn from_env(env: &dyn EnvLookup) -> Self {
        let mode = detect_mode_with(env);
        let shell_path = env.get("SHELL");
        let shell_family = shell_path
            .as_deref()
            .map(ShellFamily::from_path)
            .unwrap_or(ShellFamily::Unknown);
        Self {
            mode,
            shell_path,
            shell_family,
        }
    }

    /// Test constructor: inject mode, shell path, and family directly.
    #[cfg(test)]
    pub fn for_test(
        mode: ShellMode,
        shell_path: Option<String>,
        shell_family: ShellFamily,
    ) -> Self {
        Self {
            mode,
            shell_path,
            shell_family,
        }
    }

    fn effective_mode(&self) -> ShellMode {
        match (self.mode, self.shell_family) {
            (ShellMode::User, ShellFamily::Unknown) => ShellMode::Plain,
            (mode, _) => mode,
        }
    }

    fn build_plain(command: &str) -> Command {
        let mut c = Command::new("/bin/sh");
        c.arg("-c").arg(command);
        c
    }

    fn build_user(&self, command: &str) -> Command {
        // `shell_path` is guaranteed to be Some when effective_mode() is User because
        // `ShellFamily::Unknown` is the only classification that can arise with a None
        // path, and that case downgrades to plain in effective_mode().
        let shell_path = self
            .shell_path
            .as_deref()
            .unwrap_or("/bin/sh");
        let wrapped = match self.shell_family {
            ShellFamily::Zsh => {
                format!(r#". "$HOME/.zshrc" 2>/dev/null; {command}"#)
            }
            ShellFamily::Bash => {
                format!(r#". "$HOME/.bashrc" 2>/dev/null; {command}"#)
            }
            ShellFamily::Fish => {
                format!(
                    r#"source "$HOME/.config/fish/config.fish" 2>/dev/null; {command}"#
                )
            }
            ShellFamily::Unknown => command.to_string(),
        };
        let mut c = Command::new(shell_path);
        c.arg("-c").arg(wrapped);
        c
    }
}

#[cfg(target_os = "macos")]
impl ShellStrategy for MacOsShell {
    fn build(&self, command: &str, _cwd: &Path) -> Command {
        match self.effective_mode() {
            ShellMode::Plain => Self::build_plain(command),
            ShellMode::User => self.build_user(command),
        }
    }

    fn describe(&self) -> &'static str {
        match (self.mode, self.effective_mode()) {
            (ShellMode::Plain, _) => "macos-plain",
            (ShellMode::User, ShellMode::User) => match self.shell_family {
                ShellFamily::Zsh => "macos-user-zsh",
                ShellFamily::Bash => "macos-user-bash",
                ShellFamily::Fish => "macos-user-fish",
                ShellFamily::Unknown => "macos-plain-fallback",
            },
            (ShellMode::User, ShellMode::Plain) => "macos-plain-fallback",
        }
    }
}

// =============================================================================
// Windows
// =============================================================================

/// Maximum UTF-16LE byte length we will pass to `-EncodedCommand`. PowerShell's
/// documented command-line limit is ~32 KB; we leave headroom for the surrounding
/// argv elements and process-creation overhead.
#[cfg(target_os = "windows")]
pub const WINDOWS_ENCODED_COMMAND_MAX_BYTES: usize = 30_000;

/// Windows shell strategy. In plain mode, invokes PowerShell with `-NoProfile` (no
/// user configuration loaded). In user mode, omits `-NoProfile` so `$PROFILE` is
/// sourced. Commands are always passed via `-EncodedCommand` (base64 UTF-16LE) to
/// avoid PowerShell's inline-quoting pitfalls.
#[cfg(target_os = "windows")]
#[derive(Clone, Debug)]
pub struct WindowsShell {
    mode: ShellMode,
}

#[cfg(target_os = "windows")]
impl WindowsShell {
    /// Construct a strategy for the given mode.
    pub fn new(mode: ShellMode) -> Self {
        Self { mode }
    }
}

#[cfg(target_os = "windows")]
impl ShellStrategy for WindowsShell {
    fn build(&self, command: &str, _cwd: &Path) -> Command {
        let encoded = encode_powershell_command(command);
        let mut c = Command::new("powershell.exe");
        if matches!(self.mode, ShellMode::Plain) {
            c.arg("-NoProfile");
        }
        c.arg("-NonInteractive")
            .arg("-EncodedCommand")
            .arg(encoded);
        c
    }

    fn describe(&self) -> &'static str {
        match self.mode {
            ShellMode::Plain => "windows-powershell-plain",
            ShellMode::User => "windows-powershell-user",
        }
    }
}

/// Encode a command string as base64 of its UTF-16LE byte representation, as expected
/// by PowerShell's `-EncodedCommand` argument.
#[cfg(target_os = "windows")]
pub fn encode_powershell_command(command: &str) -> String {
    let utf16: Vec<u16> = command.encode_utf16().collect();
    let mut bytes = Vec::with_capacity(utf16.len() * 2);
    for unit in utf16 {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    base64_encode(&bytes)
}

/// Minimal, dependency-free base64 encoder. PowerShell's decoder accepts the standard
/// alphabet; output is always padded to a multiple of 4 characters with `=`.
#[cfg(target_os = "windows")]
pub fn base64_encode(input: &[u8]) -> String {
    const ALPHABET: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(input.len().div_ceil(3) * 4);
    let mut iter = input.chunks_exact(3);
    for chunk in iter.by_ref() {
        let b = [chunk[0], chunk[1], chunk[2]];
        output.push(ALPHABET[(b[0] >> 2) as usize] as char);
        output.push(ALPHABET[(((b[0] & 0b11) << 4) | (b[1] >> 4)) as usize] as char);
        output.push(ALPHABET[(((b[1] & 0b1111) << 2) | (b[2] >> 6)) as usize] as char);
        output.push(ALPHABET[(b[2] & 0b111111) as usize] as char);
    }
    let rem = iter.remainder();
    match rem.len() {
        0 => {}
        1 => {
            let b0 = rem[0];
            output.push(ALPHABET[(b0 >> 2) as usize] as char);
            output.push(ALPHABET[((b0 & 0b11) << 4) as usize] as char);
            output.push('=');
            output.push('=');
        }
        2 => {
            let b0 = rem[0];
            let b1 = rem[1];
            output.push(ALPHABET[(b0 >> 2) as usize] as char);
            output.push(ALPHABET[(((b0 & 0b11) << 4) | (b1 >> 4)) as usize] as char);
            output.push(ALPHABET[((b1 & 0b1111) << 2) as usize] as char);
            output.push('=');
        }
        _ => unreachable!("chunks_exact(3) remainder has at most 2 elements"),
    }
    output
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[derive(Default)]
    struct MapEnv(HashMap<String, String>);

    impl MapEnv {
        fn with(pairs: &[(&str, &str)]) -> Self {
            let mut map = HashMap::new();
            for (k, v) in pairs {
                map.insert((*k).to_string(), (*v).to_string());
            }
            Self(map)
        }
    }

    impl EnvLookup for MapEnv {
        fn get(&self, key: &str) -> Option<String> {
            self.0.get(key).cloned()
        }
    }

    fn args_as_strings(cmd: &Command) -> Vec<String> {
        cmd.as_std()
            .get_args()
            .map(|os| os.to_string_lossy().into_owned())
            .collect()
    }

    fn program_as_string(cmd: &Command) -> String {
        cmd.as_std().get_program().to_string_lossy().into_owned()
    }

    #[test]
    fn detect_mode_returns_plain_by_default() {
        let env = MapEnv::default();
        assert_eq!(detect_mode_with(&env), ShellMode::Plain);
    }

    #[test]
    fn detect_mode_returns_user_for_user_value() {
        let env = MapEnv::with(&[("CODER_SHELL_MODE", "user")]);
        assert_eq!(detect_mode_with(&env), ShellMode::User);
    }

    #[test]
    fn detect_mode_unknown_value_returns_plain() {
        let env = MapEnv::with(&[("CODER_SHELL_MODE", "garbage")]);
        assert_eq!(detect_mode_with(&env), ShellMode::Plain);
    }

    // =========================================================================
    // macOS-specific tests
    // =========================================================================

    #[cfg(target_os = "macos")]
    mod macos {
        use super::*;

        #[test]
        fn plain_uses_sh_dash_c() {
            let shell = MacOsShell::for_test(ShellMode::Plain, None, ShellFamily::Unknown);
            let cmd = shell.build("echo hi", &PathBuf::from("/tmp"));
            assert_eq!(program_as_string(&cmd), "/bin/sh");
            assert_eq!(args_as_strings(&cmd), vec!["-c", "echo hi"]);
            assert_eq!(shell.describe(), "macos-plain");
        }

        #[test]
        fn user_zsh_dot_sources_zshrc() {
            let shell = MacOsShell::for_test(
                ShellMode::User,
                Some("/bin/zsh".to_string()),
                ShellFamily::Zsh,
            );
            let cmd = shell.build("echo hi", &PathBuf::from("/tmp"));
            assert_eq!(program_as_string(&cmd), "/bin/zsh");
            let args = args_as_strings(&cmd);
            assert_eq!(args[0], "-c");
            assert!(
                args[1].contains(r#". "$HOME/.zshrc""#),
                "expected zshrc dot-source, got: {}",
                args[1]
            );
            assert!(args[1].ends_with("echo hi"));
            assert_eq!(shell.describe(), "macos-user-zsh");
        }

        #[test]
        fn user_bash_dot_sources_bashrc() {
            let shell = MacOsShell::for_test(
                ShellMode::User,
                Some("/bin/bash".to_string()),
                ShellFamily::Bash,
            );
            let cmd = shell.build("echo hi", &PathBuf::from("/tmp"));
            assert_eq!(program_as_string(&cmd), "/bin/bash");
            let args = args_as_strings(&cmd);
            assert!(args[1].contains(r#". "$HOME/.bashrc""#));
            assert_eq!(shell.describe(), "macos-user-bash");
        }

        #[test]
        fn user_fish_sources_config_fish() {
            let shell = MacOsShell::for_test(
                ShellMode::User,
                Some("/usr/local/bin/fish".to_string()),
                ShellFamily::Fish,
            );
            let cmd = shell.build("echo hi", &PathBuf::from("/tmp"));
            assert_eq!(program_as_string(&cmd), "/usr/local/bin/fish");
            let args = args_as_strings(&cmd);
            assert!(args[1].contains(r#"source "$HOME/.config/fish/config.fish""#));
            assert_eq!(shell.describe(), "macos-user-fish");
        }

        #[test]
        fn user_unknown_shell_falls_back_to_plain() {
            let shell = MacOsShell::for_test(
                ShellMode::User,
                Some("/opt/xonsh".to_string()),
                ShellFamily::Unknown,
            );
            let cmd = shell.build("echo hi", &PathBuf::from("/tmp"));
            // Effective mode downgrades to plain; uses /bin/sh
            assert_eq!(program_as_string(&cmd), "/bin/sh");
            assert_eq!(args_as_strings(&cmd), vec!["-c", "echo hi"]);
            assert_eq!(shell.describe(), "macos-plain-fallback");
        }

        #[test]
        fn user_shell_unset_falls_back_to_plain() {
            let env = MapEnv::with(&[("CODER_SHELL_MODE", "user")]);
            let shell = MacOsShell::from_env(&env);
            let cmd = shell.build("echo hi", &PathBuf::from("/tmp"));
            assert_eq!(program_as_string(&cmd), "/bin/sh");
            assert_eq!(shell.describe(), "macos-plain-fallback");
        }

        #[test]
        fn family_classification_from_path() {
            assert_eq!(ShellFamily::from_path("/bin/zsh"), ShellFamily::Zsh);
            assert_eq!(
                ShellFamily::from_path("/usr/local/bin/bash"),
                ShellFamily::Bash
            );
            assert_eq!(ShellFamily::from_path("/opt/fish"), ShellFamily::Fish);
            assert_eq!(ShellFamily::from_path("/opt/xonsh"), ShellFamily::Unknown);
            assert_eq!(ShellFamily::from_path(""), ShellFamily::Unknown);
        }
    }

    // =========================================================================
    // Windows-specific tests
    // =========================================================================

    #[cfg(target_os = "windows")]
    mod windows {
        use super::*;

        fn base64_decode(input: &str) -> Vec<u8> {
            const ALPHABET: &[u8; 64] =
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
            let mut lookup = [0u8; 256];
            for (i, byte) in ALPHABET.iter().enumerate() {
                lookup[*byte as usize] = i as u8;
            }
            let bytes: Vec<u8> = input.bytes().filter(|b| *b != b'=').collect();
            let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
            for chunk in bytes.chunks(4) {
                let v0 = lookup[chunk[0] as usize] as u32;
                let v1 = if chunk.len() > 1 { lookup[chunk[1] as usize] as u32 } else { 0 };
                let v2 = if chunk.len() > 2 { lookup[chunk[2] as usize] as u32 } else { 0 };
                let v3 = if chunk.len() > 3 { lookup[chunk[3] as usize] as u32 } else { 0 };
                let combined = (v0 << 18) | (v1 << 12) | (v2 << 6) | v3;
                out.push(((combined >> 16) & 0xFF) as u8);
                if chunk.len() > 2 {
                    out.push(((combined >> 8) & 0xFF) as u8);
                }
                if chunk.len() > 3 {
                    out.push((combined & 0xFF) as u8);
                }
            }
            out
        }

        fn decode_utf16le(bytes: &[u8]) -> String {
            let units: Vec<u16> = bytes
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            String::from_utf16_lossy(&units)
        }

        #[test]
        fn plain_uses_powershell_noprofile() {
            let shell = WindowsShell::new(ShellMode::Plain);
            let cmd = shell.build("echo hi", &PathBuf::from("C:\\"));
            assert_eq!(program_as_string(&cmd), "powershell.exe");
            let args = args_as_strings(&cmd);
            assert_eq!(args[0], "-NoProfile");
            assert_eq!(args[1], "-NonInteractive");
            assert_eq!(args[2], "-EncodedCommand");
            assert!(!args[3].is_empty());
            assert_eq!(shell.describe(), "windows-powershell-plain");
        }

        #[test]
        fn user_omits_noprofile() {
            let shell = WindowsShell::new(ShellMode::User);
            let cmd = shell.build("echo hi", &PathBuf::from("C:\\"));
            let args = args_as_strings(&cmd);
            assert_eq!(args[0], "-NonInteractive");
            assert_eq!(args[1], "-EncodedCommand");
            assert!(!args.contains(&"-NoProfile".to_string()));
            assert_eq!(shell.describe(), "windows-powershell-user");
        }

        #[test]
        fn encoded_command_decodes_to_utf16le_roundtrip() {
            let original = "Write-Host 'hello — world'";
            let encoded = encode_powershell_command(original);
            let decoded = decode_utf16le(&base64_decode(&encoded));
            assert_eq!(decoded, original);
        }

        #[test]
        fn encoded_command_handles_ascii() {
            let encoded = encode_powershell_command("echo hi");
            let decoded = decode_utf16le(&base64_decode(&encoded));
            assert_eq!(decoded, "echo hi");
        }

        #[test]
        fn encoded_command_handles_empty() {
            let encoded = encode_powershell_command("");
            assert_eq!(encoded, "");
        }

        #[test]
        fn base64_encode_padding() {
            assert_eq!(base64_encode(b""), "");
            assert_eq!(base64_encode(b"f"), "Zg==");
            assert_eq!(base64_encode(b"fo"), "Zm8=");
            assert_eq!(base64_encode(b"foo"), "Zm9v");
            assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
            assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
            assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
        }
    }
}
