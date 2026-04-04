# Scripts Quality Guidelines

**Status**: ACTIVE
**Last reviewed**: 2026-04-04
**Purpose**: Mandatory rules for repository automation scripts and command-line workflow helpers.

## Scope

This guide applies to:

- root-level automation scripts such as `auto-improve.sh`
- scripts under `scripts/`
- future shell-based repository automation

## Repository-specific shell stance

The current repository scripts are Bash-based. Preserve that unless a repository-wide migration is intentionally approved.

Rules:

- use one shell dialect per script
- include a clear shebang
- write scripts so they can be invoked reliably from the repository root or from their own path resolution logic
- do not mix shell automation with product logic that belongs in Rust or TypeScript

## Safety rules

- fail fast by default (`set -euo pipefail` unless a specific script has a documented reason not to)
- quote variable expansions
- avoid `eval`
- avoid parsing command output with brittle ad-hoc text processing when structured alternatives exist
- treat destructive, privileged, or network-expensive behavior as opt-in whenever possible
- if a script can loop indefinitely, it must make that behavior obvious and support clean shutdown

For existing legacy scripts that do not yet meet all of these rules, any touched portion must move toward them rather than away from them.

## Clean-code rules for scripts

- keep functions small and named by behavior
- centralize configuration at the top of the script
- prefer arrays over string-concatenated command construction
- resolve project paths explicitly
- separate argument parsing, validation, execution, and reporting
- print actionable errors
- document non-obvious behavior in comments close to the implementation

## Remove unimportant references

Do not keep:

- stale flags
- dead retry branches
- commented-out commands
- unused environment variables
- logs that claim behavior the script no longer performs
- copy-pasted command blocks that should be shared through a function

## Logging and output rules

- keep logs concise and useful for humans scanning a terminal
- include timestamps for long-running automation
- do not print secrets or sensitive file contents
- do not rely on interactive behavior unless the script is explicitly designed for interactive use
- provide `--help` or equivalent usage guidance for non-trivial scripts

## Portability and dependency rules

- assume contributors may invoke scripts from different shells, but the script itself must declare the interpreter it needs
- avoid non-portable shell features unless the shebang and documentation make the requirement explicit
- prefer standard tooling already expected in the repo; if a new external dependency is required, document it clearly
- prefer absolute or repo-root-derived paths over fragile relative assumptions

## Validation rules

Before finalizing script changes:

- run the script path you changed in a safe way when possible
- at minimum, run syntax validation such as `bash -n <script>`
- prefer `shellcheck` when available, but do not assume it is installed unless verified
- test help output, argument parsing, and failure paths for non-trivial scripts

## Refactor process for scripts

1. identify repeated command flow or unsafe behavior
2. add or preserve a safe way to verify behavior
3. extract common functions and central config
4. simplify control flow and quoting
5. remove the superseded path immediately
6. rerun syntax and behavioral validation

## Scripts review checklist

Before considering a script change complete, verify:

- the interpreter and shell dialect are clear
- failure behavior is safe and predictable
- variables and paths are quoted and explicit
- duplicate command logic was reduced
- stale flags, branches, and logs were removed
- sensitive data is not emitted
- syntax and runtime behavior were validated

Any exception must be explicit and justified.
