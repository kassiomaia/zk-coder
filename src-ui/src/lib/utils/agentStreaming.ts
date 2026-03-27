export function resolvePersistedAgentContent(
	streamedContent: string | undefined,
	executionContent: string
): string {
	if (streamedContent && streamedContent.trim().length > 0) {
		return streamedContent;
	}

	return executionContent;
}
