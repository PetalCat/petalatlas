<script lang="ts">
  import { TABS, type TabId } from "./tabs/tabs";
  import ItemSlot from "$lib/components/player/ItemSlot.svelte";
  import { SkinViewer } from "skinview3d";
  import { invoke } from "@tauri-apps/api/core";
  import type { PlayerData, FullPlayerData } from "./types";
  import type { InventoryItem } from "$lib/components/player/ItemSlot.svelte";
  import type { PotionEffect } from "$lib/components/player/EffectsPanel.svelte";
  import type { PlayerStats } from "$lib/components/player/StatsEditor.svelte";
  import type { BackendAdvancementCategory, StatCategory } from "./types";

  // Props using Runes
  let {
    player,
    fullData,
    activeTab,
    inventory,
    hotbar,
    armor,
    offhand,
    ender,
    effects,
    stats,
    advancements,
    statistics,
    onTabChange,
    onInventoryChange = () => {},
    onSlotSelect = () => {},
    onStatsChange = () => {},
    onEffectChange = () => {},
    onEffectAdd = () => {},
    onEffectRemove = () => {},
    onAdvancementGrant = () => {},
    onAdvancementRevoke = () => {},
    onStatisticChange = () => {},
    onArmorEdit = () => {},
  } = $props<{
    player: PlayerData;
    fullData: FullPlayerData | null;
    activeTab: TabId;
    inventory: (InventoryItem | null)[];
    hotbar: (InventoryItem | null)[];
    armor: (InventoryItem | null)[];
    offhand: InventoryItem | null;
    ender: (InventoryItem | null)[];
    effects: PotionEffect[];
    stats: PlayerStats;
    advancements: BackendAdvancementCategory[];
    statistics: StatCategory[];
    onTabChange: (tab: TabId) => void;
    onInventoryChange?: (slot: number, item: InventoryItem | null) => void;
    onSlotSelect?: (slot: number) => void;
    onStatsChange?: (field: string, value: unknown) => void;
    onEffectChange?: (effect: PotionEffect) => void;
    onEffectAdd?: (id: string) => void;
    onEffectRemove?: (id: string) => void;
    onAdvancementGrant?: (id: string) => void;
    onAdvancementRevoke?: (id: string) => void;
    onStatisticChange?: (category: string, stat: string, value: number) => void;
    onArmorEdit?: (index: number, item: InventoryItem | null) => void;
  }>();

  let skinViewerContainer: HTMLCanvasElement;
  let skinViewer: SkinViewer | null = null;
  let hoveredItem = $state<InventoryItem | null>(null);

  // Load skin logic (moved from +page.svelte)
  $effect(() => {
    if (skinViewerContainer && player) {
      // Dispose old viewer first to ensure clean state
      if (skinViewer) {
        skinViewer.dispose();
        skinViewer = null;
      }

      // Get skin URL from profile
      invoke<any>("fetch_player_profile", { uuid: player.primary_id }).then(
        (data) => {
          if (data.properties && data.properties.length > 0) {
            const texturesProp = data.properties.find(
              (p: any) => p.name === "textures"
            );
            if (texturesProp && texturesProp.value) {
              const decoded = atob(texturesProp.value);
              const textures = JSON.parse(decoded);
              const skinUrl = textures.textures?.SKIN?.url;

              if (skinUrl) {
                // Always create a fresh viewer for each player
                skinViewer = new SkinViewer({
                  canvas: skinViewerContainer,
                  width: 200,
                  height: 260,
                });
                skinViewer.camera.rotation.x = -0.5;
                skinViewer.camera.rotation.y = 0.5;
                skinViewer.camera.rotation.z = 0.1;
                skinViewer.camera.position.z = 40;

                // Auto-rotate
                skinViewer.autoRotate = true;
                skinViewer.autoRotateSpeed = 1;

                // Load skin through backend
                invoke<string>("fetch_image", { url: skinUrl }).then(
                  (dataUrl) => {
                    if (skinViewer) {
                      skinViewer.loadSkin(dataUrl);
                    }
                  }
                );
              }
            }
          }
        }
      );
    }
  });

  let activeDef = $derived(TABS.find((t) => t.id === activeTab));
</script>

<div class="player-details-card">
  <!-- Tab Navigation (Locked) -->
  <div class="tab-bar">
    {#each TABS as tab}
      <button
        class="tab-btn"
        class:active={activeTab === tab.id}
        onclick={() => onTabChange(tab.id)}
      >
        {tab.icon}
        {tab.label}
      </button>
    {/each}
  </div>

  <!-- Scroll Container -->
  <div class="player-scroll">
    <div class="details-header">
      <div class="player-profile-card flex gap-md">
        <!-- Armor Column -->
        <div class="armor-column">
          {#each armor.slice().reverse() as item, i}
            {@const realIndex = 3 - i}
            <div class="armor-slot-wrapper">
              <ItemSlot
                {item}
                slotIndex={36 + realIndex}
                selected={false}
                modified={false}
                onselect={() => console.log("Select armor", 36 + realIndex)}
                onedit={(newItem) => onArmorEdit(realIndex, newItem)}
                onhover={(hovering) => (hoveredItem = hovering ? item : null)}
              />
            </div>
          {/each}
        </div>

        <!-- Skin Viewer & Offhand -->
        <div class="player-skin-wrapper">
          <canvas bind:this={skinViewerContainer} class="skin-viewer"></canvas>

          <!-- Offhand Slot (Positioned absolutely bottom-right) -->
          <div class="offhand-slot">
            <ItemSlot
              item={offhand}
              selected={false}
              modified={false}
              onselect={() => console.log("Select offhand")}
              onedit={(newItem) => onInventoryChange(40, newItem)}
              onhover={(hovering) => (hoveredItem = hovering ? offhand : null)}
            />
          </div>
        </div>

        <!-- Player Info -->
        <div class="player-header-info">
          <div class="header-title">
            <h2>
              {player.display_name || player.primary_id.substring(0, 8) + "..."}
            </h2>
            <span class="edition-tag {player.edition}">
              {player.edition === "java"
                ? "☕ Java Edition"
                : "🛏️ Bedrock Edition"}
            </span>
          </div>
          <div class="id-badge">
            <span class="id-label"
              >{player.edition === "java" ? "UUID" : "XUID"}</span
            >
            <span class="id-value">{player.primary_id}</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Tab Content -->
    <div class="tab-content">
      {#if activeTab === "inventory"}
        <svelte:component
          this={activeDef.component}
          {inventory}
          {hotbar}
          {armor}
          {offhand}
          {ender}
          {hoveredItem}
          onitemchange={onInventoryChange}
          onslotselect={onSlotSelect}
          onitemhover={(item: InventoryItem | null) => (hoveredItem = item)}
        />
      {:else if activeTab === "stats"}
        <svelte:component
          this={activeDef.component}
          {stats}
          onchange={onStatsChange}
        />
      {:else if activeTab === "effects"}
        <svelte:component
          this={activeDef.component}
          {effects}
          onchange={onEffectChange}
          onadd={onEffectAdd}
          onremove={onEffectRemove}
        />
      {:else if activeTab === "advancements"}
        <svelte:component
          this={activeDef.component}
          categories={advancements}
          ongrant={onAdvancementGrant}
          onrevoke={onAdvancementRevoke}
        />
      {:else if activeTab === "statistics"}
        <svelte:component
          this={activeDef.component}
          categories={statistics}
          onchange={onStatisticChange}
        />
      {/if}
    </div>
  </div>
</div>

<style>
  .player-details-card {
    display: flex;
    flex-direction: column;
    flex: 1; /* Ensure it takes full height of parent */
    min-height: 0;
    height: 100%;
    overflow: hidden; /* Prevent body scroll */
  }

  /* Sticky Tab Bar */
  .tab-bar {
    display: flex;
    gap: 0.5rem;
    padding: 1rem 2rem 0;
    border-bottom: 1px solid var(--color-bg-3);
    background: var(--color-bg-2); /* Match header bg if needed */
    position: sticky;
    top: 0;
    flex-shrink: 0;
    z-index: 10;
  }

  .tab-btn {
    padding: 0.75rem 1.25rem;
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--color-text-2);
    cursor: pointer;
    font-size: 0.9rem;
    font-weight: 500;
    transition: all 0.2s ease;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .tab-btn:hover {
    color: var(--color-text-1);
    background: var(--color-bg-3);
    border-radius: var(--radius-md) var(--radius-md) 0 0;
  }

  .tab-btn.active {
    color: var(--color-primary);
    border-bottom-color: var(--color-primary);
  }

  /* Single Scroll Container */
  .player-scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }

  .details-header {
    margin-bottom: 0; /* Reduced to fit on screen better */
    padding: 1.5rem 2rem;
    border-bottom: 1px solid var(--color-bg-3);
    flex-shrink: 0;
  }

  .player-profile-card {
    display: flex;
    gap: 1.5rem;
    align-items: flex-start;
    background: var(--color-bg-3);
    padding: 1.5rem;
    border-radius: var(--radius-lg);
    box-shadow: inset 0 0 20px rgba(0, 0, 0, 0.1);
  }

  .flex {
    display: flex;
  }
  .gap-md {
    gap: 1.5rem;
  }

  .armor-column {
    display: flex;
    flex-direction: column;
    gap: 4px;
    justify-content: center; /* Center vertically relative to skin */
    height: 260px; /* MATCH skin viewer height */
  }

  /* WRAPPER FOR SKIN + OFFHAND */
  .player-skin-wrapper {
    position: relative;
    flex-shrink: 0;
    width: 200px; /* Fixed width */
    height: 260px; /* Clamped Height */
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .skin-viewer {
    width: 100%;
    height: 100%;
    image-rendering: pixelated;
    filter: drop-shadow(0 10px 15px rgba(0, 0, 0, 0.3));
  }

  .offhand-slot {
    position: absolute;
    bottom: 0;
    right: 0;
    width: 56px; /* Match global slot size */
    height: 56px;
    z-index: 10;
  }

  .armor-slot-wrapper {
    display: flex;
    align-items: center;
    justify-content: center;
  }

  /* Ensure ItemSlot takes full size but no extra wrappers/backgrounds needed as ItemSlot handles it */
  .armor-slot-wrapper :global(.item-slot) {
    width: 56px;
    height: 56px;
    /* Removed custom bg/border to let ItemSlot use system tokens */
  }

  .player-header-info {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 1rem;
    margin-top: 10px;
  }

  .header-title h2 {
    margin: 0 0 0.5rem 0;
    font-size: 1.5rem;
  }

  .edition-tag {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.75rem;
    padding: 0.25rem 0.75rem;
    border-radius: var(--radius-md);
    font-weight: 600;
  }

  .edition-tag.java {
    background: rgba(255, 107, 107, 0.2);
    color: #ff6b6b;
  }
  .edition-tag.bedrock {
    background: rgba(78, 205, 196, 0.2);
    color: #4ecdc4;
  }

  .id-badge {
    background: var(--color-bg-3);
    padding: 1rem;
    border-radius: var(--radius-md);
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .id-label {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--color-text-3);
    text-transform: uppercase;
  }

  .id-value {
    font-family: "SF Mono", monospace;
    font-size: 0.875rem;
    color: var(--color-text-1);
    word-break: break-all;
  }

  .tab-content {
    padding: 1rem 2rem 2rem; /* Add top padding */
    /* REMOVED: flex: 1, min-height: 0, overflow: hidden */
    /* Let content flow naturally inside scroll container */
  }
</style>
