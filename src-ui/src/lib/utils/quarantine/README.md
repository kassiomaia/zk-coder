# Quarantine Folder

This folder contains utility files that were identified as unused during a codebase audit on 2026-03-01.

## Files in Quarantine

### Utils (moved from src-ui/src/lib/utils/)
- `formatTimestamp.ts` - Only used in its own test file, not used in the application
- `formatTimestamp.test.ts` - Test file for the unused utility

## Reason for Quarantine

These utilities were defined but never imported or used anywhere in the application.

## Next Steps

1. Verify the application builds and runs correctly
2. Verify no runtime errors occur
3. After confirmation, delete this quarantine folder

## IMPORTANT

**Do not reference any files in this folder from other project files.**
This folder is meant to be isolated and eventually deleted.
