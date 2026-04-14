<script lang="ts">
  export interface PotionEffect {
    id: string;
    amplifier: number; // 0 = level I
    duration: number; // Ticks (-1 = infinite)
    showParticles?: boolean;
  }

  let {
    effects = [],
    modifiedEffects = [],
    onchange = () => {},
    onadd = () => {},
    onremove = () => {},
  } = $props<{
    effects: PotionEffect[];
    modifiedEffects?: string[];
    onchange?: (effect: PotionEffect) => void;
    onadd?: (effectId: string) => void;
    onremove?: (effectId: string) => void;
  }>();

  let showAddMenu = $state(false);
  let searchQuery = $state("");

  // Common potion effects
  const availableEffects = [
    { id: "speed", name: "Speed", icon: "💨", color: "#7cafc6" },
    { id: "slowness", name: "Slowness", icon: "🐌", color: "#5a6c81" },
    { id: "haste", name: "Haste", icon: "⛏️", color: "#d9c043" },
    {
      id: "mining_fatigue",
      name: "Mining Fatigue",
      icon: "😓",
      color: "#4a4217",
    },
    { id: "strength", name: "Strength", icon: "💪", color: "#932423" },
    {
      id: "instant_health",
      name: "Instant Health",
      icon: "❤️‍🩹",
      color: "#f82423",
    },
    {
      id: "instant_damage",
      name: "Instant Damage",
      icon: "💔",
      color: "#430a09",
    },
    { id: "jump_boost", name: "Jump Boost", icon: "🦘", color: "#22ff4c" },
    { id: "nausea", name: "Nausea", icon: "🌀", color: "#551d4a" },
    { id: "regeneration", name: "Regeneration", icon: "💖", color: "#cd5cab" },
    { id: "resistance", name: "Resistance", icon: "🛡️", color: "#99453a" },
    {
      id: "fire_resistance",
      name: "Fire Resistance",
      icon: "🔥",
      color: "#e49a3a",
    },
    {
      id: "water_breathing",
      name: "Water Breathing",
      icon: "🫧",
      color: "#2e5299",
    },
    { id: "invisibility", name: "Invisibility", icon: "👻", color: "#7f8392" },
    { id: "blindness", name: "Blindness", icon: "🙈", color: "#1f1f23" },
    { id: "night_vision", name: "Night Vision", icon: "👁️", color: "#1f1fa1" },
    { id: "hunger", name: "Hunger", icon: "🍖", color: "#587653" },
    { id: "weakness", name: "Weakness", icon: "😰", color: "#484d48" },
    { id: "poison", name: "Poison", icon: "☠️", color: "#4e9331" },
    { id: "wither", name: "Wither", icon: "💀", color: "#352a27" },
    { id: "health_boost", name: "Health Boost", icon: "❤️", color: "#f87d23" },
    { id: "absorption", name: "Absorption", icon: "💛", color: "#2552a5" },
    { id: "saturation", name: "Saturation", icon: "🥩", color: "#f82423" },
    { id: "glowing", name: "Glowing", icon: "✨", color: "#94a061" },
    { id: "levitation", name: "Levitation", icon: "🎈", color: "#ceffff" },
    { id: "luck", name: "Luck", icon: "🍀", color: "#339900" },
    { id: "bad_luck", name: "Bad Luck", icon: "🔮", color: "#c0a44d" },
    { id: "slow_falling", name: "Slow Falling", icon: "🪶", color: "#f7f8e0" },
    {
      id: "conduit_power",
      name: "Conduit Power",
      icon: "🌊",
      color: "#1dc2d1",
    },
    {
      id: "dolphins_grace",
      name: "Dolphin's Grace",
      icon: "🐬",
      color: "#88a3be",
    },
    {
      id: "hero_of_the_village",
      name: "Hero of the Village",
      icon: "🏆",
      color: "#44ff44",
    },
  ];

  function formatDuration(ticks: number): string {
    if (ticks < 0) return "∞";
    const seconds = Math.floor(ticks / 20);
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = seconds % 60;
    return `${minutes}:${remainingSeconds.toString().padStart(2, "0")}`;
  }

  function formatLevel(amplifier: number): string {
    const levels = [
      "I",
      "II",
      "III",
      "IV",
      "V",
      "VI",
      "VII",
      "VIII",
      "IX",
      "X",
    ];
    return levels[amplifier] || (amplifier + 1).toString();
  }

  function getEffectInfo(id: string) {
    const clean = id.replace("minecraft:", "");
    return (
      availableEffects.find((e) => e.id === clean) || {
        id: clean,
        name: clean.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase()),
        icon: "🧪",
        color: "#888888",
      }
    );
  }

  function handleDurationChange(effect: PotionEffect, newDuration: number) {
    effect.duration = newDuration;
    onchange(effect);
  }

  function handleAmplifierChange(effect: PotionEffect, newAmp: number) {
    effect.amplifier = Math.max(0, newAmp);
    onchange(effect);
  }

  function addEffect(effectId: string) {
    onadd(`minecraft:${effectId}`);
    showAddMenu = false;
    searchQuery = "";
  }

  let filteredEffects = $derived(
    availableEffects.filter(
      (e) =>
        e.name.toLowerCase().includes(searchQuery.toLowerCase()) &&
        !effects.some((existing) => existing.id.includes(e.id))
    )
  );
</script>

<div class="effects-panel">
  <div class="panel-header">
    <h4 class="panel-title">✨ Active Effects</h4>
    <button class="add-btn" onclick={() => (showAddMenu = !showAddMenu)}>
      {showAddMenu ? "✕ Close" : "+ Add Effect"}
    </button>
  </div>

  {#if showAddMenu}
    <div class="add-menu">
      <input
        type="text"
        class="search-input"
        placeholder="Search effects..."
        bind:value={searchQuery}
      />
      <div class="effects-list available">
        {#each filteredEffects as eff}
          <button class="effect-option" onclick={() => addEffect(eff.id)}>
            <span class="effect-icon" style="color: {eff.color}"
              >{eff.icon}</span
            >
            <span class="effect-name">{eff.name}</span>
          </button>
        {/each}
        {#if filteredEffects.length === 0}
          <p class="no-results">No matching effects</p>
        {/if}
      </div>
    </div>
  {/if}

  <div class="effects-list active">
    {#if effects.length === 0}
      <p class="no-effects">No active effects</p>
    {:else}
      {#each effects as effect}
        {@const info = getEffectInfo(effect.id)}
        <div
          class="effect-row"
          class:modified={modifiedEffects.includes(effect.id)}
          style="--effect-color: {info.color}"
        >
          <div class="effect-main">
            <span class="effect-icon">{info.icon}</span>
            <span class="effect-name">{info.name}</span>
            <span class="effect-level">{formatLevel(effect.amplifier)}</span>
          </div>

          <div class="effect-controls">
            <div class="control-group">
              <label class="control-label">Level</label>
              <div class="level-buttons">
                <button
                  class="level-btn"
                  onclick={() =>
                    handleAmplifierChange(effect, effect.amplifier - 1)}
                  disabled={effect.amplifier <= 0}>−</button
                >
                <span class="level-value">{effect.amplifier + 1}</span>
                <button
                  class="level-btn"
                  onclick={() =>
                    handleAmplifierChange(effect, effect.amplifier + 1)}
                  >+</button
                >
              </div>
            </div>

            <div class="control-group">
              <label class="control-label">Duration</label>
              <div class="duration-input">
                <input
                  type="number"
                  class="duration-field"
                  value={effect.duration < 0
                    ? -1
                    : Math.floor(effect.duration / 20)}
                  min="-1"
                  oninput={(e) => {
                    const val = parseInt((e.target as HTMLInputElement).value);
                    handleDurationChange(effect, val < 0 ? -1 : val * 20);
                  }}
                />
                <span class="duration-unit">sec</span>
              </div>
            </div>

            <button
              class="remove-btn"
              onclick={() => onremove(effect.id)}
              title="Remove effect">🗑</button
            >
          </div>
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .effects-panel {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .panel-title {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
    color: var(--color-text-1);
  }

  .add-btn {
    padding: 6px 12px;
    font-size: 0.8rem;
    font-weight: 500;
    background: var(--color-bg-3);
    border: 1px solid var(--color-bg-4);
    border-radius: var(--radius-sm);
    color: var(--color-text-2);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .add-btn:hover {
    background: var(--color-bg-4);
  }

  /* Add Menu */
  .add-menu {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px;
    background: var(--color-bg-2);
    border: 1px solid var(--color-bg-4);
    border-radius: var(--radius-md);
    max-height: 250px;
    overflow-y: auto;
  }

  .search-input {
    padding: 8px 12px;
    font-size: 0.85rem;
    background: var(--color-bg-1);
    border: 1px solid var(--color-bg-4);
    border-radius: var(--radius-sm);
    color: var(--color-text-1);
  }

  .search-input:focus {
    outline: none;
    border-color: var(--color-primary);
  }

  .effects-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .effects-list.available {
    max-height: 180px;
    overflow-y: auto;
  }

  .effect-option {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    cursor: pointer;
    text-align: left;
    transition: all 0.15s ease;
  }

  .effect-option:hover {
    background: var(--color-bg-3);
    border-color: var(--color-bg-4);
  }

  .no-results,
  .no-effects {
    text-align: center;
    color: var(--color-text-3);
    font-size: 0.85rem;
    padding: 16px;
  }

  /* Effect Row */
  .effect-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 10px 14px;
    background: var(--color-bg-2);
    border: 1px solid var(--color-bg-3);
    border-left: 3px solid var(--effect-color, var(--color-bg-4));
    border-radius: var(--radius-sm);
    transition: all 0.15s ease;
  }

  .effect-row:hover {
    background: var(--color-bg-3);
  }

  .effect-row.modified {
    border-left-color: var(--modified-color);
  }

  .effect-main {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 150px;
  }

  .effect-icon {
    font-size: 1.1rem;
  }

  .effect-name {
    font-weight: 600;
    font-size: 0.85rem;
    color: var(--color-text-1);
  }

  .effect-level {
    font-size: 0.75rem;
    font-weight: 700;
    color: var(--enchant-color);
    padding: 2px 6px;
    background: rgba(167, 139, 250, 0.15);
    border-radius: 4px;
  }

  /* Controls */
  .effect-controls {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .control-group {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .control-label {
    font-size: 0.65rem;
    font-weight: 600;
    color: var(--color-text-3);
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  .level-buttons {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .level-btn {
    width: 24px;
    height: 24px;
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 1rem;
    font-weight: 600;
    background: var(--color-bg-3);
    border: 1px solid var(--color-bg-4);
    border-radius: 4px;
    color: var(--color-text-2);
    cursor: pointer;
    transition: all 0.1s ease;
  }

  .level-btn:hover:not(:disabled) {
    background: var(--color-bg-4);
  }

  .level-btn:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  .level-value {
    min-width: 20px;
    text-align: center;
    font-family: "SF Mono", monospace;
    font-size: 0.85rem;
    font-weight: 600;
  }

  .duration-input {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .duration-field {
    width: 60px;
    padding: 4px 8px;
    font-family: "SF Mono", monospace;
    font-size: 0.8rem;
    background: var(--color-bg-1);
    border: 1px solid var(--color-bg-4);
    border-radius: 4px;
    color: var(--color-text-1);
    text-align: center;
  }

  .duration-field:focus {
    outline: none;
    border-color: var(--color-primary);
  }

  .duration-unit {
    font-size: 0.7rem;
    color: var(--color-text-3);
  }

  .remove-btn {
    padding: 6px;
    font-size: 0.9rem;
    background: transparent;
    border: none;
    cursor: pointer;
    opacity: 0.5;
    transition: opacity 0.15s ease;
  }

  .remove-btn:hover {
    opacity: 1;
  }
</style>
