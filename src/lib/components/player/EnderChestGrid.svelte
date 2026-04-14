<script lang="ts">
  import ItemSlot, { type InventoryItem } from "./ItemSlot.svelte";

  type DensityMode = "compact" | "comfortable" | "expanded";

  let {
    items = [],
    density = "comfortable",
    selectedSlot = $bindable<number | null>(null),
    modifiedSlots = [],
    onitemchange = () => {},
    onslotselect = () => {},
  } = $props<{
    items: (InventoryItem | null)[];
    density?: DensityMode;
    selectedSlot?: number | null;
    modifiedSlots?: number[];
    onitemchange?: (slot: number, item: InventoryItem | null) => void;
    onslotselect?: (slot: number) => void;
  }>();

  // Ender chest has 27 slots (9x3)
  let paddedItems = $derived(
    [...items, ...Array(27 - items.length).fill(null)].slice(0, 27)
  );

  function handleSelect(slotIndex: number) {
    selectedSlot = slotIndex;
    onslotselect(slotIndex);
  }

  function handleEdit(slotIndex: number, item: InventoryItem | null) {
    onitemchange(slotIndex, item);
  }
</script>

<div class="ender-chest">
  <div class="chest-header">
    <span class="chest-icon">🟣</span>
    <h4 class="chest-title">Ender Chest</h4>
    <span class="chest-count"
      >{items.filter((i) => i !== null).length}/27 slots used</span
    >
  </div>

  <div class="chest-grid">
    {#each paddedItems as item, i}
      <ItemSlot
        {item}
        slotIndex={i}
        {density}
        selected={selectedSlot === i}
        modified={modifiedSlots.includes(i)}
        onselect={() => handleSelect(i)}
        onedit={(newItem) => handleEdit(i, newItem)}
      />
    {/each}
  </div>
</div>

<style>
  .ender-chest {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px;
    background: linear-gradient(
      135deg,
      rgba(128, 90, 213, 0.1) 0%,
      rgba(64, 45, 106, 0.1) 100%
    );
    border: 1px solid rgba(128, 90, 213, 0.3);
    border-radius: var(--radius-lg);
  }

  .chest-header {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .chest-icon {
    font-size: 1.2rem;
  }

  .chest-title {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
    color: var(--color-text-1);
  }

  .chest-count {
    margin-left: auto;
    font-size: 0.75rem;
    color: var(--color-text-3);
  }

  .chest-grid {
    display: grid;
    grid-template-columns: repeat(9, 1fr);
    grid-template-rows: repeat(3, 1fr);
    gap: 4px;
  }

  /* Responsive */
  @media (max-width: 700px) {
    .chest-grid {
      grid-template-columns: repeat(3, 1fr);
      grid-template-rows: repeat(9, 1fr);
    }
  }
</style>
