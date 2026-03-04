/**
 * Notification Utility
 * Handles OS-level notifications using Tauri's notification plugin.
 */

import {
	isPermissionGranted,
	requestPermission,
	sendNotification
} from '@tauri-apps/plugin-notification';

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

	try {
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
		console.error('Failed to initialize notifications:', error);
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
			console.log('Notifications not permitted, skipping notification');
			return;
		}

		sendNotification({
			title,
			body
		});
	} catch (error) {
		console.error('Failed to send notification:', error);
	}
}

/**
 * Send a notification for a custom event.
 */
export async function sendCustomNotification(title: string, body: string): Promise<void> {
	try {
		const canNotify = await canSendNotifications();
		if (!canNotify) {
			console.log('Notifications not permitted');
			return;
		}

		sendNotification({
			title,
			body
		});
	} catch (error) {
		console.error('Failed to send notification:', error);
	}
}
