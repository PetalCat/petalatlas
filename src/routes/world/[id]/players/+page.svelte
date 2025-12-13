<script lang="ts">
  import { page } from "$app/stores";
  import { goto } from "$app/navigation";
  import { invoke } from "@tauri-apps/api/core";
  import type { WorldInfo } from "$lib/types";
  import { onMount } from "svelte";
  import { SkinViewer } from "skinview3d";

  let world = $state<WorldInfo | null>(null);
  let loading = $state(true);
  let error = $state("");
  let selectedPlayer = $state<string | null>(null);
  let skinViewerContainer = $state<HTMLCanvasElement | null>(null);
  let skinViewer: SkinViewer | null = null;

  interface PlayerData {
    edition: string; // "java" or "bedrock"
    primary_id: string; // UUID for Java, XUID for Bedrock
    display_name?: string; // Cached name (not authoritative)
    position: { x: number; y: number; z: number };
    health: number;
    food_level: number;
    xp_level: number;
    xp_total: number;
    game_mode: number;
    dimension: string;
    last_seen?: number;
  }

  let players = $state<PlayerData[]>([]);
  let playerDetails = $state<Map<string, PlayerData>>(new Map());
  let loadingDetails = $state<Set<string>>(new Set());
  let playerHeads = $state<Map<string, string>>(new Map());
  let playerNames = $state<Map<string, string>>(new Map());

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
        // Load player list (UUIDs only - fast!)
        try {
          const playerData = await invoke<PlayerData[]>("get_players", {
            worldPath: world.path,
          });
          players = playerData;
          console.log("Loaded player UUIDs:", players.length);

          // Start loading names in background (which also loads skins)
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

  // Load player head from backend (bypasses proxy)
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

  // Load player name and skin from Mojang API via backend (bypasses proxy)
  async function loadPlayerName(uuid: string) {
    if (playerNames.has(uuid)) return;

    try {
      const data = await invoke<any>("fetch_player_profile", { uuid });
      if (data.name) {
        playerNames.set(uuid, data.name);
        playerNames = new Map(playerNames);
      }

      // Extract skin URL from textures and fetch through backend
      if (data.properties && data.properties.length > 0) {
        const texturesProp = data.properties.find(
          (p: any) => p.name === "textures"
        );
        if (texturesProp && texturesProp.value) {
          try {
            const decoded = atob(texturesProp.value);
            const textures = JSON.parse(decoded);
            if (textures.textures?.SKIN?.url) {
              // Fetch the actual image through backend to bypass proxy
              const dataUrl = await invoke<string>("fetch_image", {
                url: textures.textures.SKIN.url,
              });

              // Crop to just the face (8x8 with overlay)
              const faceDataUrl = await cropSkinToFace(dataUrl);
              playerHeads.set(uuid, faceDataUrl);
              playerHeads = new Map(playerHeads);
            }
          } catch (e) {
            console.error("Failed to parse textures:", e);
          }
        }
      }
    } catch (e) {
      console.error("Failed to load player name:", e);
    }
  }

  // Crop skin texture to show just the face with overlay
  async function cropSkinToFace(skinDataUrl: string): Promise<string> {
    return new Promise((resolve) => {
      const img = new Image();
      img.onload = () => {
        const canvas = document.createElement("canvas");
        const ctx = canvas.getContext("2d")!;

        // Face is 8x8 pixels
        canvas.width = 8;
        canvas.height = 8;

        // Draw base layer (face at 8,8 in 64x64 texture)
        ctx.drawImage(img, 8, 8, 8, 8, 0, 0, 8, 8);

        // Draw overlay layer (face overlay at 40,8)
        ctx.drawImage(img, 40, 8, 8, 8, 0, 0, 8, 8);

        resolve(canvas.toDataURL());
      };
      img.src = skinDataUrl;
    });
  }

  // Load detailed player data on selection
  async function loadPlayerDetails(uuid: string) {
    if (playerDetails.has(uuid) || loadingDetails.has(uuid) || !world) {
      return;
    }

    loadingDetails.add(uuid);
    loadingDetails = new Set(loadingDetails);

    try {
      const details = await invoke<PlayerData>("get_player_details", {
        worldPath: world.path,
        uuid: uuid,
      });
      playerDetails.set(uuid, details);
      playerDetails = new Map(playerDetails);
      console.log("Loaded details for", uuid);
    } catch (e) {
      console.error("Failed to load player details:", e);
    } finally {
      loadingDetails.delete(uuid);
      loadingDetails = new Set(loadingDetails);
    }
  }

  // Auto-load details when player is selected
  $effect(() => {
    if (selectedPlayer) {
      loadPlayerDetails(selectedPlayer);
    }
  });

  // Update skin viewer when player changes
  $effect(() => {
    if (skinViewerContainer && selectedPlayer) {
      const player =
        playerDetails.get(selectedPlayer) ||
        players.find((p) => p.primary_id === selectedPlayer);

      if (player) {
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
                  // Create new viewer if needed
                  if (!skinViewer) {
                    skinViewer = new SkinViewer({
                      canvas: skinViewerContainer,
                      width: 200,
                      height: 300,
                    });
                    skinViewer.camera.rotation.x = -0.5;
                    skinViewer.camera.rotation.y = 0.5;
                    skinViewer.camera.rotation.z = 0.1;
                    skinViewer.camera.position.z = 40; // Increased from 30 for more zoom out

                    // Auto-rotate
                    skinViewer.autoRotate = true;
                    skinViewer.autoRotateSpeed = 1;
                  }

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
    <div class="page-header">
      <h1>👤 Players</h1>
      <p class="page-subtitle">Manage player data for {world.name}</p>
    </div>

    {#if players.length === 0}
      <div class="empty-state">
        <div class="empty-icon">👥</div>
        <h2>No Player Data Found</h2>
        <p>
          {#if world.platform.toLowerCase().includes("java")}
            Player data is stored in the <code>playerdata</code> folder when players
            join the world.
          {:else}
            Bedrock player data will appear here once available.
          {/if}
        </p>
        <div class="info-box">
          <h3>💡 Coming Soon</h3>
          <ul>
            <li>Player list with UUID lookup</li>
            <li>Inventory viewer and editor</li>
            <li>Player stats and achievements</li>
            <li>Player position and dimension</li>
            <li>Experience and health management</li>
          </ul>
        </div>
      </div>
    {:else}
      <div class="players-layout">
        <aside class="players-list">
          <div class="list-header">
            <h3>Players ({players.length})</h3>
          </div>
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
        </aside>

        <main class="player-details">
          {#if selectedPlayer}
            {@const player =
              playerDetails.get(selectedPlayer) ||
              players.find((p) => p.primary_id === selectedPlayer)}
            {#if loadingDetails.has(selectedPlayer)}
              <div class="loading-details">
                <div class="spinner"></div>
                <p>Loading player details...</p>
              </div>
            {:else if player}
              <div class="details-header">
                <div class="player-card">
                  <div class="player-skin">
                    <canvas bind:this={skinViewerContainer} class="skin-viewer"
                    ></canvas>
                  </div>
                  <div class="player-header-info">
                    <div class="header-title">
                      <h2>
                        {playerNames.get(player.primary_id) ||
                          player.display_name ||
                          "Player"}
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

              <div class="stats-grid">
                <div class="stat-card">
                  <div class="stat-header">
                    <span class="stat-icon">❤️</span>
                    <span class="stat-label">Health</span>
                  </div>
                  <span class="stat-value">{player.health.toFixed(1)} / 20</span
                  >
                </div>

                <div class="stat-card">
                  <div class="stat-header">
                    <span class="stat-icon">🍖</span>
                    <span class="stat-label">Food Level</span>
                  </div>
                  <span class="stat-value">{player.food_level} / 20</span>
                </div>

                <div class="stat-card">
                  <div class="stat-header">
                    <span class="stat-icon">⭐</span>
                    <span class="stat-label">XP Level</span>
                  </div>
                  <span class="stat-value">{player.xp_level}</span>
                  <span class="stat-subvalue">{player.xp_total} total</span>
                </div>

                <div class="stat-card">
                  <div class="stat-header">
                    <span class="stat-icon">🎮</span>
                    <span class="stat-label">Game Mode</span>
                  </div>
                  <span class="stat-value">
                    {["Survival", "Creative", "Adventure", "Spectator"][
                      player.game_mode
                    ] || "Unknown"}
                  </span>
                </div>

                <div class="stat-card full-width">
                  <div class="stat-header">
                    <span class="stat-icon">🌍</span>
                    <span class="stat-label">Dimension</span>
                  </div>
                  <span class="stat-value">{player.dimension}</span>
                </div>

                <div class="stat-card full-width">
                  <div class="stat-header">
                    <span class="stat-icon">📍</span>
                    <span class="stat-label">Position</span>
                  </div>
                  <span class="stat-value position">
                    X: {player.position.x.toFixed(2)}
                    Y: {player.position.y.toFixed(2)}
                    Z: {player.position.z.toFixed(2)}
                  </span>
                </div>
              </div>
            {/if}
          {:else}
            <div class="no-selection">
              <p>Select a player to view details</p>
            </div>
          {/if}
        </main>
      </div>
    {/if}
  </div>
{/if}

<style>
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
  }

  .page-header {
    margin-bottom: 2rem;
  }

  .page-header h1 {
    margin: 0 0 0.5rem 0;
    font-size: 2rem;
    font-weight: 700;
  }

  .page-subtitle {
    margin: 0;
    color: var(--color-text-3);
    font-size: 1rem;
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

  .info-box {
    background: var(--color-primary-light);
    border-left: 4px solid var(--color-primary);
    border-radius: var(--radius-md);
    padding: 1.5rem;
    text-align: left;
    margin-top: 2rem;
  }

  .info-box h3 {
    margin: 0 0 1rem 0;
    color: var(--color-primary);
    font-size: 1rem;
  }

  .info-box ul {
    margin: 0;
    padding-left: 1.5rem;
  }

  .info-box li {
    color: var(--color-text-2);
    font-size: 0.875rem;
    margin-bottom: 0.5rem;
  }

  .players-layout {
    display: grid;
    grid-template-columns: 300px 1fr;
    gap: 2rem;
    min-height: 500px;
  }

  .players-list {
    background: var(--color-bg-2);
    border-radius: var(--radius-lg);
    overflow: hidden;
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
    background: none;
    border: none;
    border-bottom: 1px solid var(--color-bg-3);
    cursor: pointer;
    transition: all 0.2s;
    text-align: left;
  }

  .player-item:hover {
    background: var(--color-bg-3);
  }

  .player-item.active {
    background: var(--color-primary);
    color: white;
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
  }

  .player-name {
    font-weight: 600;
    font-size: 0.875rem;
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
    background: var(--color-bg-2);
    border-radius: var(--radius-lg);
    padding: 2rem;
  }

  .details-header {
    margin-bottom: 2rem;
    padding-bottom: 1.5rem;
    border-bottom: 1px solid var(--color-bg-3);
  }

  .player-card {
    display: flex;
    gap: 2rem;
    align-items: flex-start;
  }

  .player-skin {
    flex-shrink: 0;
    background: linear-gradient(135deg, var(--color-bg-3), var(--color-bg-4));
    border-radius: var(--radius-lg);
    padding: 0.5rem;
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 200px;
  }

  .skin-viewer {
    width: 200px;
    height: 300px;
    image-rendering: pixelated;
  }

  .skin-render {
    width: 120px;
    height: auto;
    image-rendering: pixelated;
    filter: drop-shadow(0 10px 20px rgba(0, 0, 0, 0.2));
  }

  .skin-placeholder {
    width: 120px;
    height: 180px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 4rem;
    opacity: 0.3;
  }

  .player-header-info {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .header-title {
    margin-bottom: 1rem;
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

  .warning-box {
    background: #fef3c7;
    border-left: 4px solid #f59e0b;
    border-radius: var(--radius-md);
    padding: 1.5rem;
    margin-bottom: 2rem;
  }

  .warning-box h3 {
    margin: 0 0 1rem 0;
    color: #92400e;
    font-size: 1rem;
  }

  .safety-info {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .safe-section,
  .danger-section {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .safe-section strong {
    color: #15803d;
  }

  .danger-section strong {
    color: #991b1b;
  }

  .safe-section span,
  .danger-section span {
    font-size: 0.875rem;
    color: #78350f;
  }

  .uuid-badge {
    font-family: "SF Mono", monospace;
    font-size: 0.75rem;
    background: var(--color-bg-3);
    padding: 0.5rem 1rem;
    border-radius: var(--radius-md);
    color: var(--color-text-3);
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 1rem;
  }

  .stat-card {
    padding: 1.5rem;
    background: var(--color-bg-3);
    border-radius: var(--radius-lg);
  }

  .stat-card.full-width {
    grid-column: 1 / -1;
  }

  .stat-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.75rem;
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

  .stat-value.position {
    font-family: "SF Mono", monospace;
    font-size: 1rem;
  }

  .stat-subvalue {
    display: block;
    font-size: 0.875rem;
    color: var(--color-text-3);
    margin-top: 0.25rem;
  }

  .no-selection {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--color-text-3);
  }
</style>
