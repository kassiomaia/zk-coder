<script lang="ts">
	/**
	 * ChatArea Component (Enhanced)
	 * Main chat interface combining message list, input box, typing indicator, and enhanced features.
	 */
	import ChatMessageList from './ChatMessageList.svelte';
	import InputBox from '../InputBox.svelte';
	import TypingIndicator from './TypingIndicator.svelte';
	import styles from './ChatArea.module.css';

	interface Message {
		id: string;
		conversationId: string;
		role: 'user' | 'assistant' | 'system';
		content: string;
		timestamp: Date;
		status?: 'sending' | 'sent' | 'error';
		metadata?: MessageMetadata;
	}

	interface Props {
		/** ID of the current conversation */
	 conversationId: string;

        /** Array of messages to display */
        messages: Message[];

        /** Current input value */
        inputValue: string;

        /** Is assistant typing */
        isTyping?: boolean;

        /** Is input disabled */
        isDisabled?: boolean;

        /** Is loading/sending in progress */
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
        conversationId,
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

    // Group messages by time and role
    let messageGroups = $derived(() => {
        const groups: MessageGroup[] = [];
        let currentGroup: Message[] = [];

        for (const message of messages) {
            if (currentGroup.length === 0) {
                currentGroup.push(message);
            } else {
                const lastMessage = currentGroup[currentGroup.length - 1];
                const timeDiff =
                    message.timestamp.getTime() - lastMessage.timestamp.getTime();
                const sameRole = message.role === lastMessage.role;

                // Group if within 5 minutes and same role
                if (timeDiff < 5 * 60 * 1000 && sameRole) {
                    currentGroup.push(message);
                } else {
                    // Start a new group
                    groups.push({ messages: currentGroup });
                    currentGroup = [message];
                }
            }
        }

        if (currentGroup.length > 0) {
            groups.push({ messages: currentGroup });
        }

        return groups;
    });
</script>

<div class={styles.chatArea}>
    <!-- Message list with grouping -->
    <ChatMessageList
        {messages}
        {isTyping}
        onEditMessage={handleEditMessage}
        onDeleteMessage={handleDeleteMessage}
        onRetryMessage={handleRetryMessage}
    />

    <!-- Enhanced input area -->
    <div class={styles.inputArea}>
        <InputBox
            value={inputValue}
            onchange={onInputChange}
            onsubmit={onSubmit}
            disabled={isDisabled}
            loading={isLoading}
            attachments={attachments}
            onattach={onAttach}
            onremoveattachment={onRemoveAttachment}
            placeholder="Send a message to Coder..."
        />
    </div>
</div>
