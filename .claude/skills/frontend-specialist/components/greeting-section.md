# Greeting Section Component

Personalized welcome message for the user.

## TypeScript Interface

```typescript
interface GreetingSectionProps {
  /** User's name */
  name?: string;

  /** Custom greeting (overrides auto-generated) */
  greeting?: string;

  /** Subtitle/description */
  subtitle?: string;

  /** Show time-based greeting */
  showTimeGreeting?: boolean;
}
```

---

## Design Tokens

```css
/* Typography - Greeting */
font-size: var(--text-3xl);         /* 28px */
font-weight: var(--font-semibold);  /* 600 */

/* Typography - Subtitle */
font-size: var(--text-sm);          /* 14px */
color: var(--color-text-secondary);

/* Spacing */
margin-bottom: var(--space-2);      /* Greeting */
margin-bottom: var(--space-6);      /* Section */
```

---

## Implementation

```svelte
<script lang="ts">
  import styles from './GreetingSection.module.css';

  interface Props {
    name?: string;
    greeting?: string;
    subtitle?: string;
    showTimeGreeting?: boolean;
  }

  let {
    name,
    greeting,
    subtitle = 'How can I help you today?',
    showTimeGreeting = true
  }: Props = $props();

  function getTimeGreeting(): string {
    const hour = new Date().getHours();
    if (hour < 12) return 'Good morning';
    if (hour < 18) return 'Good afternoon';
    return 'Good evening';
  }

  let displayGreeting = $derived(() => {
    if (greeting) return greeting;
    if (showTimeGreeting) {
      return name ? `${getTimeGreeting()}, ${name}` : getTimeGreeting();
    }
    return name ? `Hello, ${name}` : 'Hello';
  });
</script>

<section class={styles.section}>
  <h1 class={styles.greeting}>{displayGreeting()}</h1>
  {#if subtitle}
    <p class={styles.subtitle}>{subtitle}</p>
  {/if}
</section>
```

```css
/* GreetingSection.module.css */
.section {
  margin-bottom: var(--space-6);
}

.greeting {
  font-size: var(--text-3xl);
  font-weight: var(--font-semibold);
  color: var(--color-text-primary);
  margin: 0 0 var(--space-2);
  line-height: var(--leading-tight);
}

.subtitle {
  font-size: var(--text-sm);
  color: var(--color-text-secondary);
  margin: 0;
  line-height: var(--leading-normal);
}
```

---

## Variations

### With Avatar

```svelte
<section class={styles.section}>
  <div class={styles.header}>
    {#if avatarUrl}
      <img src={avatarUrl} alt="" class={styles.avatar} />
    {/if}
    <div>
      <h1 class={styles.greeting}>{displayGreeting()}</h1>
      <p class={styles.subtitle}>{subtitle}</p>
    </div>
  </div>
</section>

<style>
  .header {
    display: flex;
    align-items: center;
    gap: var(--space-4);
  }

  .avatar {
    width: 48px;
    height: 48px;
    border-radius: var(--radius-full);
    object-fit: cover;
  }
</style>
```

### With Quick Stats

```svelte
<section class={styles.section}>
  <h1 class={styles.greeting}>{displayGreeting()}</h1>
  <p class={styles.subtitle}>{subtitle}</p>

  {#if stats}
    <div class={styles.stats}>
      <div class={styles.stat}>
        <span class={styles.statValue}>{stats.sessions}</span>
        <span class={styles.statLabel}>Sessions</span>
      </div>
      <div class={styles.stat}>
        <span class={styles.statValue}>{stats.filesEdited}</span>
        <span class={styles.statLabel}>Files edited</span>
      </div>
    </div>
  {/if}
</section>
```
