<script lang="ts">
  import { goto } from "$app/navigation";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  let useSystemProxy = $state(true); // Default to using system proxies
  let clearing = $state<string | null>(null);

  // Load settings on mount
  onMount(() => {
    const saved = localStorage.getItem("useSystemProxy");
    if (saved !== null) {
      useSystemProxy = saved === "true";
    }
  });

  // Save proxy setting when changed
  $effect(() => {
    localStorage.setItem("useSystemProxy", String(useSystemProxy));
  });

  async function clearWorldsCache() {
    clearing = "worlds";
    try {
      console.log("Clearing world cache...");
      await invoke("save_worlds_cache", { worlds: [] });
      alert("World cache cleared successfully!");
      location.reload(); // Reload to show empty state
    } catch (e) {
      console.error("Clear world cache error:", e);
      alert("Failed to clear world cache: " + e);
    } finally {
      clearing = null;
    }
  }

  async function clearPlayerCache() {
    clearing = "players";
    try {
      console.log("Clearing player cache...");
      await invoke("clear_player_caches");
      alert("Player data cache cleared successfully!");
    } catch (e) {
      console.error("Clear player cache error:", e);
      alert("Failed to clear player cache: " + e);
    } finally {
      clearing = null;
    }
  }

  async function clearAllData() {
    if (
      !confirm(
        "Are you sure you want to clear ALL app data? This cannot be undone."
      )
    ) {
      return;
    }

    clearing = "all";
    try {
      console.log("Clearing all data...");
      await invoke("save_worlds_cache", { worlds: [] });
      await invoke("clear_player_caches");
      localStorage.clear();
      alert("All app data cleared! The app will reload.");
      location.reload();
    } catch (e) {
      console.error("Clear all data error:", e);
      alert("Failed to clear all data: " + e);
    } finally {
      clearing = null;
    }
  }
</script>

<div class="settings-page">
  <div class="settings-header">
    <button class="back-btn" onclick={() => goto("/")}>← Back</button>
    <h1>⚙️ Settings</h1>
  </div>

  <div class="settings-content">
    <section class="settings-section">
      <h2>Network</h2>
      <div class="setting-item">
        <div class="setting-info">
          <h3>Use System Proxies</h3>
          <p>Enable to use system proxy settings for API requests</p>
        </div>
        <label class="toggle">
          <input type="checkbox" bind:checked={useSystemProxy} />
          <span class="toggle-slider"></span>
        </label>
      </div>
      {#if useSystemProxy}
        <div class="info-banner">
          ℹ️ System proxy settings will be used for API requests.
        </div>
      {:else}
        <div class="info-banner">
          ℹ️ Proxy bypass is enabled. API requests will ignore system proxies.
        </div>
      {/if}
    </section>

    <section class="settings-section">
      <h2>Data Management</h2>
      <div class="data-actions">
        <div class="action-item">
          <div class="action-info">
            <h3>Clear World Cache</h3>
            <p>Remove cached world data and scan results</p>
          </div>
          <button
            class="action-btn danger"
            onclick={clearWorldsCache}
            disabled={clearing === "worlds"}
          >
            {clearing === "worlds" ? "Clearing..." : "Clear Cache"}
          </button>
        </div>

        <div class="action-item">
          <div class="action-info">
            <h3>Clear Player Cache</h3>
            <p>Remove cached player names and skin images</p>
          </div>
          <button
            class="action-btn danger"
            onclick={clearPlayerCache}
            disabled={clearing === "players"}
          >
            {clearing === "players" ? "Clearing..." : "Clear Cache"}
          </button>
        </div>

        <div class="action-item">
          <div class="action-info">
            <h3>Clear All Data</h3>
            <p>Reset the app to factory defaults</p>
          </div>
          <button
            class="action-btn danger-solid"
            onclick={clearAllData}
            disabled={clearing === "all"}
          >
            {clearing === "all" ? "Clearing..." : "Clear All Data"}
          </button>
        </div>
      </div>
    </section>

    <section class="settings-section">
      <h2>About</h2>
      <div class="info-grid">
        <div class="info-item">
          <span class="info-label">App Name</span>
          <span class="info-value">Petal Atlas</span>
        </div>
        <div class="info-item">
          <span class="info-label">Version</span>
          <span class="info-value">0.1.0</span>
        </div>
        <div class="info-item">
          <span class="info-label">Platform</span>
          <span class="info-value">macOS (Apple Silicon)</span>
        </div>
      </div>
    </section>

    <section class="settings-section">
      <h2>Features</h2>
      <div class="feature-list">
        <div class="feature-item">
          <span class="feature-icon">✅</span>
          <div class="feature-info">
            <h3>Edition-Aware Player System</h3>
            <p>Support for both Java and Bedrock Edition player data</p>
          </div>
        </div>
        <div class="feature-item">
          <span class="feature-icon">✅</span>
          <div class="feature-info">
            <h3>NBT Editor</h3>
            <p>View and edit world NBT data with syntax highlighting</p>
          </div>
        </div>
        <div class="feature-item">
          <span class="feature-icon">✅</span>
          <div class="feature-info">
            <h3>Player Skin Viewer</h3>
            <p>3D skin rendering with auto-rotation</p>
          </div>
        </div>
        <div class="feature-item">
          <span class="feature-icon">✅</span>
          <div class="feature-info">
            <h3>Proxy Bypass</h3>
            <p>Backend API calls to avoid network restrictions</p>
          </div>
        </div>
      </div>
    </section>
  </div>
</div>

<style>
  .settings-page {
    max-width: 900px;
    margin: 0 auto;
    padding: 2rem;
  }

  .settings-header {
    display: flex;
    align-items: center;
    gap: 1rem;
    margin-bottom: 2rem;
  }

  .back-btn {
    padding: 0.5rem 1rem;
    background: var(--color-primary);
    color: white;
    border: none;
    border-radius: var(--radius-md);
    cursor: pointer;
    font-size: 0.875rem;
    transition: all 0.2s;
  }

  .back-btn:hover {
    background: var(--color-primary-dark);
    transform: translateX(-2px);
  }

  .settings-header h1 {
    margin: 0;
    font-size: 2rem;
  }

  .settings-content {
    display: flex;
    flex-direction: column;
    gap: 2rem;
  }

  .settings-section {
    background: var(--color-bg-2);
    padding: 2rem;
    border-radius: var(--radius-lg);
  }

  .settings-section h2 {
    margin: 0 0 1.5rem 0;
    font-size: 1.25rem;
    color: var(--color-text-1);
  }

  .info-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 1.5rem;
  }

  .info-item {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .info-label {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--color-text-3);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .info-value {
    font-size: 1rem;
    color: var(--color-text-1);
    font-weight: 500;
  }

  .feature-list {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .feature-item {
    display: flex;
    gap: 1rem;
    align-items: flex-start;
    padding: 1rem;
    background: var(--color-bg-3);
    border-radius: var(--radius-md);
  }

  .feature-icon {
    font-size: 1.5rem;
    flex-shrink: 0;
  }

  .feature-info h3 {
    margin: 0 0 0.25rem 0;
    font-size: 1rem;
    color: var(--color-text-1);
  }

  .feature-info p {
    margin: 0;
    font-size: 0.875rem;
    color: var(--color-text-3);
  }

  .setting-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 2rem;
  }

  .setting-info h3 {
    margin: 0 0 0.25rem 0;
    font-size: 1rem;
    color: var(--color-text-1);
  }

  .setting-info p {
    margin: 0;
    font-size: 0.875rem;
    color: var(--color-text-3);
  }

  .toggle {
    position: relative;
    display: inline-block;
    width: 48px;
    height: 28px;
    flex-shrink: 0;
  }

  .toggle input {
    opacity: 0;
    width: 0;
    height: 0;
  }

  .toggle-slider {
    position: absolute;
    cursor: pointer;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: var(--color-bg-4);
    transition: 0.3s;
    border-radius: 28px;
  }

  .toggle-slider:before {
    position: absolute;
    content: "";
    height: 20px;
    width: 20px;
    left: 4px;
    bottom: 4px;
    background-color: white;
    transition: 0.3s;
    border-radius: 50%;
  }

  .toggle input:checked + .toggle-slider {
    background-color: var(--color-primary);
  }

  .toggle input:checked + .toggle-slider:before {
    transform: translateX(20px);
  }

  .info-banner {
    margin-top: 1rem;
    padding: 0.75rem 1rem;
    background: rgba(59, 130, 246, 0.1);
    border-left: 3px solid var(--color-primary);
    border-radius: var(--radius-md);
    font-size: 0.875rem;
    color: var(--color-text-2);
  }

  .data-actions {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .action-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem;
    background: var(--color-bg-3);
    border-radius: var(--radius-md);
  }

  .action-info h3 {
    margin: 0 0 0.25rem 0;
    font-size: 1rem;
    color: var(--color-text-1);
  }

  .action-info p {
    margin: 0;
    font-size: 0.875rem;
    color: var(--color-text-3);
  }

  .action-btn {
    padding: 0.5rem 1rem;
    border-radius: var(--radius-md);
    cursor: pointer;
    font-size: 0.875rem;
    font-weight: 600;
    transition: all 0.2s;
    border: none;
    white-space: nowrap;
  }

  .action-btn.danger {
    background: rgba(239, 68, 68, 0.1);
    color: #ef4444;
    border: 1px solid rgba(239, 68, 68, 0.3);
  }

  .action-btn.danger:hover:not(:disabled) {
    background: rgba(239, 68, 68, 0.2);
  }

  .action-btn.danger-solid {
    background: #ef4444;
    color: white;
  }

  .action-btn.danger-solid:hover:not(:disabled) {
    background: #dc2626;
  }

  .action-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
