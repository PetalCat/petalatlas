<script lang="ts">
  import { page } from "$app/stores";
  import { goto } from "$app/navigation";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import type { WorldInfo } from "$lib/types";
  import "../app.css";

  let worlds: WorldInfo[] = $state([]);
  let currentWorld: WorldInfo | null = $state(null);
  let openTabs: WorldInfo[] = $state([]);

  onMount(async () => {
    try {
      const cached = await invoke<WorldInfo[]>("load_worlds_cache");
      worlds = cached;

      if ($page.params.id) {
        const found = cached.find((w) => w.id === $page.params.id);
        if (found) {
          currentWorld = found;
          if (!openTabs.some((t) => t.id === found.id)) {
            openTabs = [...openTabs, found];
          }
        }
      }
    } catch (e) {
      console.error("Failed to load worlds:", e);
    }
  });

  $effect(() => {
    if ($page.params.id && worlds.length > 0) {
      const found = worlds.find((w) => w.id === $page.params.id);
      if (found) {
        currentWorld = found;
        if (!openTabs.some((t) => t.id === found.id)) {
          openTabs = [...openTabs, found];
        }
      }
    } else if (!$page.params.id) {
      currentWorld = null;
    }
  });

  function closeTab(worldId: string, event: Event) {
    event.stopPropagation();

    const index = openTabs.findIndex((t) => t.id === worldId);
    if (index === -1) return;

    if (currentWorld?.id === worldId) {
      if (openTabs.length > 1) {
        const nextTab = openTabs[index + 1] || openTabs[index - 1];
        goto(`/world/${nextTab.id}`);
      } else {
        goto("/");
      }
    }

    openTabs = openTabs.filter((t) => t.id !== worldId);
  }

  function switchWorld(world: WorldInfo) {
    goto(`/world/${world.id}`);
  }

  function goHome() {
    goto("/");
  }
</script>

<!-- HEADER (fixed) -->
<header class="shell-header">
  <div class="shell-header-left">
    <button class="logo-btn" onclick={goHome}>
      <img src="/logo.png" alt="Logo" class="logo-img" />
      <span class="app-name">Petal Atlas</span>
    </button>

    {#if openTabs.length > 0}
      <div class="tab-bar-container">
        <div class="tab-bar">
          {#each openTabs as tab (tab.id)}
            <div
              class="tab-item"
              class:active={tab.id === currentWorld?.id}
              role="button"
              tabindex="0"
              onclick={() => switchWorld(tab)}
              onkeydown={(e) => e.key === "Enter" && switchWorld(tab)}
            >
              {#if tab.icon}
                <img
                  src={`data:image/png;base64,${tab.icon}`}
                  alt=""
                  class="tab-icon"
                />
              {:else}
                <span class="tab-icon">🌍</span>
              {/if}
              <span class="tab-name">{tab.name}</span>
              <button
                class="tab-close"
                onclick={(e) => closeTab(tab.id, e)}
                title="Close"
              >
                ×
              </button>
            </div>
          {/each}
        </div>
      </div>
    {/if}
  </div>

  <div class="shell-header-right">
    <button
      class="settings-btn"
      onclick={() => goto("/settings")}
      title="Settings"
    >
      ⚙️
    </button>
    <button class="header-btn" title="Help">❓</button>
  </div>
</header>

<!-- APP SHELL: sidebar + content host -->
<div class="shell-main">
  {#if $page.params.id}
    <aside class="shell-sidebar">
      <nav class="sidebar-nav">
        <div class="nav-section">
          <span class="nav-section-title">World Tools</span>

          <a
            href="/world/{$page.params.id}"
            class="nav-item"
            class:active={$page.url.pathname === `/world/${$page.params.id}`}
          >
            <span class="nav-icon">📋</span>
            <span class="nav-label">Overview</span>
          </a>

          <a
            href="/world/{$page.params.id}/nbt"
            class="nav-item"
            class:active={$page.url.pathname.includes("/nbt")}
            class:disabled={currentWorld?.is_zip}
          >
            <span class="nav-icon">✏️</span>
            <span class="nav-label">NBT Editor</span>
            {#if currentWorld?.is_zip}
              <span class="nav-badge">Read-only</span>
            {/if}
          </a>

          <a
            href="/world/{$page.params.id}/players"
            class="nav-item"
            class:active={$page.url.pathname.includes("/players")}
          >
            <span class="nav-icon">👤</span>
            <span class="nav-label">Players</span>
          </a>

          <a
            href="/world/{$page.params.id}/map"
            class="nav-item"
            class:active={$page.url.pathname.includes("/map")}
          >
            <span class="nav-icon">🗺️</span>
            <span class="nav-label">Map</span>
          </a>

          <button class="nav-item" disabled>
            <span class="nav-icon">🗂️</span>
            <span class="nav-label">World Data</span>
            <span class="nav-badge">Soon</span>
          </button>
        </div>

        <div class="nav-section">
          <span class="nav-section-title">Advanced</span>
          <button class="nav-item" disabled>
            <span class="nav-icon">🗺️</span>
            <span class="nav-label">Regions</span>
            <span class="nav-badge">v2</span>
          </button>
          <button class="nav-item" disabled>
            <span class="nav-icon">🌱</span>
            <span class="nav-label">Seeds</span>
            <span class="nav-badge">v2</span>
          </button>
          <button class="nav-item" disabled>
            <span class="nav-icon">🛠️</span>
            <span class="nav-label">Tools</span>
            <span class="nav-badge">v3</span>
          </button>
        </div>
      </nav>
    </aside>
  {/if}

  <main class="shell-content">
    <!-- Pages (World Library, Map, etc) live entirely inside here -->
    <slot />
  </main>
</div>

<style>
  :global(html, body) {
    margin: 0;
    padding: 0;
    height: 100%;
    overflow: hidden;
    background: var(--color-bg-1);
  }

  *,
  *::before,
  *::after {
    box-sizing: border-box;
  }

  /* HEADER */

  .shell-header {
    height: 60px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 1rem;
    background: var(--color-bg-2);
    border-bottom: 1px solid var(--color-bg-3);
    flex-shrink: 0;
  }

  .shell-header-left,
  .shell-header-right {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    height: 100%;
  }

  .logo-btn {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background: none;
    border: none;
    cursor: pointer;
    padding: 0.5rem;
    border-radius: var(--radius-md);
    transition: background 0.15s;
  }

  .logo-btn:hover {
    background: var(--color-bg-3);
  }

  .logo-img {
    width: 42px;
    height: 42px;
    object-fit: contain;
    image-rendering: pixelated;
  }

  .app-name {
    font-size: 1.125rem;
    font-weight: 700;
    color: var(--color-text-1);
    letter-spacing: -0.01em;
  }

  .settings-btn,
  .header-btn {
    width: 40px;
    height: 40px;
    border-radius: var(--radius-md);
    border: none;
    background: var(--color-bg-3);
    font-size: 1.25rem;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition:
      background 0.15s,
      transform 0.15s;
  }

  .settings-btn:hover {
    background: var(--color-bg-4);
    transform: rotate(30deg);
  }

  .header-btn:hover {
    background: var(--color-bg-4);
  }

  /* TABS */

  .tab-bar-container {
    flex: 1;
    min-width: 0;
    margin-left: 1rem;
    overflow: hidden;
  }

  .tab-bar {
    display: flex;
    gap: 0.25rem;
    overflow-x: auto;
    overflow-y: hidden;
    scrollbar-width: none;
  }

  .tab-bar::-webkit-scrollbar {
    display: none;
  }

  .tab-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    background: var(--color-bg-3);
    border-radius: var(--radius-md) var(--radius-md) 0 0;
    cursor: pointer;
    min-width: 0;
    max-width: 220px;
    white-space: nowrap;
    transition: background 0.15s;
  }

  .tab-item:hover {
    background: var(--color-bg-4);
  }

  .tab-item.active {
    background: var(--color-bg-1);
    border-bottom: 2px solid var(--color-primary);
  }

  .tab-icon {
    width: 20px;
    height: 20px;
    border-radius: var(--radius-sm);
    object-fit: cover;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--color-bg-2);
    font-size: 0.75rem;
  }

  .tab-name {
    font-size: 0.875rem;
    font-weight: 500;
    color: var(--color-text-1);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    min-width: 0;
  }

  .tab-close {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: #ff5f57;
    border: none;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 10px;
    line-height: 1;
    color: transparent;
    transition: all 0.15s;
    flex-shrink: 0;
    padding: 0;
    margin: 0;
  }

  .tab-item:hover .tab-close {
    color: rgba(0, 0, 0, 0.6);
  }

  .tab-close:hover {
    background: #ff3b30;
    color: rgba(0, 0, 0, 0.8);
    transform: scale(1.1);
  }

  /* SHELL */

  .shell-main {
    display: flex;
    height: calc(100vh - 60px);
    overflow: hidden;
  }

  .shell-sidebar {
    width: 240px;
    flex-shrink: 0;
    height: 100%;
    overflow-y: auto;
    background: var(--color-bg-2);
    border-right: 1px solid var(--color-bg-3);
  }

  .sidebar-nav {
    padding: 1rem 0;
  }

  .nav-section {
    margin-bottom: 1.5rem;
  }

  .nav-section-title {
    display: block;
    padding: 0 1.5rem 0.75rem 1.5rem;
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--color-text-3);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    width: 100%;
    padding: 0.75rem 1.5rem;
    background: none;
    border: none;
    text-decoration: none;
    color: var(--color-text-2);
    cursor: pointer;
    transition:
      background 0.15s,
      color 0.15s;
    position: relative;
  }

  .nav-item:hover:not(:disabled) {
    background: var(--color-bg-3);
    color: var(--color-text-1);
  }

  .nav-item.active {
    background: var(--color-primary);
    color: white;
  }

  .nav-item.active::before {
    content: "";
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 3px;
    background: white;
  }

  .nav-item.disabled,
  .nav-item:disabled {
    opacity: 0.5;
    cursor: not-allowed;
    pointer-events: none;
  }

  .nav-icon {
    font-size: 1.125rem;
    width: 1.5rem;
    text-align: center;
  }

  .nav-label {
    flex: 1;
    text-align: left;
    font-size: 0.875rem;
    font-weight: 500;
  }

  .nav-badge {
    font-size: 0.625rem;
    padding: 0.125rem 0.375rem;
    background: var(--color-bg-4);
    border-radius: var(--radius-sm);
    font-weight: 600;
    text-transform: uppercase;
  }

  .nav-item.active .nav-badge {
    background: rgba(255, 255, 255, 0.2);
  }

  /* CONTENT HOST: pages control their own layout inside here */

  .shell-content {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    /* no padding, no scroll here */
    overflow: hidden;
  }
</style>
