<script lang="ts">
	/**
	 * ChatArea Component
	 * Main chat interface combining message list and input box.
	 */
	import ChatMessageList from './ChatMessageList.svelte';
	import InputBox from './InputBox.svelte';
	import styles from './ChatArea.module.css';

	interface Message {
		id: string;
		role: 'user' | 'assistant' | 'system';
		content: string;
		timestamp: Date;
		status?: 'sending' | 'sent' | 'error';
	}

	interface FileAttachment {
		id: string;
		name: string;
		type: string;
		size: number;
		preview?: string;
	}

	interface Props {
		/** Array of messages to display */
		messages: Message[];

		/** Current input value */
		inputValue: string;

		/** Is assistant typing */
		isTyping?: boolean;

		/** Is input disabled */
		isDisabled?: boolean;

		/** Is loading/sending */
		isLoading?: boolean;

		/** Attached files */
		attachments?: FileAttachment[];

		/** Input change handler */
		onInputChange: (value: string) => void;

		/** Submit message handler */
		onSubmit: () => void;

		/** Attach files handler */
		onAttach?: (files: File[]) => void;

		/** Remove attachment handler */
		onRemoveAttachment?: (id: string) => void;

		/** Edit message handler */
		onEditMessage?: (id: string) => void;

		/** Delete message handler */
		onDeleteMessage?: (id: string) => void;

		/** Retry failed message handler */
		onRetryMessage?: (id: string) => void;
	}

	let {
		messages,
		inputValue,
		isTyping = false,
		isDisabled = false,
		isLoading = false,
		attachments = [],
		onInputChange,
		onSubmit,
		onAttach,
		onRemoveAttachment,
		onEditMessage,
		onDeleteMessage,
		onRetryMessage
	}: Props = $props();
</script>

<div class={styles.chatArea}>
	<!-- Message list takes remaining space -->
	<ChatMessageList {messages} {isTyping} {onEditMessage} {onDeleteMessage} {onRetryMessage} />

	<!-- Input box at bottom -->
	<div class={styles.inputContainer}>
		<InputBox
			value={inputValue}
			onchange={onInputChange}
			onsubmit={onSubmit}
			disabled={isDisabled}
			loading={isLoading}
			{attachments}
			onattach={onAttach}
			onremoveattachment={onRemoveAttachment}
			placeholder="Send a message to Coder..."
		/>
	</div>
</div>
