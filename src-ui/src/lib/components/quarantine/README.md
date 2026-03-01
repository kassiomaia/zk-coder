# Quarantine Folder

This folder contains components and files that were identified as unused during a codebase audit on 2026-03-01.

## Files in Quarantine

### Root Components (moved from src-ui/src/lib/components/)
- `ChatArea.svelte` + `ChatArea.module.css` - Duplicate of chat/ChatArea.svelte
- `ChatMessageList.svelte` + `ChatMessageList.module.css` - Not imported anywhere
- `CodeBlock.svelte` + `CodeBlock.module.css` - Not imported anywhere
- `InputBox.svelte` + `InputBox.module.css` - Not imported anywhere
- `ChatDemo.svelte` + `ChatDemo.module.css` - Not imported anywhere
- `Skeleton.svelte` + `Skeleton.module.css` - Moved from chat/ folder, not imported anywhere

## Reason for Quarantine

These files were exported from the components index but never actually imported or used anywhere in the application.

## Next Steps

1. Verify the application builds and runs correctly
2. Verify no runtime errors occur
3. After confirmation, delete this quarantine folder

## IMPORTANT

**Do not reference any files in this folder from other project files.**
This folder is meant to be isolated and eventually deleted.
