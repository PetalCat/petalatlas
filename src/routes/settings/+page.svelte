<script lang="ts">
  import { goto } from "$app/navigation";
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";

  // Types for Java assets
  interface JavaCapabilities {
    has_local_java_data: boolean;
    has_local_java_assets: boolean;
    java_version: string | null;
    jar_path: string | null;
    minecraft_dir: string | null;
  }

  interface FoundJar {
    id: string;
    version_type: string;
    jar_path: string | null;
  }

  let useSystemProxy = $state(true);
  let clearing = $state<string | null>(null);

  // Java assets state
  let javaCapabilities = $state<JavaCapabilities | null>(null);
  let javaLoading = $state(false);
  let javaError = $state<string | null>(null);
  let foundJars = $state<FoundJar[]>([]);
  let selectedDir = $state<string | null>(null);

  // Load settings on mount
  onMount(() => {
    const saved = localStorage.getItem("useSystemProxy");
    if (saved !== null) {
      useSystemProxy = saved === "true";
    }

    // Load saved Java capabilities if any
    const savedDir = localStorage.getItem("selectedMinecraftDir");
    const savedJar = localStorage.getItem("selectedJavaJar");
    const savedVersion = localStorage.getItem("selectedJavaVersion");
    if (savedJar) {
      javaCapabilities = {
        has_local_java_data: true,
        has_local_java_assets: true,
        java_version: savedVersion,
        jar_path: savedJar,
        minecraft_dir: savedDir,
      };
    }
  });

  // Save proxy setting when changed
  $effect(() => {
    localStorage.setItem("useSystemProxy", String(useSystemProxy));
  });

  async function selectMinecraftFolder() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Select your .minecraft folder",
      });

      if (selected) {
        javaLoading = true;
        javaError = null;
        selectedDir = selected as string;

        // Search for JARs in that directory
        const jars = await invoke<FoundJar[]>("search_for_client_jars", {
          path: selected,
        });

        if (jars.length === 0) {
          javaError = "No Minecraft client JARs found in that folder";
          foundJars = [];
        } else if (jars.length === 1) {
          // Only one JAR, auto-select it
          await selectJar(jars[0]);
        } else {
          // Multiple JARs, let user pick
          foundJars = jars;
        }
      }
    } catch (e) {
      console.error("Folder selection error:", e);
      javaError = String(e);
    } finally {
      javaLoading = false;
    }
  }

  async function selectJar(jar: FoundJar) {
    if (!jar.jar_path) return;

    // Validate the JAR
    try {
      const capabilities = await invoke<JavaCapabilities>("select_java_jar", {
        path: jar.jar_path,
      });

      javaCapabilities = {
        ...capabilities,
        minecraft_dir: selectedDir,
      };
      foundJars = [];

      // Save to localStorage
      if (selectedDir) {
        localStorage.setItem("selectedMinecraftDir", selectedDir);
      }
      localStorage.setItem("selectedJavaJar", jar.jar_path);
      if (capabilities.java_version) {
        localStorage.setItem("selectedJavaVersion", capabilities.java_version);
      }
    } catch (e) {
      javaError = String(e);
    }
  }

  async function clearAssetCache() {
    clearing = "assets";
    try {
      await invoke("clear_asset_cache");
      alert("Java asset cache cleared successfully!");
    } catch (e) {
      console.error("Clear asset cache error:", e);
      alert("Failed to clear asset cache: " + e);
    } finally {
      clearing = null;
    }
  }

  async function reloadAssets() {
    if (!javaCapabilities?.jar_path) return;

    clearing = "reloading";
    try {
      // Re-run extraction which now updates textures too
      await invoke("extract_all_block_colors", {
        overridePath: javaCapabilities.jar_path,
      });
      await invoke("extract_biome_data", {
        overridePath: javaCapabilities.jar_path,
      });
      alert(
        "Assets reloaded successfully! Textures and Biomes are now updated."
      );
      // Optional: Refresh local state if needed, though most is server-side
    } catch (e) {
      console.error("Reload assets error:", e);
      alert("Failed to reload assets: " + e);
    } finally {
      clearing = null;
    }
  }

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
      await invoke("clear_asset_cache");
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
      <h2>🎮 Java Assets</h2>
      <p class="section-description">
        Extract textures, language files, and advancement data from your local
        Minecraft installation for enhanced visuals.
      </p>

      {#if javaLoading}
        <div class="loading-state">
          <span class="spinner"></span>
          Scanning for Minecraft JARs...
        </div>
      {:else if foundJars.length > 0}
        <!-- Multiple JARs found, let user pick -->
        <div class="jar-picker">
          <h3>Select a Minecraft JAR</h3>
          <p>Found {foundJars.length} JAR files. Pick one:</p>
          <div class="jar-list">
            {#each foundJars as jar}
              <button class="jar-option" onclick={() => selectJar(jar)}>
                <span class="jar-version">{jar.id}</span>
                <span class="jar-path"
                  >{jar.jar_path?.split("/").slice(-2).join("/")}</span
                >
              </button>
            {/each}
          </div>
          <button class="action-btn secondary" onclick={() => (foundJars = [])}>
            Cancel
          </button>
        </div>
      {:else if javaError}
        <div class="error-banner">
          ⚠️ {javaError}
        </div>
        <div class="java-actions">
          <button class="action-btn primary" onclick={selectMinecraftFolder}>
            📁 Select .minecraft Folder
          </button>
        </div>
      {:else if javaCapabilities && javaCapabilities.has_local_java_assets}
        <div class="java-status">
          <div class="status-indicator active">
            <span class="status-dot success"></span>
            <span>Java assets configured</span>
          </div>
        </div>

        {#if javaCapabilities.minecraft_dir}
          <div class="info-item compact">
            <span class="info-label">Minecraft Folder</span>
            <span class="info-value path">{javaCapabilities.minecraft_dir}</span
            >
          </div>
        {/if}

        {#if javaCapabilities.java_version}
          <div class="info-item compact">
            <span class="info-label">Version (auto-selected)</span>
            <span class="info-value">{javaCapabilities.java_version}</span>
          </div>
        {/if}

        <div class="java-actions">
          <button class="action-btn secondary" onclick={selectMinecraftFolder}>
            📁 Change Folder
          </button>
          <button
            class="action-btn primary"
            onclick={reloadAssets}
            disabled={clearing === "reloading"}
          >
            {clearing === "reloading" ? "♻️ Reloading..." : "♻️ Reload Assets"}
          </button>
          <button
            class="action-btn danger"
            onclick={clearAssetCache}
            disabled={clearing === "assets"}
          >
            {clearing === "assets" ? "Clearing..." : "Clear Asset Cache"}
          </button>
        </div>
      {:else}
        <!-- No folder selected yet -->
        <div class="java-empty-state">
          <div class="empty-icon">🎮</div>
          <h3>No Minecraft Folder Selected</h3>
          <p>
            Select your .minecraft folder to enable texture extraction, language
            lookups, and advancement data.
          </p>
          <button class="action-btn primary" onclick={selectMinecraftFolder}>
            📁 Select .minecraft Folder
          </button>
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

  /* Java Assets Section Styles */
  .section-description {
    margin: 0 0 1.5rem 0;
    font-size: 0.875rem;
    color: var(--color-text-3);
  }

  .loading-state {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 1rem;
    background: var(--color-bg-3);
    border-radius: var(--radius-md);
    color: var(--color-text-2);
  }

  .spinner {
    width: 20px;
    height: 20px;
    border: 2px solid var(--color-bg-4);
    border-top-color: var(--color-primary);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .error-banner {
    padding: 1rem;
    background: rgba(239, 68, 68, 0.1);
    border-left: 3px solid #ef4444;
    border-radius: var(--radius-md);
    color: #ef4444;
    font-size: 0.875rem;
  }

  .java-status {
    margin-bottom: 1rem;
  }

  .status-indicator {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.875rem;
    color: var(--color-text-2);
  }

  .status-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
  }

  .status-dot.success {
    background: #22c55e;
    box-shadow: 0 0 8px rgba(34, 197, 94, 0.5);
  }

  .status-dot.warning {
    background: #f59e0b;
  }

  .info-item.compact {
    margin-bottom: 1rem;
    padding: 0.75rem 1rem;
    background: var(--color-bg-3);
    border-radius: var(--radius-md);
  }

  .info-value.path {
    font-family: ui-monospace, monospace;
    font-size: 0.75rem;
    word-break: break-all;
  }

  .version-select {
    padding: 0.5rem 1rem;
    background: var(--color-bg-3);
    border: 1px solid var(--color-bg-4);
    border-radius: var(--radius-md);
    color: var(--color-text-1);
    font-size: 0.875rem;
    cursor: pointer;
    min-width: 200px;
  }

  .version-select:focus {
    outline: none;
    border-color: var(--color-primary);
  }

  .java-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
    margin-top: 1.5rem;
  }

  .action-btn.secondary {
    background: var(--color-bg-4);
    color: var(--color-text-1);
    border: 1px solid var(--color-bg-4);
  }

  .action-btn.secondary:hover {
    background: var(--color-bg-3);
    border-color: var(--color-primary);
  }

  .action-btn.primary {
    background: var(--color-primary);
    color: white;
    border: 1px solid var(--color-primary);
  }

  .action-btn.primary:hover {
    background: var(--color-primary-dark, #2563eb);
  }

  .info-banner.warning {
    margin-top: 1.5rem;
    background: rgba(245, 158, 11, 0.1);
    border-left-color: #f59e0b;
    color: var(--color-text-2);
  }

  .java-empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    padding: 2rem;
    background: var(--color-bg-3);
    border-radius: var(--radius-lg);
    border: 2px dashed var(--color-bg-4);
  }

  .java-empty-state .empty-icon {
    font-size: 3rem;
    margin-bottom: 1rem;
  }

  .java-empty-state h3 {
    margin: 0 0 0.5rem 0;
    font-size: 1.125rem;
    color: var(--color-text-1);
  }

  .java-empty-state p {
    margin: 0 0 1.5rem 0;
    font-size: 0.875rem;
    color: var(--color-text-3);
    max-width: 400px;
  }

  .jar-picker {
    padding: 1.5rem;
    background: var(--color-bg-3);
    border-radius: var(--radius-lg);
  }

  .jar-picker h3 {
    margin: 0 0 0.5rem 0;
    font-size: 1rem;
    color: var(--color-text-1);
  }

  .jar-picker > p {
    margin: 0 0 1rem 0;
    font-size: 0.875rem;
    color: var(--color-text-3);
  }

  .jar-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    margin-bottom: 1rem;
    max-height: 300px;
    overflow-y: auto;
  }

  .jar-option {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.25rem;
    padding: 0.75rem 1rem;
    background: var(--color-bg-2);
    border: 1px solid var(--color-bg-4);
    border-radius: var(--radius-md);
    cursor: pointer;
    text-align: left;
    transition: all 0.15s;
  }

  .jar-option:hover {
    border-color: var(--color-primary);
    background: var(--color-bg-1);
  }

  .jar-version {
    font-weight: 600;
    color: var(--color-text-1);
  }

  .jar-path {
    font-size: 0.75rem;
    font-family: ui-monospace, monospace;
    color: var(--color-text-3);
  }
</style>
