<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { goto } from "$app/navigation";
  import { onMount } from "svelte";
  import type { WorldInfo } from "$lib/types";
  import WorldCard from "$lib/WorldCard.svelte";

  let worlds = $state<WorldInfo[]>([]);
  let customPath = $state("~");
  let maxDepth = $state(6);
  let scanZips = $state(true);
  let loading = $state(false);
  let errorMsg = $state("");
  let currentPath = $state("");
  let worldsFound = $state(0);
  let showSettings = $state(false);
  let scanningZip = $state(false);
  let totalItems = $state(0);
  let itemsProcessed = $state(0);
  let eta = $state("");
  let scanSpeed = $state(0);
  let currentPhase = $state("");
  let searchQuery = $state("");
  let scrolled = $state(false);

  // Interpolation for smooth counting
  let displayedTotalItems = $state(0);
  let targetTotalItems = $state(0);
  let interpolationInterval: number | null = null;

  // Filter worlds based on search
  $effect(() => {
    if (searchQuery) {
      // Filter is reactive based on searchQuery
    }
  });

  let filteredWorlds = $derived(
    worlds.filter(
      (world) =>
        world.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
        world.folder_name.toLowerCase().includes(searchQuery.toLowerCase())
    )
  );

  // Test that JavaScript is running
  console.log("Page loaded, customPath:", customPath);

  async function loadCachedWorlds() {
    try {
      const cached = await invoke<WorldInfo[]>("load_worlds_cache");
      if (cached.length > 0) {
        worlds = cached.sort((a, b) => b.last_played - a.last_played);
        console.log("Loaded", cached.length, "cached worlds");
      }
    } catch (e) {
      console.error("Failed to load cache:", e);
    }
  }

  async function saveCachedWorlds() {
    try {
      await invoke("save_worlds_cache", { worlds });
      console.log("Saved", worlds.length, "worlds to cache");
    } catch (e) {
      console.error("Failed to save cache:", e);
    }
  }

  // Smooth interpolation effect
  $effect(() => {
    if (targetTotalItems > displayedTotalItems) {
      if (interpolationInterval) {
        clearInterval(interpolationInterval);
      }

      interpolationInterval = setInterval(() => {
        const diff = targetTotalItems - displayedTotalItems;
        if (diff <= 0) {
          displayedTotalItems = targetTotalItems;
          if (interpolationInterval) {
            clearInterval(interpolationInterval);
            interpolationInterval = null;
          }
        } else {
          // Adaptive increment: faster for large gaps, smoother for small ones
          // Aim to close the gap in ~1 second (33 frames at 30ms each)
          const increment = Math.max(5, Math.ceil(diff / 20));
          displayedTotalItems = Math.min(
            displayedTotalItems + increment,
            targetTotalItems
          );
        }
      }, 30) as unknown as number;
    }

    return () => {
      if (interpolationInterval) {
        clearInterval(interpolationInterval);
      }
    };
  });

  onMount(() => {
    // Load cached worlds first
    loadCachedWorlds();

    // Scroll detection for compact header
    let scrollContainer: HTMLElement | null = null;
    const handleScroll = () => {
      if (scrollContainer) {
        scrolled = scrollContainer.scrollTop > 20;
      }
    };

    // Find the scroll container after mount
    setTimeout(() => {
      scrollContainer = document.querySelector(".page-root");
      if (scrollContainer) {
        scrollContainer.addEventListener("scroll", handleScroll);
      }
    }, 100);

    // Listen for scan phase changes
    const unlistenPhase = listen<string>("scan-phase", (event) => {
      currentPhase = event.payload;
    });

    // Listen for total items
    const unlistenTotal = listen<number>("scan-total", (event) => {
      totalItems = event.payload;
    });

    // Listen for scan progress
    const unlisten1 = listen<{
      current_path: string;
      worlds_found: number;
      items_processed: number;
      total_items: number;
      elapsed_seconds: number;
    }>("scan-progress", (event) => {
      currentPath = event.payload.current_path;
      worldsFound = event.payload.worlds_found;
      itemsProcessed = event.payload.items_processed;
      // During collection phase (itemsProcessed === 0), interpolate the count
      if (event.payload.items_processed === 0) {
        targetTotalItems = event.payload.total_items;
      } else {
        // During scanning phase, update directly
        totalItems = event.payload.total_items;
        displayedTotalItems = event.payload.total_items;
      }
      scanningZip = false;

      // Calculate scan speed
      if (event.payload.elapsed_seconds > 0) {
        scanSpeed = Math.round(
          event.payload.items_processed / event.payload.elapsed_seconds
        );
      }

      // Calculate ETA
      if (
        event.payload.items_processed > 0 &&
        event.payload.elapsed_seconds > 0
      ) {
        const itemsPerSec =
          event.payload.items_processed / event.payload.elapsed_seconds;
        const remainingItems =
          event.payload.total_items - event.payload.items_processed;
        const remainingSeconds = remainingItems / itemsPerSec;

        if (remainingSeconds < 60) {
          eta = `${Math.ceil(remainingSeconds)}s`;
        } else if (remainingSeconds < 3600) {
          eta = `${Math.ceil(remainingSeconds / 60)}m`;
        } else {
          eta = `${Math.ceil(remainingSeconds / 3600)}h`;
        }
      }
    });

    const unlisten2 = listen<number>("scan-complete", (event) => {
      console.log("Scan complete:", event.payload, "worlds");
      loading = false;
      scanningZip = false;
      // Auto-save after scan
      if (worlds.length > 0) {
        saveCachedWorlds();
      }
    });

    const unlisten3 = listen<string>("scanning-zip", (event) => {
      currentPath = `🗜️ ${event.payload}`;
      scanningZip = true;
    });

    const unlisten4 = listen("scan-cancelled", () => {
      console.log("Scan cancelled");
      loading = false;
      scanningZip = false;
    });

    return () => {
      unlistenPhase.then((fn) => fn());
      unlistenTotal.then((fn) => fn());
      unlisten1.then((fn) => fn());
      unlisten2.then((fn) => fn());
      unlisten3.then((fn) => fn());
      unlisten4.then((fn) => fn());
    };
  });

  async function scanWorlds() {
    console.log(
      "scanWorlds called with path:",
      customPath,
      "maxDepth:",
      maxDepth
    );
    if (!customPath) return;
    loading = true;
    errorMsg = "";
    worlds = [];
    currentPath = "";
    worldsFound = 0;
    scanningZip = false;
    totalItems = 0;
    itemsProcessed = 0;
    eta = "";
    displayedTotalItems = 0;
    targetTotalItems = 0;

    try {
      console.log("Invoking scan_worlds...");
      const res = await invoke<WorldInfo[]>("scan_worlds", {
        path: customPath,
        maxDepth: maxDepth,
        scanZips: scanZips,
      });
      console.log("Scan complete, found", res.length, "worlds");
      worlds = res.sort((a, b) => b.last_played - a.last_played);
      // Auto-save
      if (worlds.length > 0) {
        saveCachedWorlds();
      }
    } catch (e) {
      console.error("Scan error:", e);
      errorMsg = String(e);
    } finally {
      loading = false;
      scanningZip = false;
    }
  }

  async function cancelScan() {
    await invoke("cancel_scan");
  }

  async function skipZip() {
    await invoke("skip_zip");
    scanningZip = false;
  }
</script>

<div class="page-root">
  <div class="library-layout">
    <!-- Sidebar -->
    <aside class="library-sidebar">
      <nav class="sidebar-nav">
        <!-- Quick Actions -->
        <div class="nav-section">
          <span class="nav-section-title">Quick Actions</span>
          <button class="nav-item" onclick={() => (showSettings = true)}>
            <span class="nav-icon">➕</span>
            <span class="nav-label">Add Worlds</span>
          </button>
          <a href="/nbt" class="nav-item">
            <span class="nav-icon">📝</span>
            <span class="nav-label">NBT Editor</span>
          </a>
          <button class="nav-item" disabled>
            <span class="nav-icon">🔄</span>
            <span class="nav-label">Rescan</span>
            <span class="nav-badge">Soon</span>
          </button>
        </div>

        <!-- Filters -->
        <div class="nav-section">
          <span class="nav-section-title">Filters</span>
          <button class="nav-item active">
            <span class="nav-icon">🌍</span>
            <span class="nav-label">All Worlds</span>
            <span class="nav-count">{worlds.length}</span>
          </button>
          <button class="nav-item" disabled>
            <span class="nav-icon">☕</span>
            <span class="nav-label">Java Edition</span>
            <span class="nav-badge">Soon</span>
          </button>
          <button class="nav-item" disabled>
            <span class="nav-icon">🛏️</span>
            <span class="nav-label">Bedrock Edition</span>
            <span class="nav-badge">Soon</span>
          </button>
          <button class="nav-item" disabled>
            <span class="nav-icon">⭐</span>
            <span class="nav-label">Favorites</span>
            <span class="nav-badge">v2</span>
          </button>
        </div>

        <!-- Recent Worlds -->
        {#if worlds.length > 0}
          <div class="nav-section">
            <span class="nav-section-title">Recent Worlds</span>
            {#each worlds.slice(0, 5) as world (world.id)}
              <a
                href="/world/{world.id}"
                class="nav-item nav-world-item"
                title={world.name}
              >
                {#if world.icon}
                  <img
                    src="data:image/png;base64,{world.icon}"
                    alt=""
                    class="nav-world-icon"
                  />
                {:else}
                  <span class="nav-world-icon">🌍</span>
                {/if}
                <span class="nav-label truncate">{world.name}</span>
              </a>
            {/each}
          </div>
        {/if}
      </nav>
    </aside>

    <!-- Main Content -->
    <main class="library-main">
      <!-- Tahoe-style translucent header -->
      <header class="library-header" class:compact={scrolled}>
        <div class="header-content">
          <div class="header-top">
            <div class="title-section">
              <h1 class="library-title">World Library</h1>
              {#if worlds.length > 0}
                <span class="world-count"
                  >{worlds.length} world{worlds.length !== 1 ? "s" : ""}</span
                >
              {/if}
            </div>

            <button
              class="add-worlds-btn"
              onclick={() => (showSettings = !showSettings)}
            >
              <span class="btn-icon">➕</span>
              <span>Add Worlds</span>
            </button>
          </div>

          {#if worlds.length > 0}
            <!-- macOS-style search bar -->
            <div class="search-container">
              <span class="search-icon">🔍</span>
              <input
                type="text"
                bind:value={searchQuery}
                placeholder="Search worlds..."
                class="search-input"
              />
              {#if searchQuery}
                <button class="clear-search" onclick={() => (searchQuery = "")}>
                  ✕
                </button>
              {/if}
            </div>
          {/if}
        </div>

        <!-- Collapsible Add Worlds Panel -->
        {#if showSettings}
          <div class="add-panel">
            <div class="panel-content">
              <div class="input-group">
                <label class="input-label">Scan Location</label>
                <input
                  type="text"
                  bind:value={customPath}
                  placeholder="/Users/you/Documents"
                  class="path-input"
                />
              </div>

              <div class="settings-row">
                <div class="setting-item">
                  <label class="setting-label">
                    Search Depth: <strong>{maxDepth}</strong>
                  </label>
                  <input
                    type="range"
                    bind:value={maxDepth}
                    min="3"
                    max="15"
                    class="depth-slider"
                  />
                  <span class="hint-text">
                    Lower = faster, Higher = finds buried worlds
                  </span>
                </div>

                <label class="checkbox-container">
                  <input type="checkbox" bind:checked={scanZips} />
                  <span class="checkbox-label">Scan .zip files</span>
                  <span class="checkbox-hint">(slower)</span>
                </label>
              </div>

              <button onclick={scanWorlds} disabled={loading} class="scan-btn">
                {#if loading}
                  <span class="btn-spinner">⏳</span>
                  <span>Scanning...</span>
                {:else}
                  <span class="btn-icon">🔍</span>
                  <span>Start Scan</span>
                {/if}
              </button>
            </div>
          </div>
        {/if}

        <!-- Scanning Progress -->
        {#if loading}
          <div class="progress-container">
            <div class="progress-info">
              <div class="progress-text">
                <span class="current-path">{currentPath}</span>
                <span class="progress-stats">
                  {#if itemsProcessed === 0}
                    {displayedTotalItems.toLocaleString()} items found
                  {:else}
                    {worldsFound} worlds • {itemsProcessed.toLocaleString()} / {totalItems.toLocaleString()}
                    items
                    {#if eta}• {eta} left{/if}
                  {/if}
                </span>
              </div>
              <div class="progress-actions">
                {#if scanningZip}
                  <button onclick={skipZip} class="action-btn skip"
                    >Skip Zip</button
                  >
                {/if}
                <button onclick={cancelScan} class="action-btn cancel"
                  >Cancel</button
                >
              </div>
            </div>
            <div class="progress-track">
              <div
                class="progress-fill"
                style="width: {totalItems > 0
                  ? (itemsProcessed / totalItems) * 100
                  : 0}%"
              ></div>
            </div>
          </div>
        {/if}
      </header>

      <!-- World Library Content -->
      <section class="library-content">
        {#if errorMsg}
          <div class="error-banner">
            <span class="error-icon">⚠️</span>
            <p>{errorMsg}</p>
          </div>
        {/if}

        {#if filteredWorlds.length > 0}
          <div class="world-library">
            <div class="world-grid">
              {#each filteredWorlds as world, i (world.id)}
                <div
                  class="world-card-wrapper"
                  style="animation-delay: {i * 0.05}s"
                  onclick={() => goto(`/world/${world.id}`)}
                  role="button"
                  tabindex="0"
                >
                  <WorldCard {world} />
                </div>
              {/each}
            </div>
          </div>
        {:else if worlds.length > 0}
          <!-- No search results -->
          <div class="empty-state">
            <div class="empty-icon">🔍</div>
            <h2>No worlds found</h2>
            <p>Try a different search term</p>
          </div>
        {:else if !loading}
          <!-- Empty library state -->
          <div class="empty-state">
            <div class="empty-icon">🌍</div>
            <h2>Your World Library is Empty</h2>
            <p>Get started by scanning for Minecraft worlds on your system</p>
            <button
              class="empty-action-btn"
              onclick={() => (showSettings = true)}
            >
              <span>➕</span>
              Add Worlds
            </button>
          </div>
        {/if}
      </section>
    </main>
  </div>
</div>

<style>
  /* macOS Tahoe - World Library Styles */

  /* Scrollable Page Container */
  .page-root {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }

  /* Library Layout with Sidebar */
  .library-layout {
    display: flex;
    min-height: 100vh;
  }

  .library-sidebar {
    width: 240px;
    background: var(--color-bg-2);
    border-right: 1px solid var(--color-bg-3);
    overflow-y: auto;
    flex-shrink: 0;
    position: sticky;
    top: 0;
    height: 100vh;
  }

  .sidebar-nav {
    padding: 1.5rem 0;
  }

  .nav-section {
    margin-bottom: 2rem;
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
    transition: all 0.2s;
    position: relative;
    overflow: hidden; /* Added to constrain overflow */
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

  .nav-item:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .nav-icon {
    font-size: 1.125rem;
    width: 1.5rem;
    text-align: center;
    flex-shrink: 0;
  }

  .nav-label {
    flex: 1;
    min-width: 0; /* Added to allow truncation */
    text-align: left;
    font-size: 0.875rem;
    font-weight: 500;
  }

  .nav-label.truncate {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .nav-badge {
    font-size: 0.625rem;
    padding: 0.125rem 0.375rem;
    background: var(--color-bg-4);
    border-radius: var(--radius-sm);
    font-weight: 600;
    text-transform: uppercase;
    flex-shrink: 0;
  }

  .nav-count {
    font-size: 0.75rem;
    padding: 0.125rem 0.5rem;
    background: var(--color-bg-4);
    border-radius: var(--radius-sm);
    font-weight: 600;
    flex-shrink: 0;
  }

  .nav-item.active .nav-badge,
  .nav-item.active .nav-count {
    background: rgba(255, 255, 255, 0.2);
  }

  .nav-world-item {
    padding: 0.625rem 1.5rem;
  }

  .nav-world-icon {
    width: 24px;
    height: 24px;
    border-radius: var(--radius-sm);
    object-fit: cover;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--color-bg-3);
    font-size: 0.875rem;
  }

  .library-main {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 100vh;
  }

  /* ===== HEADER ===== */

  .library-header {
    background: var(--material-bg-2);
    -webkit-backdrop-filter: blur(30px) saturate(180%);
    backdrop-filter: blur(30px) saturate(180%);
    border-bottom: 1px solid rgba(0, 0, 0, 0.06);
    position: sticky;
    top: 0;
    z-index: 100;
    box-shadow: var(--shadow-sm);
  }

  @media (prefers-color-scheme: dark) {
    .library-header {
      border-bottom-color: rgba(255, 255, 255, 0.06);
    }
  }

  .header-content {
    max-width: 1400px;
    margin: 0 auto;
    padding: 1.5rem 2rem;
  }

  .header-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1.25rem;
    transition: margin 0.2s ease;
  }

  .library-header.compact .header-top {
    margin-bottom: 0.75rem;
  }

  .library-header {
    transition: all 0.2s ease;
  }

  .library-header.compact .header-content {
    padding: 1rem 2rem;
  }

  .library-header.compact .library-title {
    font-size: 1.5rem;
  }

  .library-header.compact .world-count {
    font-size: 0.875rem;
    padding: 0.25rem 0.75rem;
  }

  .library-header.compact .search-container {
    max-width: 500px;
  }

  .title-section {
    display: flex;
    align-items: baseline;
    gap: 1rem;
  }

  .library-title {
    margin: 0;
    font-size: 2rem;
    font-weight: 700;
    color: var(--color-text-1);
    letter-spacing: -0.03em;
  }

  .world-count {
    font-size: 1rem;
    font-weight: 500;
    color: var(--color-text-3);
    padding: 0.375rem 0.875rem;
    background: var(--color-bg-3);
    border-radius: var(--radius-lg);
  }

  /* Tahoe Button - Add Worlds */
  .add-worlds-btn {
    display: flex;
    align-items: center;
    gap: 0.625rem;
    padding: 0.75rem 1.25rem;
    background: var(--color-primary);
    color: white;
    border: none;
    border-radius: var(--radius-lg);
    font-size: 0.9375rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
    box-shadow: var(--shadow-sm);
    letter-spacing: -0.01em;
  }

  .add-worlds-btn:hover {
    background: var(--color-primary-hover);
    transform: translateY(-1px);
    box-shadow: var(--shadow-md);
  }

  .add-worlds-btn:active {
    transform: translateY(0);
  }

  .btn-icon {
    font-size: 1.125rem;
    line-height: 1;
  }

  /* macOS Search Bar */
  .search-container {
    position: relative;
    max-width: 600px;
  }

  .search-icon {
    position: absolute;
    left: 1rem;
    top: 50%;
    transform: translateY(-50%);
    font-size: 1.125rem;
    opacity: 0.5;
    pointer-events: none;
  }

  .search-input {
    width: 100%;
    padding: 0.875rem 1rem 0.875rem 3rem;
    background: var(--color-bg-2);
    border: 1px solid var(--color-bg-3);
    border-radius: var(--radius-lg);
    font-size: 1rem;
    color: var(--color-text-1);
    transition: all 0.2s;
  }

  .search-input:focus {
    outline: none;
    background: var(--color-bg-1);
    border-color: var(--color-primary);
    box-shadow: 0 0 0 4px var(--color-primary-light);
  }

  .search-input::placeholder {
    color: var(--color-text-4);
  }

  .clear-search {
    position: absolute;
    right: 0.75rem;
    top: 50%;
    transform: translateY(-50%);
    width: 1.5rem;
    height: 1.5rem;
    background: var(--color-bg-3);
    border: none;
    border-radius: 50%;
    cursor: pointer;
    color: var(--color-text-3);
    font-size: 0.875rem;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s;
  }

  .clear-search:hover {
    background: var(--color-bg-4);
    color: var(--color-text-1);
  }

  /* Add Panel */
  .add-panel {
    background: var(--color-bg-2);
    border-top: 1px solid var(--color-bg-3);
    animation: slideDown 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  }

  @keyframes slideDown {
    from {
      opacity: 0;
      transform: translateY(-10px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .panel-content {
    max-width: 1400px;
    margin: 0 auto;
    padding: 2rem;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  .input-group {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .input-label {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--color-text-2);
  }

  .path-input {
    padding: 0.875rem 1rem;
    background: var(--color-bg-1);
    border: 1px solid var(--color-bg-3);
    border-radius: var(--radius-lg);
    font-size: 0.9375rem;
    color: var(--color-text-1);
    font-family: "SF Mono", "Monaco", monospace;
    transition: all 0.2s;
  }

  .path-input:focus {
    outline: none;
    border-color: var(--color-primary);
    box-shadow: 0 0 0 4px var(--color-primary-light);
  }

  .settings-row {
    display: flex;
    gap: 2rem;
    align-items: flex-start;
    flex-wrap: wrap;
  }

  .setting-item {
    flex: 1;
    min-width: 250px;
    display: flex;
    flex-direction: column;
    gap: 0.625rem;
  }

  .setting-label {
    font-size: 0.875rem;
    color: var(--color-text-2);
    font-weight: 500;
  }

  .depth-slider {
    width: 100%;
    height: 6px;
    border-radius: 3px;
    background: var(--color-bg-3);
    outline: none;
    -webkit-appearance: none;
    appearance: none;
  }

  .depth-slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--color-primary);
    cursor: pointer;
    box-shadow: var(--shadow-sm);
    transition: all 0.2s;
  }

  .depth-slider::-webkit-slider-thumb:hover {
    transform: scale(1.1);
    box-shadow: var(--shadow-md);
  }

  .depth-slider::-moz-range-thumb {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--color-primary);
    cursor: pointer;
    border: none;
    box-shadow: var(--shadow-sm);
  }

  .hint-text {
    font-size: 0.8125rem;
    color: var(--color-text-3);
  }

  .checkbox-container {
    display: flex;
    align-items: center;
    gap: 0.625rem;
    cursor: pointer;
    padding: 0.75rem;
    background: var(--color-bg-1);
    border-radius: var(--radius-md);
    transition: background 0.2s;
  }

  .checkbox-container:hover {
    background: var(--color-bg-3);
  }

  .checkbox-container input[type="checkbox"] {
    width: 1.25rem;
    height: 1.25rem;
    cursor: pointer;
    accent-color: var(--color-primary);
  }

  .checkbox-label {
    font-size: 0.9375rem;
    font-weight: 500;
    color: var(--color-text-1);
  }

  .checkbox-hint {
    font-size: 0.8125rem;
    color: var(--color-text-3);
  }

  .scan-btn {
    align-self: flex-start;
    display: flex;
    align-items: center;
    gap: 0.625rem;
    padding: 0.875rem 1.75rem;
    background: var(--color-primary);
    color: white;
    border: none;
    border-radius: var(--radius-lg);
    font-size: 1rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
    box-shadow: var(--shadow-md);
  }

  .scan-btn:hover:not(:disabled) {
    background: var(--color-primary-hover);
    transform: translateY(-2px);
    box-shadow: var(--shadow-lg);
  }

  .scan-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .btn-spinner {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  /* Progress Container */
  .progress-container {
    background: var(--color-bg-2);
    border-top: 1px solid var(--color-bg-3);
    padding: 1.25rem 2rem;
  }

  .progress-info {
    max-width: 1400px;
    margin: 0 auto 0.875rem;
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
  }

  .progress-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
  }

  .current-path {
    font-size: 0.875rem;
    font-weight: 500;
    color: var(--color-text-1);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    display: block;
    max-width: 100%;
  }

  .progress-stats {
    font-size: 0.8125rem;
    color: var(--color-text-3);
  }

  .progress-actions {
    display: flex;
    gap: 0.625rem;
    flex-shrink: 0;
  }

  .action-btn {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: var(--radius-md);
    font-size: 0.875rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
  }

  .action-btn.skip {
    background: var(--color-orange);
    color: white;
  }

  .action-btn.skip:hover {
    filter: brightness(0.9);
  }

  .action-btn.cancel {
    background: var(--color-red);
    color: white;
  }

  .action-btn.cancel:hover {
    filter: brightness(0.9);
  }

  .progress-track {
    max-width: 1400px;
    margin: 0 auto;
    height: 6px;
    background: var(--color-bg-3);
    border-radius: 3px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: linear-gradient(90deg, var(--color-primary), var(--color-teal));
    border-radius: 3px;
    transition: width 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  }

  /* ===== LIBRARY CONTENT ===== */

  .library-content {
    flex: 1;
    max-width: 1400px;
    margin: 0 auto;
    padding: 2.5rem 2rem;
    width: 100%;
  }

  /* Error Banner */
  .error-banner {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    background: rgba(255, 59, 48, 0.1);
    color: var(--color-red);
    padding: 1rem 1.25rem;
    border-radius: var(--radius-lg);
    margin-bottom: 2rem;
    border: 1px solid rgba(255, 59, 48, 0.2);
  }

  .error-icon {
    font-size: 1.5rem;
    flex-shrink: 0;
  }

  .error-banner p {
    margin: 0;
    font-weight: 500;
  }

  /* World Grid */
  .world-library {
    animation: fadeIn 0.4s ease-out;
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
      transform: translateY(10px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .world-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(400px, 1fr));
    gap: 1.5rem;
  }

  @media (max-width: 500px) {
    .world-grid {
      grid-template-columns: 1fr;
    }
  }

  .world-card-wrapper {
    opacity: 0;
    animation: cardFadeIn 0.4s ease-out forwards;
  }

  @keyframes cardFadeIn {
    from {
      opacity: 0;
      transform: translateY(20px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  /* Empty State */
  .empty-state {
    text-align: center;
    padding: 6rem 2rem;
    animation: fadeIn 0.5s ease-out;
  }

  .empty-icon {
    font-size: 5rem;
    margin-bottom: 1.5rem;
    opacity: 0.4;
  }

  .empty-state h2 {
    margin: 0 0 0.75rem;
    font-size: 1.75rem;
    font-weight: 700;
    color: var(--color-text-1);
    letter-spacing: -0.02em;
  }

  .empty-state p {
    margin: 0 0 2rem;
    font-size: 1.0625rem;
    color: var(--color-text-3);
    max-width: 400px;
    margin-left: auto;
    margin-right: auto;
  }

  .empty-action-btn {
    display: inline-flex;
    align-items: center;
    gap: 0.625rem;
    padding: 1rem 2rem;
    background: var(--color-primary);
    color: white;
    border: none;
    border-radius: var(--radius-lg);
    font-size: 1.0625rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    box-shadow: var(--shadow-md);
  }

  .empty-action-btn:hover {
    background: var(--color-primary-hover);
    transform: translateY(-2px);
    box-shadow: var(--shadow-lg);
  }

  .empty-action-btn span {
    font-size: 1.25rem;
  }

  /* Responsive */
  @media (max-width: 768px) {
    .header-content {
      padding: 1.25rem 1.5rem;
    }

    .header-top {
      flex-direction: column;
      align-items: flex-start;
      gap: 1rem;
    }

    .library-title {
      font-size: 1.5rem;
    }

    .search-container {
      width: 100%;
    }

    .panel-content {
      padding: 1.5rem;
    }

    .settings-row {
      flex-direction: column;
      gap: 1.5rem;
    }

    .library-content {
      padding: 2rem 1.5rem;
    }

    .world-grid {
      grid-template-columns: 1fr;
      gap: 1.25rem;
    }
  }
</style>
