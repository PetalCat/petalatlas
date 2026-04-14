<script lang="ts">
  import ValueSlider from "./ValueSlider.svelte";

  export interface PlayerStats {
    health: number;
    maxHealth: number;
    foodLevel: number;
    saturation: number;
    xpProgress: number; // 0-1 progress to next level
    xpLevel: number;
    xpTotal: number;
    gameMode: number; // 0=survival, 1=creative, 2=adventure, 3=spectator
    position: { x: number; y: number; z: number };
    dimension: string;
    spawnPoint?: { x: number; y: number; z: number };
    deathLocation?: { x: number; y: number; z: number; dimension: string };
  }

  let {
    stats = $bindable<PlayerStats>({
      health: 20,
      maxHealth: 20,
      foodLevel: 20,
      saturation: 5,
      xpProgress: 0,
      xpLevel: 0,
      xpTotal: 0,
      gameMode: 0,
      position: { x: 0, y: 64, z: 0 },
      dimension: "minecraft:overworld",
    }),
    modifiedFields = [],
    onchange = () => {},
  } = $props<{
    stats: PlayerStats;
    modifiedFields?: string[];
    onchange?: (field: string, value: unknown) => void;
  }>();

  const gameModes = [
    {
      id: 0,
      name: "Survival",
      icon: "⚔️",
      description: "Collect resources and survive",
    },
    {
      id: 1,
      name: "Creative",
      icon: "🎨",
      description: "Unlimited resources, fly",
    },
    {
      id: 2,
      name: "Adventure",
      icon: "🗺️",
      description: "Explore, cannot break blocks",
    },
    {
      id: 3,
      name: "Spectator",
      icon: "👻",
      description: "Fly through blocks, invisible",
    },
  ];

  const dimensions = [
    { id: "minecraft:overworld", name: "Overworld", icon: "🌍" },
    { id: "minecraft:the_nether", name: "The Nether", icon: "🔥" },
    { id: "minecraft:the_end", name: "The End", icon: "🌌" },
  ];

  let editingPosition = $state(false);
  let tempPosition = $state({ x: 0, y: 0, z: 0 });

  function handleHealthChange(value: number) {
    stats.health = value;
    onchange("health", value);
  }

  function handleHungerChange(value: number) {
    stats.foodLevel = value;
    onchange("foodLevel", value);
  }

  function handleXpChange(progress: number, level?: number) {
    if (level !== undefined) {
      stats.xpLevel = level;
      onchange("xpLevel", level);
    }
    stats.xpProgress = progress / 100; // Convert from percent
    onchange("xpProgress", stats.xpProgress);
  }

  function setGameMode(mode: number) {
    stats.gameMode = mode;
    onchange("gameMode", mode);
  }

  function setDimension(dim: string) {
    stats.dimension = dim;
    onchange("dimension", dim);
  }

  function startEditPosition() {
    tempPosition = { ...stats.position };
    editingPosition = true;
  }

  function applyPosition() {
    stats.position = { ...tempPosition };
    onchange("position", stats.position);
    editingPosition = false;
  }

  function cancelPosition() {
    editingPosition = false;
  }
</script>

<div class="stats-editor">
  <!-- Health, Hunger, XP Sliders -->
  <div class="sliders-section">
    <ValueSlider
      type="health"
      bind:value={stats.health}
      max={stats.maxHealth}
      modified={modifiedFields.includes("health")}
      onchange={handleHealthChange}
    />

    <ValueSlider
      type="hunger"
      bind:value={stats.foodLevel}
      max={20}
      modified={modifiedFields.includes("foodLevel")}
      onchange={handleHungerChange}
    />

    <ValueSlider
      type="xp"
      value={Math.round(stats.xpProgress * 100)}
      max={100}
      level={stats.xpLevel}
      modified={modifiedFields.includes("xpLevel") ||
        modifiedFields.includes("xpProgress")}
      onchange={handleXpChange}
    />
  </div>

  <!-- Game Mode -->
  <section class="stat-section">
    <h4 class="section-title">🎮 Game Mode</h4>
    <div class="game-mode-buttons">
      {#each gameModes as mode}
        <button
          class="mode-btn"
          class:active={stats.gameMode === mode.id}
          class:modified={modifiedFields.includes("gameMode") &&
            stats.gameMode === mode.id}
          onclick={() => setGameMode(mode.id)}
          title={mode.description}
        >
          <span class="mode-icon">{mode.icon}</span>
          <span class="mode-name">{mode.name}</span>
        </button>
      {/each}
    </div>
  </section>

  <!-- Dimension -->
  <section class="stat-section">
    <h4 class="section-title">🌍 Dimension</h4>
    <div class="dimension-buttons">
      {#each dimensions as dim}
        <button
          class="dim-btn"
          class:active={stats.dimension === dim.id}
          class:modified={modifiedFields.includes("dimension") &&
            stats.dimension === dim.id}
          onclick={() => setDimension(dim.id)}
        >
          <span class="dim-icon">{dim.icon}</span>
          <span class="dim-name">{dim.name}</span>
        </button>
      {/each}
    </div>
  </section>

  <!-- Position -->
  <section class="stat-section">
    <h4 class="section-title">📍 Position</h4>
    {#if editingPosition}
      <div class="position-editor">
        <div class="coord-inputs">
          <label class="coord-input">
            <span class="coord-label">X</span>
            <input type="number" step="0.1" bind:value={tempPosition.x} />
          </label>
          <label class="coord-input">
            <span class="coord-label">Y</span>
            <input type="number" step="0.1" bind:value={tempPosition.y} />
          </label>
          <label class="coord-input">
            <span class="coord-label">Z</span>
            <input type="number" step="0.1" bind:value={tempPosition.z} />
          </label>
        </div>
        <div class="position-actions">
          <button class="btn-apply" onclick={applyPosition}>✓ Apply</button>
          <button class="btn-cancel" onclick={cancelPosition}>✕ Cancel</button>
        </div>
      </div>
    {:else}
      <div
        class="position-display"
        class:modified={modifiedFields.includes("position")}
      >
        <span class="coord"
          ><strong>X:</strong> {stats.position.x.toFixed(2)}</span
        >
        <span class="coord"
          ><strong>Y:</strong> {stats.position.y.toFixed(2)}</span
        >
        <span class="coord"
          ><strong>Z:</strong> {stats.position.z.toFixed(2)}</span
        >
        <button class="btn-edit" onclick={startEditPosition}>✏️ Edit</button>
      </div>
    {/if}
  </section>

  <!-- Spawn Point (read-only display) -->
  {#if stats.spawnPoint}
    <section class="stat-section info-section">
      <h4 class="section-title">🛏️ Spawn Point</h4>
      <div class="position-display readonly">
        <span class="coord"><strong>X:</strong> {stats.spawnPoint.x}</span>
        <span class="coord"><strong>Y:</strong> {stats.spawnPoint.y}</span>
        <span class="coord"><strong>Z:</strong> {stats.spawnPoint.z}</span>
      </div>
    </section>
  {/if}

  <!-- Death Location (read-only display) -->
  {#if stats.deathLocation}
    <section class="stat-section info-section">
      <h4 class="section-title">💀 Last Death</h4>
      <div class="position-display readonly">
        <span class="coord"><strong>X:</strong> {stats.deathLocation.x}</span>
        <span class="coord"><strong>Y:</strong> {stats.deathLocation.y}</span>
        <span class="coord"><strong>Z:</strong> {stats.deathLocation.z}</span>
        <span class="coord-dim"
          >{stats.deathLocation.dimension.replace("minecraft:", "")}</span
        >
      </div>
    </section>
  {/if}
</div>

<style>
  .stats-editor {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .sliders-section {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .stat-section {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px 16px;
    background: var(--color-bg-2);
    border-radius: var(--radius-md);
    border: 1px solid var(--color-bg-3);
  }

  .info-section {
    opacity: 0.8;
  }

  .section-title {
    margin: 0;
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--color-text-2);
  }

  /* Game Mode Buttons */
  .game-mode-buttons {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 8px;
  }

  .mode-btn {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 10px 8px;
    background: var(--color-bg-3);
    border: 1px solid var(--color-bg-4);
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .mode-btn:hover {
    background: var(--color-bg-4);
    transform: translateY(-1px);
  }

  .mode-btn.active {
    background: var(--color-primary-light);
    border-color: var(--color-primary);
    color: var(--color-primary);
  }

  .mode-btn.modified {
    border-left: 3px solid var(--modified-color);
  }

  .mode-icon {
    font-size: 1.2rem;
  }

  .mode-name {
    font-size: 0.7rem;
    font-weight: 600;
  }

  /* Dimension Buttons */
  .dimension-buttons {
    display: flex;
    gap: 8px;
  }

  .dim-btn {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 8px 12px;
    background: var(--color-bg-3);
    border: 1px solid var(--color-bg-4);
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .dim-btn:hover {
    background: var(--color-bg-4);
  }

  .dim-btn.active {
    background: var(--color-primary-light);
    border-color: var(--color-primary);
  }

  .dim-btn.modified {
    border-left: 3px solid var(--modified-color);
  }

  .dim-icon {
    font-size: 1rem;
  }

  .dim-name {
    font-size: 0.8rem;
    font-weight: 500;
  }

  /* Position Display */
  .position-display {
    display: flex;
    align-items: center;
    gap: 16px;
    font-family: "SF Mono", "Monaco", monospace;
    font-size: 0.85rem;
  }

  .position-display.modified {
    border-left: 3px solid var(--modified-color);
    padding-left: 12px;
    margin-left: -12px;
  }

  .position-display.readonly {
    opacity: 0.8;
  }

  .coord {
    color: var(--color-text-2);
  }

  .coord strong {
    color: var(--color-text-3);
    margin-right: 4px;
  }

  .coord-dim {
    font-size: 0.75rem;
    color: var(--color-text-3);
    text-transform: capitalize;
  }

  .btn-edit {
    margin-left: auto;
    padding: 4px 10px;
    font-size: 0.75rem;
    background: transparent;
    border: 1px solid var(--color-bg-4);
    border-radius: var(--radius-sm);
    color: var(--color-text-2);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .btn-edit:hover {
    background: var(--color-bg-3);
    border-color: var(--color-primary);
  }

  /* Position Editor */
  .position-editor {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .coord-inputs {
    display: flex;
    gap: 12px;
  }

  .coord-input {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .coord-label {
    font-size: 0.7rem;
    font-weight: 600;
    color: var(--color-text-3);
    text-transform: uppercase;
  }

  .coord-input input {
    padding: 8px 10px;
    font-family: "SF Mono", "Monaco", monospace;
    font-size: 0.9rem;
    background: var(--color-bg-1);
    border: 1px solid var(--color-bg-4);
    border-radius: var(--radius-sm);
    color: var(--color-text-1);
  }

  .coord-input input:focus {
    outline: none;
    border-color: var(--color-primary);
  }

  .position-actions {
    display: flex;
    gap: 8px;
  }

  .btn-apply,
  .btn-cancel {
    padding: 6px 14px;
    font-size: 0.8rem;
    font-weight: 500;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .btn-apply {
    background: var(--color-green);
    border: none;
    color: white;
  }

  .btn-apply:hover {
    filter: brightness(1.1);
  }

  .btn-cancel {
    background: transparent;
    border: 1px solid var(--color-bg-4);
    color: var(--color-text-2);
  }

  .btn-cancel:hover {
    background: var(--color-bg-3);
  }
</style>
