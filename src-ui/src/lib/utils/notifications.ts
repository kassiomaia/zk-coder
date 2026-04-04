/**
 * Notification Utility
 * Handles OS-level notifications using Tauri's notification plugin.
 */

import {
	isPermissionGranted,
	requestPermission,
	sendNotification
} from '@tauri-apps/plugin-notification';
import { createLogger } from './logger';

const log = createLogger('notifications');

/** Track if we've already requested permission */
let permissionRequested = false;

/** Track if permission was granted */
let hasPermission = false;

/**
 * Initialize notification permissions.
 * Should be called early in the app lifecycle.
 */
export async function initNotifications(): Promise<boolean> {
	if (permissionRequested) {
		return hasPermission;
	}

	// Check if we are in the main window.
	// The notification plugin is currently only allowed on the "main" window.
	// We use try/catch and check the window label to avoid triggering the Tauri permission error
	// in secondary windows like "agent-trace-dev".
	try {
		const { getCurrentWindow } = await import('@tauri-apps/api/window');
		const windowLabel = getCurrentWindow().label;

		if (windowLabel !== 'main') {
			log.info(`Notifications disabled for window: ${windowLabel}`);
			permissionRequested = true;
			hasPermission = false;
			return false;
		}

		// Check if permission is already granted
		let granted = await isPermissionGranted();

		if (!granted) {
			// Request permission
			const permission = await requestPermission();
			granted = permission === 'granted';
		}

		permissionRequested = true;
		hasPermission = granted;
		return granted;
	} catch (error) {
		log.error('Failed to initialize notifications:', error);
		permissionRequested = true;
		hasPermission = false;
		return false;
	}
}

/**
 * Check if notifications are available and permitted.
 */
export async function canSendNotifications(): Promise<boolean> {
	if (!permissionRequested) {
		await initNotifications();
	}
	return hasPermission;
}

/**
 * Send a notification when the assistant finishes a message.
 * @param title - Notification title
 * @param body - Notification body text
 */
export async function notifyAssistantComplete(
	title: string = 'Assistant',
	body: string = 'The assistant has finished responding.'
): Promise<void> {
	try {
		const canNotify = await canSendNotifications();
		if (!canNotify) {
			log.info('Notifications not permitted, skipping notification');
			return;
		}

		sendNotification({
			title,
			body
		});
	} catch (error) {
		log.error('Failed to send notification:', error);
	}
}

/**
 * Send a notification for a custom event.
 */
export async function sendCustomNotification(title: string, body: string): Promise<void> {
	try {
		const canNotify = await canSendNotifications();
		if (!canNotify) {
			log.info('Notifications not permitted');
			return;
		}

		sendNotification({
			title,
			body
		});
	} catch (error) {
		log.error('Failed to send notification:', error);
	}
}
