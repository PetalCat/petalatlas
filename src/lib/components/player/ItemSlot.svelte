<script lang="ts">
  import { loadItemTexture } from "$lib/textureHelper.svelte";

  export interface InventoryItem {
    id: string;
    count: number;
    enchantments?: { id: string; level: number }[];
    customName?: string;
    damage?: number;
    maxDamage?: number;
    tag?: any;
  }

  let {
    item = null,
    selected = false,
    modified = false,
    onselect = () => {},
    onedit = () => {},
    onhover = (hovering: boolean) => {},
  } = $props<{
    item: InventoryItem | null;
    selected?: boolean;
    modified?: boolean;
    onselect?: () => void;
    onedit?: (item: InventoryItem | null) => void;
    onhover?: (hovering: boolean) => void;
  }>();

  function formatName(id: string) {
    return id
      .replace(/^[^:]+:/, "")
      .replace(/_/g, " ")
      .replace(/\b\w/g, (c) => c.toUpperCase());
  }

  function getDurabilityColor(damage: number, max: number) {
    const fraction = Math.max(0, Math.min(1, 1 - damage / max));
    const hue = Math.floor(fraction * 120);
    return `hsl(${hue}, 100%, 50%)`;
  }

  let enchanted = $derived(!!item?.enchantments?.length);

  // ===== TEXTURE STATE =====
  let textureUrl = $state<string | null>(null);

  $effect(() => {
    textureUrl = null;
    if (!item?.id) return;

    let cancelled = false;

    loadItemTexture(item.id).then((url) => {
      if (!cancelled) textureUrl = url;
    });

    return () => {
      cancelled = true;
    };
  });
</script>

<div
  class="slot"
  class:empty={!item}
  class:selected
  class:modified
  class:enchanted
  role="button"
  tabindex="0"
  onclick={onselect}
  ondblclick={() => onedit(item)}
  onmouseenter={() => onhover(true)}
  onmouseleave={() => onhover(false)}
  onkeydown={(e) => {
    if (e.key === "Enter" || e.key === " ") {
      onselect();
      e.preventDefault();
    }
  }}
>
  <div class="bg"></div>

  {#if enchanted}
    <div class="glint"></div>
  {/if}

  {#if item}
    <div class="visual">
      {#if textureUrl}
        <img src={textureUrl} alt="" draggable="false" />
      {:else}
        <div class="fallback-tile"></div>
      {/if}
    </div>

    {#if item.count > 1}
      <div class="count">{item.count}</div>
    {/if}

    {#if item.maxDamage && (item.damage || 0) > 0}
      <div class="durability-bar">
        <div
          class="durability-fill"
          style="width: {Math.max(
            0,
            100 * (1 - (item.damage || 0) / item.maxDamage)
          )}%; background: {getDurabilityColor(
            item.damage || 0,
            item.maxDamage
          )}"
        ></div>
      </div>
    {/if}

    <div class="label">
      {item.customName ?? formatName(item.id)}
    </div>
  {:else}
    <div class="empty-slot"></div>
  {/if}
</div>

<style>
  .slot {
    position: relative;
    width: 100%;
    height: 100%;
    background: var(--color-bg-1); /* Recessed look */
    border: 1px solid var(--color-bg-3);
    border-radius: var(--radius-md); /* System radius */
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .slot:hover {
    border-color: var(--color-text-3);
    transform: translateY(-1px);
    box-shadow: var(--shadow-sm);
  }

  .slot.selected {
    border-color: var(--color-primary);
    box-shadow: 0 0 0 2px var(--color-primary-light);
  }

  .slot.modified {
    border-color: var(--color-yellow);
    box-shadow: 0 0 0 2px rgba(255, 204, 0, 0.2);
  }

  .bg {
    position: absolute;
    inset: 0;
    background: linear-gradient(
      180deg,
      rgba(255, 255, 255, 0.04),
      rgba(0, 0, 0, 0.25)
    );
  }

  .visual {
    width: 72%;
    height: 72%;
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1;
  }

  .visual img {
    max-width: 100%;
    max-height: 100%;
    image-rendering: pixelated;
    object-fit: contain;
  }

  .fallback-tile {
    width: 100%;
    height: 100%;
    border-radius: 3px;
    background: linear-gradient(
      135deg,
      rgba(255, 255, 255, 0.08),
      rgba(0, 0, 0, 0.25)
    );
    box-shadow:
      inset 0 0 0 1px rgba(255, 255, 255, 0.12),
      inset 0 0 8px rgba(0, 0, 0, 0.4);
  }

  .count {
    position: absolute;
    bottom: 2px;
    right: 4px;
    font-size: 0.75rem;
    font-weight: 700;
    color: #fff;
    text-shadow:
      0 1px 1px rgba(0, 0, 0, 0.8),
      0 0 1px rgba(0, 0, 0, 0.5);
    z-index: 2;
    pointer-events: none;
    line-height: 1;
  }

  .durability-bar {
    position: absolute;
    bottom: 2px;
    left: 2px;
    right: 2px;
    height: 2px;
    background: #000;
    z-index: 2;
  }

  .durability-fill {
    height: 100%;
    transition:
      width 0.2s ease,
      background 0.2s ease;
  }

  .label {
    position: absolute;
    left: 2px;
    right: 2px;
    bottom: -2px;
    font-size: 0.55rem;
    font-weight: 600;
    color: #e0e0e0;
    text-align: center;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    opacity: 0;
    transform: translateY(6px);
    transition:
      opacity 0.12s ease,
      transform 0.12s ease;
    pointer-events: none;
  }

  .slot:hover .label {
    opacity: 1;
    transform: translateY(0);
  }

  .empty-slot {
    width: 60%;
    height: 60%;
    border-radius: 3px;
    background: linear-gradient(
      135deg,
      rgba(255, 255, 255, 0.02),
      rgba(0, 0, 0, 0.3)
    );
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.06);
  }

  .glint {
    position: absolute;
    inset: -40%;
    background: linear-gradient(
      120deg,
      transparent 40%,
      rgba(180, 140, 255, 0.35) 50%,
      transparent 60%
    );
    transform: translateX(-60%);
    animation: glint 2.6s linear infinite;
    mix-blend-mode: screen;
    pointer-events: none;
  }

  @keyframes glint {
    from {
      transform: translateX(-60%);
    }
    to {
      transform: translateX(60%);
    }
  }
</style>
