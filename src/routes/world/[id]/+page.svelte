<script lang="ts">
  import { page } from "$app/stores";
  import { goto } from "$app/navigation";
  import { invoke } from "@tauri-apps/api/core";
  import type { WorldInfo } from "$lib/types";
  import { onMount } from "svelte";

  let world = $state<WorldInfo | null>(null);
  let worlds = $state<WorldInfo[]>([]);
  let loading = $state(true);
  let error = $state("");

  // Load world data
  async function loadWorld() {
    loading = true;
    error = "";
    world = null; // Clear world data before loading new one

    try {
      const cached = await invoke<WorldInfo[]>("load_worlds_cache");
      worlds = cached;

      const id = $page.params.id;
      const found = cached.find((w) => w.id === id);

      if (found) {
        world = found;
      } else {
        error = "World not found";
      }
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    loadWorld();
  });

  // Reload when route changes
  $effect(() => {
    // Check if the ID parameter exists and has changed
    // Svelte's $effect automatically re-runs when dependencies ($page.params.id) change
    if ($page.params.id) {
      loadWorld();
    }
  });

  function goBack() {
    goto("/");
  }
</script>

{#if loading}
  <div class="loading">
    <div class="spinner"></div>
    <p>Loading world...</p>
  </div>
{:else if error || !world}
  <div class="error-page">
    <h1>⚠️ Error</h1>
    <p>{error || "World not found"}</p>
    <button onclick={goBack}>Back to Worlds</button>
  </div>
{:else}
  <div class="overview">
    <!-- World Hero Section -->
    <section class="hero-section">
      <div class="hero-content">
        {#if world.icon}
          <img
            src="data:image/png;base64,{world.icon}"
            alt="World icon"
            class="hero-icon"
          />
        {:else}
          <div class="hero-icon-placeholder">🌍</div>
        {/if}
        <div class="hero-info">
          <h1>{world.name}</h1>
          <div class="hero-meta">
            <span
              class="meta-badge {world.platform.toLowerCase().includes('java')
                ? 'java'
                : 'bedrock'}"
            >
              {world.platform}
            </span>
            <span class="meta-item">📦 {world.folder_name}</span>
            {#if world.is_zip}
              <span class="meta-badge zip">Zipped Backup</span>
            {/if}
          </div>
        </div>
      </div>
    </section>

    <!-- Quick Actions -->
    <section class="quick-actions">
      <h2>Quick Actions</h2>
      <div class="action-grid">
        <button
          class="action-card"
          disabled={world.is_zip}
          onclick={() => goto($page.url.pathname + "/map")}
        >
          <span class="action-icon">🗺️</span>
          <span class="action-title">Map Viewer</span>
          <span class="action-desc">View chunk map</span>
        </button>
        <button class="action-card" disabled={world.is_zip}>
          <span class="action-icon">✏️</span>
          <span class="action-title">Edit NBT</span>
          <span class="action-desc">Modify world settings</span>
        </button>
        <button class="action-card">
          <span class="action-icon">💾</span>
          <span class="action-title">Backup World</span>
          <span class="action-desc">Create a ZIP backup</span>
        </button>
        <button class="action-card">
          <span class="action-icon">📋</span>
          <span class="action-title">Duplicate</span>
          <span class="action-desc">Copy this world</span>
        </button>
        <button class="action-card">
          <span class="action-icon">📁</span>
          <span class="action-title">Open Folder</span>
          <span class="action-desc">View in file browser</span>
        </button>
      </div>
    </section>

    <!-- World Stats -->
    <section class="stats-section">
      <h2>World Information</h2>
      <div class="stats-grid">
        <div class="stat-card">
          <div class="stat-header">
            <span class="stat-icon">🎮</span>
            <span class="stat-label">Game Mode</span>
          </div>
          <span class="stat-value">{world.game_mode}</span>
        </div>

        <div class="stat-card">
          <div class="stat-header">
            <span class="stat-icon">📦</span>
            <span class="stat-label">Version</span>
          </div>
          <span class="stat-value">{world.version_id}</span>
        </div>

        {#if world.seed}
          <div class="stat-card">
            <div class="stat-header">
              <span class="stat-icon">🌱</span>
              <span class="stat-label">Seed</span>
            </div>
            <span class="stat-value seed">{world.seed}</span>
          </div>
        {/if}

        <div class="stat-card">
          <div class="stat-header">
            <span class="stat-icon">💾</span>
            <span class="stat-label">Size</span>
          </div>
          <span class="stat-value">
            {world.size > 1024 * 1024 * 1024
              ? (world.size / (1024 * 1024 * 1024)).toFixed(2) + " GB"
              : world.size > 1024 * 1024
                ? (world.size / (1024 * 1024)).toFixed(2) + " MB"
                : world.size > 1024
                  ? (world.size / 1024).toFixed(2) + " KB"
                  : world.size + " B"}
          </span>
        </div>

        <div class="stat-card">
          <div class="stat-header">
            <span class="stat-icon">🕐</span>
            <span class="stat-label">Last Played</span>
          </div>
          <span class="stat-value">
            {new Date(world.last_played).toLocaleDateString()}
          </span>
          <span class="stat-subvalue">
            {new Date(world.last_played).toLocaleTimeString()}
          </span>
        </div>

        <div class="stat-card full-width">
          <div class="stat-header">
            <span class="stat-icon">📍</span>
            <span class="stat-label">Location</span>
          </div>
          <span class="stat-value path">{world.path}</span>
        </div>
      </div>
    </section>

    <!-- Warnings/Status -->
    {#if world.is_zip}
      <section class="warnings-section">
        <div class="warning-card">
          <span class="warning-icon">⚠️</span>
          <div class="warning-content">
            <h3>Read-Only World</h3>
            <p>
              This world is a zipped backup and cannot be modified directly.
              Extract it first to enable editing.
            </p>
          </div>
        </div>
      </section>
    {/if}
  </div>
{/if}

<style>
  .loading {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 100vh;
    gap: 1rem;
  }

  .spinner {
    width: 48px;
    height: 48px;
    border: 4px solid var(--color-bg-3);
    border-top-color: var(--color-primary);
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .error-page {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 100vh;
    text-align: center;
    padding: 2rem;
  }

  .error-page h1 {
    font-size: 3rem;
    margin-bottom: 1rem;
  }

  .error-page button {
    margin-top: 2rem;
    padding: 0.75rem 1.5rem;
    background: var(--color-primary);
    color: white;
    border: none;
    border-radius: var(--radius-md);
    cursor: pointer;
    font-size: 1rem;
  }

  .overview {
    padding: 2rem;
    max-width: 1400px;
    margin: 0 auto;
  }

  /* Hero Section */
  .hero-section {
    margin-bottom: 3rem;
  }

  .hero-content {
    display: flex;
    align-items: center;
    gap: 2rem;
    padding: 2rem;
    background: linear-gradient(
      135deg,
      var(--color-bg-2) 0%,
      var(--color-bg-3) 100%
    );
    border-radius: var(--radius-lg);
  }

  .hero-icon {
    width: 96px;
    height: 96px;
    border-radius: var(--radius-lg);
    object-fit: cover;
    box-shadow: var(--shadow-lg);
  }

  .hero-icon-placeholder {
    width: 96px;
    height: 96px;
    border-radius: var(--radius-lg);
    background: var(--color-bg-3);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 3rem;
    box-shadow: var(--shadow-lg);
  }

  .hero-info h1 {
    margin: 0 0 0.5rem 0;
    font-size: 2rem;
    font-weight: 700;
  }

  .hero-meta {
    display: flex;
    align-items: center;
    gap: 1rem;
    flex-wrap: wrap;
  }

  .meta-badge {
    padding: 0.25rem 0.75rem;
    border-radius: var(--radius-md);
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .meta-badge.java {
    background: #ff6b6b;
    color: white;
  }

  .meta-badge.bedrock {
    background: #4ecdc4;
    color: white;
  }

  .meta-badge.zip {
    background: #ffa502;
    color: white;
  }

  .meta-item {
    color: var(--color-text-3);
    font-size: 0.875rem;
  }

  /* Quick Actions */
  .quick-actions {
    margin-bottom: 3rem;
  }

  .quick-actions h2 {
    margin: 0 0 1.5rem 0;
    font-size: 1.5rem;
  }

  .action-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 1rem;
  }

  .action-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.75rem;
    padding: 2rem 1.5rem;
    background: var(--color-bg-2);
    border: 2px solid var(--color-bg-3);
    border-radius: var(--radius-lg);
    cursor: pointer;
    transition: all 0.2s;
  }

  .action-card:hover:not(:disabled) {
    border-color: var(--color-primary);
    background: var(--color-bg-3);
    transform: translateY(-2px);
    box-shadow: var(--shadow-lg);
  }

  .action-card:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .action-icon {
    font-size: 2.5rem;
  }

  .action-title {
    font-weight: 600;
    font-size: 1rem;
    color: var(--color-text-1);
  }

  .action-desc {
    font-size: 0.75rem;
    color: var(--color-text-3);
    text-align: center;
  }

  /* Stats Section */
  .stats-section h2 {
    margin: 0 0 1.5rem 0;
    font-size: 1.5rem;
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 1rem;
  }

  .stat-card {
    padding: 1.5rem;
    background: var(--color-bg-2);
    border-radius: var(--radius-lg);
    border: 1px solid var(--color-bg-3);
  }

  .stat-card.full-width {
    grid-column: 1 / -1;
  }

  .stat-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 1rem;
  }

  .stat-icon {
    font-size: 1.25rem;
  }

  .stat-label {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--color-text-3);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .stat-value {
    display: block;
    font-size: 1.25rem;
    font-weight: 700;
    color: var(--color-text-1);
  }

  .stat-value.seed {
    font-family: "Courier New", monospace;
    background: var(--color-bg-3);
    padding: 0.5rem;
    border-radius: var(--radius-sm);
    font-size: 1rem;
  }

  .stat-value.path {
    font-family: "Courier New", monospace;
    font-size: 0.875rem;
    color: var(--color-text-3);
    word-break: break-all;
    font-weight: 400;
  }

  .stat-subvalue {
    display: block;
    font-size: 0.875rem;
    color: var(--color-text-3);
    margin-top: 0.25rem;
  }

  /* Warnings Section */
  .warnings-section {
    margin-top: 2rem;
  }

  .warning-card {
    display: flex;
    align-items: flex-start;
    gap: 1rem;
    padding: 1.5rem;
    background: #fef3c7;
    border-left: 4px solid #f59e0b;
    border-radius: var(--radius-lg);
  }

  .warning-icon {
    font-size: 1.5rem;
    flex-shrink: 0;
  }

  .warning-content h3 {
    margin: 0 0 0.5rem 0;
    color: #92400e;
    font-size: 1.125rem;
  }

  .warning-content p {
    margin: 0;
    color: #78350f;
    font-size: 0.875rem;
  }
</style>
