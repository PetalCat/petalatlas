<script lang="ts">
  import ItemSlot, { type InventoryItem } from "./ItemSlot.svelte";

  type DensityMode = "compact" | "comfortable" | "expanded";

  let {
    inventory = [],
    hotbar = [],
    armor = [],
    offhand = null,
    selectedSlot = $bindable<number | null>(null),
    modifiedSlots = [],
    onitemchange = () => {},
    onslotselect = () => {},
    onitemhover = () => {},
  } = $props<{
    inventory: (InventoryItem | null)[];
    hotbar: (InventoryItem | null)[];
    armor: (InventoryItem | null)[];
    offhand: InventoryItem | null;
    selectedSlot?: number | null;
    modifiedSlots?: number[];
    onitemchange?: (slot: number, item: InventoryItem | null) => void;
    onslotselect?: (slot: number) => void;
    onitemhover?: (item: InventoryItem | null) => void;
  }>();

  function handleSelect(slot: number) {
    selectedSlot = slot;
    onslotselect(slot);
  }

  function handleEdit(slot: number, item: InventoryItem | null) {
    onitemchange(slot, item);
  }

  function handleHover(item: InventoryItem | null, hovering: boolean) {
    if (hovering) {
      onitemhover(item);
    } else {
      onitemhover(null);
    }
  }

  let paddedInventory = $derived(
    [...inventory, ...Array(27 - inventory.length).fill(null)].slice(0, 27)
  );

  let paddedHotbar = $derived(
    [...hotbar, ...Array(9 - hotbar.length).fill(null)].slice(0, 9)
  );
</script>

<div class="inventory-root">
  <div class="inventory-panel">
    <section class="block">
      <h4>Inventory</h4>
      <div class="grid grid-9x3">
        {#each paddedInventory as item, i}
          <ItemSlot
            {item}
            selected={selectedSlot === 9 + i}
            modified={modifiedSlots.includes(9 + i)}
            onselect={() => handleSelect(9 + i)}
            onedit={(v) => handleEdit(9 + i, v)}
            onhover={(h) => handleHover(item, h)}
          />
        {/each}
      </div>
    </section>

    <section class="block">
      <h4>Hotbar</h4>
      <div class="grid grid-9x1">
        {#each paddedHotbar as item, i}
          <ItemSlot
            {item}
            selected={selectedSlot === i}
            modified={modifiedSlots.includes(i)}
            onselect={() => handleSelect(i)}
            onedit={(v) => handleEdit(i, v)}
            onhover={(h) => handleHover(item, h)}
          />
        {/each}
      </div>
    </section>
  </div>
</div>

<style>
  .inventory-root {
    display: flex;
    justify-content: center;

    /* Hardcoded Comfortable Density */
    --slot-size: 56px;
    --slot-gap: 6px;
  }

  .inventory-panel {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 12px;
    background: var(--color-bg-2); /* System background */
    border: 1px solid var(--color-bg-3); /* Softer system border */
    border-radius: var(--radius-lg); /* System radius */
    box-shadow: var(--shadow-sm); /* System shadow */
    width: fit-content;
  }

  .block {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 6px 0;
  }

  .block + .block {
    border-top: 1px solid var(--color-bg-3);
    padding-top: 12px;
  }

  h4 {
    margin: 0;
    font-size: 0.7rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--color-text-3);
  }

  /* ========================= */
  /* HARD GRID LOCKING */
  /* ========================= */

  .grid {
    display: grid;
    gap: var(--slot-gap);
    align-items: start;
    justify-items: start;
  }

  .grid-9x3 {
    grid-template-columns: repeat(9, var(--slot-size)) !important;
    grid-auto-rows: var(--slot-size);
  }

  .grid-9x1 {
    grid-template-columns: repeat(9, var(--slot-size)) !important;
    grid-auto-rows: var(--slot-size);
  }

  /* FORCE CHILD COMPONENT COMPLIANCE */
  .grid > :global(*) {
    width: var(--slot-size) !important;
    height: var(--slot-size) !important;
    max-width: var(--slot-size);
    max-height: var(--slot-size);
    flex: none !important;
  }

  /* ========================= */

  @media (max-width: 900px) {
    .inventory-panel {
      grid-template-columns: auto;
    }
  }
</style>
