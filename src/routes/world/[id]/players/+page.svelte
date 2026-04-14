<script lang="ts">
  import { page } from "$app/stores";
  import { goto } from "$app/navigation";
  import { invoke } from "@tauri-apps/api/core";
  import type { WorldInfo } from "$lib/types";
  import { onMount } from "svelte";
  import { PaneGroup, Pane, PaneResizer } from "paneforge";

  // Components
  import PlayerDetailsCard from "./PlayerDetailsCard.svelte";

  // Types & Utils
  import type {
    PlayerData,
    FullPlayerData,
    BackendAdvancementCategory,
  } from "./types";
  import type { InventoryItem } from "$lib/components/player/ItemSlot.svelte";
  import type { PlayerStats } from "$lib/components/player/StatsEditor.svelte";
  import type { PotionEffect } from "$lib/components/player/EffectsPanel.svelte";
  import type { StatCategory } from "$lib/components/player/StatisticsPanel.svelte";
  import type { TabId } from "./tabs/tabs";
  import {
    getInventoryItems,
    getHotbarItems,
    getArmorItems,
    getOffhandItem,
    getEnderItems,
    getActiveEffects,
    buildPlayerStats,
  } from "./utils";

  // State
  let world = $state<WorldInfo | null>(null);
  let loading = $state(true);
  let error = $state("");
  let selectedPlayer = $state<string | null>(null);

  // Tab System
  let activeTab = $state<TabId>("inventory");

  // Player Data Maps
  let players = $state<PlayerData[]>([]);
  let playerDetails = $state<Map<string, PlayerData>>(new Map());
  let fullPlayerData = $state<Map<string, FullPlayerData>>(new Map());
  let playerAdvancements = $state<Map<string, BackendAdvancementCategory[]>>(
    new Map()
  );
  let loadingDetails = $state<Set<string>>(new Set());
  let playerHeads = $state<Map<string, string>>(new Map());
  let playerNames = $state<Map<string, string>>(new Map());

  // Derived Values
  // Derived Values
  let currentPlayer = $derived(
    selectedPlayer
      ? (playerDetails.get(selectedPlayer) ??
          players.find((p) => p.primary_id === selectedPlayer) ??
          null)
      : null
  );

  let currentFullData = $derived(
    selectedPlayer ? (fullPlayerData.get(selectedPlayer) ?? null) : null
  );

  // Mock data (keep for now as fallbacks)
  let mockInventory = $state<(InventoryItem | null)[]>(
    new Array(27).fill(null)
  );
  let mockHotbar = $state<(InventoryItem | null)[]>(new Array(9).fill(null));
  let mockArmor = $state<(InventoryItem | null)[]>(new Array(4).fill(null));
  let mockOffhand = $state<InventoryItem | null>(null);
  let mockEnderChest = $state<(InventoryItem | null)[]>(
    new Array(27).fill(null)
  );
  let mockEffects = $state<PotionEffect[]>([]);
  let mockStatistics = $state<StatCategory[]>([]); // Initialize empty or with mock data if needed

  // Actions
  function handleInventoryChange(slot: number, item: InventoryItem | null) {
    console.log("Inventory changed:", slot, item);
  }

  function handleStatsChange(field: string, value: unknown) {
    console.log("Stats changed:", field, value);
  }

  async function loadWorld() {
    loading = true;
    error = "";

    try {
      const cached = await invoke<WorldInfo[]>("load_worlds_cache");
      const id = $page.params.id;
      world = cached.find((w) => w.id === id) || null;

      if (!world) {
        error = "World not found";
      } else if (world.is_zip) {
        error = "Cannot view players in zipped worlds";
      } else {
        try {
          const playerData = await invoke<PlayerData[]>("get_players", {
            worldPath: world.path,
          });
          players = playerData;

          players.forEach((player) => {
            if (player.edition === "java") {
              loadPlayerName(player.primary_id);
            }
          });
        } catch (e) {
          console.error("Failed to load players:", e);
          players = [];
        }
      }
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function loadPlayerHead(uuid: string) {
    if (playerHeads.has(uuid)) return;
    try {
      const dataUrl = await invoke<string>("fetch_player_avatar", {
        uuid,
        avatarType: "head",
      });
      playerHeads.set(uuid, dataUrl);
      playerHeads = new Map(playerHeads);
    } catch (e) {
      console.error("Failed to load player head:", e);
    }
  }

  async function loadPlayerName(uuid: string) {
    // ... (skin loading logic for list items kept here)
    // Note: The PlayerDetailsCard has its own skin logic for the 3D viewer,
    // but we can keep list item logic here.
    if (playerNames.has(uuid)) return;

    try {
      const data = await invoke<any>("fetch_player_profile", { uuid });
      if (data.name) {
        playerNames.set(uuid, data.name);
        playerNames = new Map(playerNames);
      }

      // Also fetch head for list
      if (data.properties) {
        // ... existing logic to fetch head ...
        const texturesProp = data.properties.find(
          (p: any) => p.name === "textures"
        );
        if (texturesProp && texturesProp.value) {
          const decoded = atob(texturesProp.value);
          const textures = JSON.parse(decoded);
          if (textures.textures?.SKIN?.url) {
            const dataUrl = await invoke<string>("fetch_image", {
              url: textures.textures.SKIN.url,
            });
            const faceDataUrl = await cropSkinToFace(dataUrl);
            playerHeads.set(uuid, faceDataUrl);
            playerHeads = new Map(playerHeads);
          }
        }
      }
    } catch (e) {
      console.error("Failed to load player name:", e);
    }
  }

  async function cropSkinToFace(skinDataUrl: string): Promise<string> {
    return new Promise((resolve) => {
      const img = new Image();
      img.onload = () => {
        const canvas = document.createElement("canvas");
        const ctx = canvas.getContext("2d")!;
        canvas.width = 8;
        canvas.height = 8;
        ctx.drawImage(img, 8, 8, 8, 8, 0, 0, 8, 8);
        ctx.drawImage(img, 40, 8, 8, 8, 0, 0, 8, 8);
        resolve(canvas.toDataURL());
      };
      img.src = skinDataUrl;
    });
  }

  async function loadPlayerDetails(uuid: string) {
    if (fullPlayerData.has(uuid) || loadingDetails.has(uuid) || !world) {
      return;
    }

    loadingDetails.add(uuid);
    loadingDetails = new Set(loadingDetails);

    try {
      const fullData = await invoke<FullPlayerData>("get_player_full", {
        worldPath: world.path,
        uuid: uuid,
      });
      fullPlayerData.set(uuid, fullData);
      fullPlayerData = new Map(fullPlayerData);

      playerDetails.set(uuid, {
        edition: fullData.edition,
        primary_id: fullData.primary_id,
        display_name: fullData.display_name,
        position: fullData.position,
        health: fullData.health,
        food_level: fullData.food_level,
        xp_level: fullData.xp_level,
        xp_total: fullData.xp_total,
        game_mode: fullData.game_mode,
        dimension: fullData.dimension,
      });
      playerDetails = new Map(playerDetails);
    } catch (e) {
      console.error("Failed to load player details:", e);
    } finally {
      loadingDetails.delete(uuid);
      loadingDetails = new Set(loadingDetails);
    }
  }

  async function loadPlayerAdvancements(uuid: string) {
    if (playerAdvancements.has(uuid) || !world) return;

    try {
      const categories = await invoke<BackendAdvancementCategory[]>(
        "get_player_advancements",
        {
          worldPath: world.path,
          uuid: uuid,
        }
      );
      playerAdvancements.set(uuid, categories);
      playerAdvancements = new Map(playerAdvancements);
    } catch (e) {
      console.error("Failed to load player advancements:", e);
    }
  }

  $effect(() => {
    if (selectedPlayer) {
      loadPlayerDetails(selectedPlayer);
      loadPlayerAdvancements(selectedPlayer);
    }
  });

  onMount(() => {
    loadWorld();
  });

  $effect(() => {
    if ($page.params.id) {
      loadWorld();
    }
  });

  function goBack() {
    goto(`/world/${world?.id}`);
  }
</script>

{#if loading}
  <div class="loading">
    <div class="spinner"></div>
    <p>Loading players...</p>
  </div>
{:else if error || !world}
  <div class="error-page">
    <h1>⚠️ Error</h1>
    <p>{error || "World not found"}</p>
    <button onclick={goBack}>Back to Overview</button>
  </div>
{:else}
  <div class="players-page">
    {#if players.length === 0}
      <div class="empty-state">
        <div class="empty-icon">👥</div>
        <h2>No Player Data Found</h2>
        <p>
          {#if world.platform.toLowerCase().includes("java")}
            Player data is stored in the <code>playerdata</code> folder when players
            join.
          {:else}
            Bedrock player data will appear here once available.
          {/if}
        </p>
      </div>
    {:else}
      <PaneGroup direction="horizontal" class="players-pane-group">
        <Pane
          defaultSize={25}
          minSize={15}
          maxSize={40}
          class="players-list-pane"
        >
          <div class="players-list">
            <div class="list-header">
              <h3>Players ({players.length})</h3>
            </div>
            <div class="players-scroll">
              {#each players as player (player.primary_id)}
                <button
                  class="player-item"
                  class:active={selectedPlayer === player.primary_id}
                  onclick={() => (selectedPlayer = player.primary_id)}
                >
                  <div class="player-avatar">
                    {#if playerHeads.has(player.primary_id)}
                      <img
                        src={playerHeads.get(player.primary_id)}
                        alt="Player head"
                        class="player-head"
                        crossorigin="anonymous"
                      />
                    {:else}
                      <div class="player-head-placeholder">
                        {player.edition === "java" ? "☕" : "🛏️"}
                      </div>
                    {/if}
                  </div>
                  <div class="player-info">
                    <span class="player-name">
                      {playerNames.get(player.primary_id) ||
                        player.display_name ||
                        player.primary_id.substring(0, 8) + "..."}
                    </span>
                    <span class="player-meta">
                      <span class="edition-badge {player.edition}"
                        >{player.edition.toUpperCase()}</span
                      >
                    </span>
                  </div>
                </button>
              {/each}
            </div>
          </div>
        </Pane>

        <PaneResizer class="pane-resizer">
          <div class="resizer-handle"></div>
        </PaneResizer>

        <Pane defaultSize={75} minSize={50} class="player-details-pane">
          <main class="player-details">
            {#if !selectedPlayer}
              <div class="no-selection">
                <p>Select a player to view details</p>
              </div>
            {:else if loadingDetails.has(selectedPlayer)}
              <div class="loading-details">
                <div class="spinner"></div>
                <p>Loading player details...</p>
              </div>
            {:else if currentPlayer}
              <PlayerDetailsCard
                player={currentPlayer}
                fullData={currentFullData}
                {activeTab}
                onTabChange={(v) => (activeTab = v)}
                inventory={currentFullData
                  ? getInventoryItems(currentFullData)
                  : mockInventory}
                hotbar={currentFullData
                  ? getHotbarItems(currentFullData)
                  : mockHotbar}
                armor={currentFullData
                  ? getArmorItems(currentFullData)
                  : mockArmor}
                offhand={currentFullData
                  ? getOffhandItem(currentFullData)
                  : mockOffhand}
                ender={currentFullData
                  ? getEnderItems(currentFullData)
                  : mockEnderChest}
                stats={currentFullData
                  ? {
                      health: currentFullData.health,
                      maxHealth: currentFullData.max_health,
                      foodLevel: currentFullData.food_level,
                      saturation: currentFullData.saturation,
                      xpProgress: currentFullData.xp_progress,
                      xpLevel: currentFullData.xp_level,
                      xpTotal: currentFullData.xp_total,
                      gameMode: currentFullData.game_mode,
                      position: currentFullData.position,
                      dimension: currentFullData.dimension,
                    }
                  : buildPlayerStats(currentPlayer)}
                effects={currentFullData
                  ? getActiveEffects(currentFullData)
                  : mockEffects}
                advancements={playerAdvancements.get(selectedPlayer) || []}
                statistics={mockStatistics}
                onInventoryChange={handleInventoryChange}
                onStatsChange={handleStatsChange}
              />
            {/if}
          </main>
        </Pane>
      </PaneGroup>
    {/if}
  </div>
{/if}

<style>
  /* Keep existing styles */
  .loading,
  .error-page,
  .loading-details {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: calc(100vh - 200px);
    text-align: center;
    padding: 2rem;
  }

  .loading-details {
    min-height: 300px;
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

  .error-page button {
    margin-top: 2rem;
    padding: 0.75rem 1.5rem;
    background: var(--color-primary);
    color: white;
    border: none;
    border-radius: var(--radius-md);
    cursor: pointer;
  }

  .players-page {
    padding: 2rem;
    max-width: 1400px;
    margin: 0 auto;
    height: 100vh; /* Full viewport height */
    display: flex;
    flex-direction: column;
    overflow: hidden; /* No scroll here */
    min-height: 0;
  }

  .empty-state {
    text-align: center;
    padding: 4rem 2rem;
    max-width: 600px;
    margin: 0 auto;
  }

  .empty-icon {
    font-size: 4rem;
    margin-bottom: 1rem;
    opacity: 0.5;
  }

  .empty-state h2 {
    margin: 0 0 1rem 0;
    font-size: 1.5rem;
  }

  .empty-state p {
    color: var(--color-text-3);
    margin-bottom: 2rem;
  }

  .empty-state code {
    background: var(--color-bg-3);
    padding: 0.25rem 0.5rem;
    border-radius: var(--radius-sm);
    font-family: "SF Mono", monospace;
  }

  /* Pane layout */
  :global(.players-pane-group) {
    display: flex !important;
    flex: 1;
    min-height: 0;
    height: 100%;
  }

  :global(.players-list-pane) {
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  :global(.player-details-pane) {
    display: flex;
    flex-direction: column;
    overflow: hidden !important; /* Critical: NO SCROLL */
    min-height: 0;
  }

  :global(.pane-resizer) {
    width: 8px;
    background: transparent;
    position: relative;
    cursor: col-resize;
    transition: background 0.15s ease;
  }

  :global(.pane-resizer:hover),
  :global(.pane-resizer[data-resizing="true"]) {
    background: var(--color-bg-4);
  }

  .resizer-handle {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: 4px;
    height: 40px;
    background: var(--color-bg-4);
    border-radius: 2px;
    opacity: 0.5;
    transition: opacity 0.15s ease;
  }

  :global(.pane-resizer:hover) .resizer-handle,
  :global(.pane-resizer[data-resizing="true"]) .resizer-handle {
    opacity: 1;
    background: var(--color-primary);
  }

  .players-list {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--color-bg-2);
    border-radius: var(--radius-lg);
    overflow: hidden;
  }

  .players-scroll {
    flex: 1;
    overflow-y: auto;
  }

  .list-header {
    padding: 1rem;
    border-bottom: 1px solid var(--color-bg-3);
    background: var(--color-bg-3);
  }

  .list-header h3 {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
  }

  .player-item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    width: 100%;
    padding: 0.75rem 1rem;
    background: transparent;
    border: none;
    border-bottom: 1px solid var(--color-bg-3);
    cursor: pointer;
    transition:
      background 0.2s ease,
      transform 0.1s ease;
    text-align: left;
    color: var(--color-text-1);
  }

  .player-item:hover {
    background: var(--color-bg-3);
  }
  .player-item:active {
    transform: scale(0.98);
  }

  .player-item.active {
    background: var(--color-primary);
    color: white;
  }

  .player-item.active .player-name {
    color: white;
  }
  .player-item.active .player-meta {
    opacity: 0.85;
  }
  .player-item.active .edition-badge {
    background: rgba(255, 255, 255, 0.2);
    color: white;
  }

  .player-avatar {
    width: 48px;
    height: 48px;
    border-radius: 4px;
    flex-shrink: 0;
    overflow: hidden;
    background: var(--color-bg-4);
  }

  .player-head {
    width: 100%;
    height: 100%;
    object-fit: cover;
    image-rendering: pixelated;
  }

  .player-head-placeholder {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 1.5rem;
    background: var(--color-bg-3);
  }

  .player-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .player-name {
    font-weight: 600;
    font-size: 0.875rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .player-meta {
    font-size: 0.75rem;
    opacity: 0.7;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .edition-badge {
    font-size: 0.625rem;
    padding: 0.125rem 0.375rem;
    border-radius: var(--radius-sm);
    font-weight: 700;
    text-transform: uppercase;
  }

  .edition-badge.java {
    background: #ff6b6b;
    color: white;
  }
  .edition-badge.bedrock {
    background: #4ecdc4;
    color: white;
  }

  .player-details {
    background: transparent;
    padding: 0;
    flex: 1;
    min-height: 0;
  }

  :global(.player-details-pane) {
    display: flex;
    flex-direction: column;
    overflow: hidden !important;
    min-height: 0;
  }

  .no-selection {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--color-text-3);
  }
</style>
