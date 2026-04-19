# Unsafe Audit

> The workspace crate is configured with `[lints.rust] unsafe_code = "forbid"`
> (see `src-tauri/Cargo.toml`). No `unsafe` blocks exist in the changed code.

## Sites

None. The shell strategy implementation and its tests use only safe Rust:
- Byte manipulation for base64 encoding / UTF-16LE conversion uses standard
  safe slice indexing and arithmetic.
- Process spawning goes through `tokio::process::Command`, which is a safe
  wrapper over platform primitives.

No audit actions required.
