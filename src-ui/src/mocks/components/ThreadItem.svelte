<script lang="ts">
  interface Props {
    thread: Record<string, unknown>;
    variant?: string;
    selected?: boolean;
    isStreaming?: boolean;
    hasUnread?: boolean;
    onclick?: () => void;
    onresolve?: () => void;
    onrename?: () => void;
    ondelete?: () => void;
    onarchive?: () => void;
  }

  let {
    thread,
    variant = 'default',
    selected = false,
    isStreaming = false,
    hasUnread = false,
    onclick,
    onresolve,
    onrename,
    ondelete,
    onarchive
  }: Props = $props();
</script>

<div
  class="mock-thread-item"
  data-thread-id={thread.id}
  data-variant={variant}
  data-selected={selected}
  data-streaming={isStreaming}
  data-unread={hasUnread}
  role="button"
  tabindex="0"
  onclick={onclick}
  onkeydown={(e) => {
    if ((e.key === 'Enter' || e.key === ' ') && onclick) {
      e.preventDefault();
      onclick();
    }
  }}
>
  <span class="thread-title">{thread.title}</span>
</div>
