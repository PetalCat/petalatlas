<script lang="ts">
  type SliderType = "health" | "hunger" | "xp";

  let {
    type = "health",
    value = $bindable(20),
    max = 20,
    level = 0, // For XP type only
    modified = false,
    onchange = () => {},
  } = $props<{
    type?: SliderType;
    value: number;
    max?: number;
    level?: number;
    modified?: boolean;
    onchange?: (value: number, level?: number) => void;
  }>();

  let isDragging = $state(false);
  let sliderRef = $state<HTMLDivElement | null>(null);

  const config = {
    health: {
      icon: "❤️",
      label: "Health",
      color: "var(--health-color)",
      bgColor: "rgba(239, 68, 68, 0.15)",
      segments: 10, // 10 hearts = 20 health
    },
    hunger: {
      icon: "🍖",
      label: "Hunger",
      color: "var(--hunger-color)",
      bgColor: "rgba(217, 119, 6, 0.15)",
      segments: 10, // 10 drumsticks = 20 food
    },
    xp: {
      icon: "⭐",
      label: "XP",
      color: "var(--xp-color)",
      bgColor: "rgba(34, 197, 94, 0.15)",
      segments: 0, // Continuous
    },
  };

  let cfg = $derived(config[type]);
  let percent = $derived(Math.min(100, Math.max(0, (value / max) * 100)));
  let displayValue = $derived(
    type === "xp" ? `${Math.round(percent)}%` : `${value}/${max}`
  );

  function handleClick(e: MouseEvent) {
    if (!sliderRef) return;
    const rect = sliderRef.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const newPercent = Math.max(0, Math.min(100, (x / rect.width) * 100));
    const newValue = Math.round((newPercent / 100) * max);
    value = newValue;
    onchange(newValue, type === "xp" ? level : undefined);
  }

  function handleMouseDown(e: MouseEvent) {
    isDragging = true;
    handleClick(e);

    const onMouseMove = (e: MouseEvent) => {
      if (isDragging && sliderRef) {
        const rect = sliderRef.getBoundingClientRect();
        const x = e.clientX - rect.left;
        const newPercent = Math.max(0, Math.min(100, (x / rect.width) * 100));
        const newValue = Math.round((newPercent / 100) * max);
        value = newValue;
      }
    };

    const onMouseUp = () => {
      isDragging = false;
      onchange(value, type === "xp" ? level : undefined);
      document.removeEventListener("mousemove", onMouseMove);
      document.removeEventListener("mouseup", onMouseUp);
    };

    document.addEventListener("mousemove", onMouseMove);
    document.addEventListener("mouseup", onMouseUp);
  }

  function handleLevelChange(e: Event) {
    const input = e.target as HTMLInputElement;
    const newLevel = parseInt(input.value) || 0;
    onchange(value, newLevel);
  }
</script>

<div class="value-slider {type}" class:modified class:dragging={isDragging}>
  <div class="slider-header">
    <span class="slider-icon">{cfg.icon}</span>
    <span class="slider-label">{cfg.label}</span>
    {#if type === "xp"}
      <div class="level-input">
        <span class="level-label">Level</span>
        <input
          type="number"
          class="level-field"
          value={level}
          min="0"
          max="21863"
          oninput={handleLevelChange}
        />
      </div>
    {/if}
    <span class="slider-value">{displayValue}</span>
  </div>

  <div
    class="slider-track"
    bind:this={sliderRef}
    onmousedown={handleMouseDown}
    role="slider"
    aria-valuenow={value}
    aria-valuemin={0}
    aria-valuemax={max}
    tabindex="0"
  >
    <!-- Segment markers (for health/hunger) -->
    {#if cfg.segments > 0}
      <div class="segments">
        {#each Array(cfg.segments) as _, i}
          {@const segmentPercent = ((i + 1) / cfg.segments) * 100}
          {@const isFilled = percent >= segmentPercent}
          {@const isHalf =
            percent >= segmentPercent - 5 && percent < segmentPercent}
          <div
            class="segment"
            class:filled={isFilled}
            class:half={isHalf}
            style="--segment-color: {cfg.color}"
          >
            <div class="segment-fill"></div>
          </div>
        {/each}
      </div>
    {:else}
      <!-- Continuous bar (for XP) -->
      <div class="bar-bg" style="background: {cfg.bgColor}">
        <div
          class="bar-fill"
          style="width: {percent}%; background: linear-gradient(90deg, {cfg.color} 0%, {cfg.color}dd 100%)"
        >
          <div class="bar-shine"></div>
        </div>
      </div>
    {/if}

    <!-- Drag handle -->
    <div class="slider-thumb" style="left: {percent}%; background: {cfg.color}">
      <div class="thumb-glow" style="background: {cfg.color}"></div>
    </div>
  </div>
</div>

<style>
  .value-slider {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px 16px;
    background: var(--color-bg-2);
    border-radius: var(--radius-md);
    border: 1px solid var(--color-bg-3);
    transition: all 0.2s ease;
  }

  .value-slider:hover {
    border-color: var(--color-bg-4);
    background: var(--color-bg-3);
  }

  .value-slider.modified {
    border-left: 3px solid var(--modified-color);
  }

  .value-slider.dragging {
    border-color: var(--color-primary);
  }

  /* Header */
  .slider-header {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .slider-icon {
    font-size: 1.1rem;
    line-height: 1;
  }

  .slider-label {
    font-weight: 600;
    font-size: 0.85rem;
    color: var(--color-text-1);
  }

  .slider-value {
    margin-left: auto;
    font-family: "SF Mono", "Monaco", monospace;
    font-size: 0.9rem;
    font-weight: 600;
    color: var(--color-text-2);
    font-variant-numeric: tabular-nums;
  }

  /* Level input for XP */
  .level-input {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-left: 16px;
  }

  .level-label {
    font-size: 0.75rem;
    color: var(--color-text-3);
  }

  .level-field {
    width: 60px;
    padding: 4px 8px;
    font-family: "SF Mono", "Monaco", monospace;
    font-size: 0.85rem;
    font-weight: 600;
    background: var(--color-bg-1);
    border: 1px solid var(--color-bg-4);
    border-radius: var(--radius-sm);
    color: var(--color-text-1);
    text-align: center;
  }

  .level-field:focus {
    outline: none;
    border-color: var(--color-primary);
  }

  /* Track */
  .slider-track {
    position: relative;
    height: 32px;
    cursor: pointer;
    user-select: none;
  }

  /* Segment-based (health/hunger) */
  .segments {
    display: flex;
    gap: 4px;
    height: 100%;
  }

  .segment {
    flex: 1;
    height: 100%;
    background: var(--color-bg-4);
    border-radius: 4px;
    overflow: hidden;
    transition: transform 0.1s ease;
  }

  .segment:hover {
    transform: scale(1.05);
  }

  .segment-fill {
    height: 100%;
    width: 0;
    background: var(--segment-color);
    transition: width 0.15s ease;
  }

  .segment.filled .segment-fill {
    width: 100%;
  }

  .segment.half .segment-fill {
    width: 50%;
  }

  /* Continuous bar (XP) */
  .bar-bg {
    height: 100%;
    border-radius: 6px;
    overflow: hidden;
    position: relative;
  }

  .bar-fill {
    height: 100%;
    border-radius: 6px;
    transition: width 0.15s ease;
    position: relative;
    overflow: hidden;
  }

  .bar-shine {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 50%;
    background: linear-gradient(
      180deg,
      rgba(255, 255, 255, 0.2) 0%,
      transparent 100%
    );
    border-radius: 6px 6px 0 0;
  }

  /* Thumb */
  .slider-thumb {
    position: absolute;
    top: 50%;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    transform: translate(-50%, -50%);
    border: 2px solid white;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.3);
    transition:
      transform 0.1s ease,
      box-shadow 0.1s ease;
    z-index: 2;
  }

  .slider-track:hover .slider-thumb,
  .dragging .slider-thumb {
    transform: translate(-50%, -50%) scale(1.2);
    box-shadow: 0 3px 10px rgba(0, 0, 0, 0.4);
  }

  .thumb-glow {
    position: absolute;
    inset: -4px;
    border-radius: 50%;
    opacity: 0;
    filter: blur(6px);
    transition: opacity 0.2s ease;
  }

  .slider-track:hover .thumb-glow,
  .dragging .thumb-glow {
    opacity: 0.4;
  }

  /* Type-specific animations */
  .health .segment.filled .segment-fill {
    animation: pulse-health 2s ease-in-out infinite;
  }

  @keyframes pulse-health {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.85;
    }
  }

  .hunger .segment.filled .segment-fill {
    animation: none;
  }

  .xp .bar-fill {
    animation: shimmer-xp 3s ease-in-out infinite;
  }

  @keyframes shimmer-xp {
    0%,
    100% {
      filter: brightness(1);
    }
    50% {
      filter: brightness(1.1);
    }
  }
</style>
