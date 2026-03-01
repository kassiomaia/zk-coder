# Prototype Location Rule

This is a MANDATORY rule. No exceptions.

## Rule

All prototypes MUST be located at:

```
./prototypes/<prototype-name>/
```

From the project root directory.

## Valid Paths

| Path | Status |
|------|--------|
| `./prototypes/config-flow/` | ✅ Valid |
| `./prototypes/thread-panel/` | ✅ Valid |
| `./prototypes/settings-modal/` | ✅ Valid |

## Invalid Paths

| Path | Status |
|------|--------|
| `./docs/prototypes/config-flow/` | ❌ Invalid |
| `./src-ui/prototypes/config-flow/` | ❌ Invalid |
| `~/prototypes/config-flow/` | ❌ Invalid |
| `/tmp/prototypes/config-flow/` | ❌ Invalid |
| `./config-flow/` | ❌ Invalid |

## Enforcement

When a user references a prototype:

1. **Check the path** - Verify it starts with `./prototypes/`
2. **If valid** - Proceed with analysis
3. **If invalid** - STOP and respond with:

```
⚠️ Invalid prototype location.

Prototypes must be located at: ./prototypes/<prototype-name>/

Please ensure the prototype is in the correct location before proceeding.
```

## Rationale

- **Consistency:** All team members know where to find prototypes
- **Tooling:** Build scripts and CI can assume this location
- **Isolation:** Keeps prototype code separate from production code
- **Documentation:** Easy to reference in docs and issues

## No Exceptions

This rule has no exceptions. Even if:
- The prototype is "just for testing"
- The prototype is "temporary"
- The user says "it's fine"
- The prototype is in a "similar" location

The prototype MUST be in `./prototypes/` for this skill to analyze it.
