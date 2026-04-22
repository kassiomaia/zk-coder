import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createAgentEventBus } from './eventBus';

describe('agentEventBus', () => {
	let bus: ReturnType<typeof createAgentEventBus>;

	beforeEach(() => {
		bus = createAgentEventBus();
	});

	it('delivers emitted payloads to every registered handler', () => {
		const first = vi.fn();
		const second = vi.fn();
		bus.on('cleanup-transients', first);
		bus.on('cleanup-transients', second);

		bus.emit('cleanup-transients', {
			conversationId: 'c',
			threadId: 't',
			sessionId: 's'
		});

		const payload = { conversationId: 'c', threadId: 't', sessionId: 's' };
		expect(first).toHaveBeenCalledWith(payload);
		expect(second).toHaveBeenCalledWith(payload);
	});

	it('returns an unsubscribe function that removes just that handler', () => {
		const first = vi.fn();
		const second = vi.fn();
		const unsubFirst = bus.on('agent-activity', first);
		bus.on('agent-activity', second);

		unsubFirst();

		bus.emit('agent-activity', {
			conversationId: 'c',
			threadId: 't',
			message: { id: 'x' }
		});

		expect(first).not.toHaveBeenCalled();
		expect(second).toHaveBeenCalledTimes(1);
	});

	it('is a no-op to emit an event with no subscribers', () => {
		expect(() =>
			bus.emit('system-message', {
				sessionId: 's',
				conversationId: 'c',
				threadId: 't',
				message: { role: 'system', content: '' }
			})
		).not.toThrow();
	});

	it('isolates failures in one handler so later handlers still run', () => {
		const boom = vi.fn(() => {
			throw new Error('boom');
		});
		const survivor = vi.fn();

		bus.on('persist-assistant-message', boom);
		bus.on('persist-assistant-message', survivor);

		bus.emit('persist-assistant-message', {
			sessionId: 's',
			conversationId: 'c',
			threadId: 't',
			messageId: 'm',
			content: 'x'
		});

		expect(boom).toHaveBeenCalledTimes(1);
		expect(survivor).toHaveBeenCalledTimes(1);
	});

	it('clear() removes every subscription', () => {
		const handler = vi.fn();
		bus.on('execution-complete', handler);
		bus.clear();

		bus.emit('execution-complete', {
			sessionId: 's',
			conversationId: 'c',
			threadId: 't',
			content: '',
			artifacts: []
		});

		expect(handler).not.toHaveBeenCalled();
	});
});
