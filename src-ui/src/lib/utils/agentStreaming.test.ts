import {describe, expect, it} from 'vitest';
import {resolvePersistedAgentContent} from './agentStreaming';

describe('resolvePersistedAgentContent', () => {
	it('prefers the streamed assistant content when available', () => {
		expect(
			resolvePersistedAgentContent(
				'Let me inspect the project for you.\n\nHere is what I found.',
				'Here is what I found.'
			)
		).toBe('Let me inspect the project for you.\n\nHere is what I found.');
	});

	it('falls back to the execution result content when streamed content is empty', () => {
		expect(resolvePersistedAgentContent('', 'Final answer')).toBe('Final answer');
		expect(resolvePersistedAgentContent(undefined, 'Final answer')).toBe('Final answer');
	});
});
