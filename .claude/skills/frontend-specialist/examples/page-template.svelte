<!--
  PageTemplate.svelte
  Standard page layout template for Coder application pages.

  @example
  <PageTemplate title="Settings">
    <SettingsContent />
  </PageTemplate>
-->
<script lang="ts">
  // ============================================
  // 1. IMPORTS
  // ============================================
  import Sidebar from '$lib/components/Sidebar.svelte';
  import Header from '$lib/components/Header.svelte';
  import styles from './PageTemplate.module.css';

  // ============================================
  // 2. TYPES
  // ============================================
  interface Props {
    /** Page title (shown in header) */
    title: string;

    /** Page subtitle/description */
    subtitle?: string;

    /** Breadcrumb navigation items */
    breadcrumbs?: BreadcrumbItem[];

    /** Header action buttons */
    actions?: HeaderAction[];

    /** Show sidebar */
    showSidebar?: boolean;

    /** Show header */
    showHeader?: boolean;

    /** Page content */
    children?: import('svelte').Snippet;

    /** Sidebar content (optional) */
    sidebarContent?: import('svelte').Snippet;
  }

  interface BreadcrumbItem {
    label: string;
    href?: string;
  }

  interface HeaderAction {
    id: string;
    label: string;
    onclick: () => void;
    variant?: 'primary' | 'secondary';
  }

  // ============================================
  // 3. PROPS
  // ============================================
  let {
    title,
    subtitle,
    breadcrumbs,
    actions,
    showSidebar = true,
    showHeader = true,
    children,
    sidebarContent
  }: Props = $props();

  // ============================================
  // 4. STATE
  // ============================================
  let sidebarCollapsed = $state(false);

  // ============================================
  // 5. HANDLERS
  // ============================================
  function toggleSidebar() {
    sidebarCollapsed = !sidebarCollapsed;
  }
</script>

<!-- ============================================
     6. TEMPLATE
     ============================================ -->
<div class={styles.app}>
  {#if showSidebar}
    <Sidebar collapsed={sidebarCollapsed} onToggle={toggleSidebar}>
      {#if sidebarContent}
        {@render sidebarContent()}
      {/if}
    </Sidebar>
  {/if}

  <div class={styles.main}>
    {#if showHeader}
      <Header {title} {subtitle} {breadcrumbs} {actions} />
    {/if}

    <main class={styles.content}>
      {@render children?.()}
    </main>
  </div>
</div>
