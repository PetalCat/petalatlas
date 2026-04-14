<script lang="ts">
  import type { InventoryItem } from "./ItemSlot.svelte";

  let {
    item = $bindable<InventoryItem | null>(null),
    visible = $bindable(false),
    onclose = () => {},
    onsave = () => {},
    ondelete = () => {},
  } = $props<{
    item: InventoryItem | null;
    visible?: boolean;
    onclose?: () => void;
    onsave?: (item: InventoryItem) => void;
    ondelete?: () => void;
  }>();

  let editedItem = $state<InventoryItem | null>(null);
  let searchQuery = $state("");
  let showItemPicker = $state(false);
  let newEnchantId = $state("");
  let newEnchantLevel = $state(1);

  // Common items for quick picker
  const commonItems = [
    "minecraft:diamond_sword",
    "minecraft:diamond_pickaxe",
    "minecraft:diamond_axe",
    "minecraft:netherite_sword",
    "minecraft:netherite_pickaxe",
    "minecraft:elytra",
    "minecraft:diamond_helmet",
    "minecraft:diamond_chestplate",
    "minecraft:diamond_leggings",
    "minecraft:diamond_boots",
    "minecraft:golden_apple",
    "minecraft:enchanted_golden_apple",
    "minecraft:totem_of_undying",
    "minecraft:diamond",
    "minecraft:emerald",
    "minecraft:netherite_ingot",
    "minecraft:ender_pearl",
    "minecraft:blaze_rod",
    "minecraft:shulker_shell",
  ];

  // Common enchantments
  const enchantments = [
    { id: "sharpness", name: "Sharpness", maxLevel: 5 },
    { id: "smite", name: "Smite", maxLevel: 5 },
    { id: "bane_of_arthropods", name: "Bane of Arthropods", maxLevel: 5 },
    { id: "knockback", name: "Knockback", maxLevel: 2 },
    { id: "fire_aspect", name: "Fire Aspect", maxLevel: 2 },
    { id: "looting", name: "Looting", maxLevel: 3 },
    { id: "sweeping", name: "Sweeping Edge", maxLevel: 3 },
    { id: "efficiency", name: "Efficiency", maxLevel: 5 },
    { id: "silk_touch", name: "Silk Touch", maxLevel: 1 },
    { id: "fortune", name: "Fortune", maxLevel: 3 },
    { id: "unbreaking", name: "Unbreaking", maxLevel: 3 },
    { id: "mending", name: "Mending", maxLevel: 1 },
    { id: "protection", name: "Protection", maxLevel: 4 },
    { id: "fire_protection", name: "Fire Protection", maxLevel: 4 },
    { id: "blast_protection", name: "Blast Protection", maxLevel: 4 },
    { id: "projectile_protection", name: "Projectile Protection", maxLevel: 4 },
    { id: "feather_falling", name: "Feather Falling", maxLevel: 4 },
    { id: "thorns", name: "Thorns", maxLevel: 3 },
    { id: "respiration", name: "Respiration", maxLevel: 3 },
    { id: "aqua_affinity", name: "Aqua Affinity", maxLevel: 1 },
    { id: "depth_strider", name: "Depth Strider", maxLevel: 3 },
    { id: "frost_walker", name: "Frost Walker", maxLevel: 2 },
    { id: "soul_speed", name: "Soul Speed", maxLevel: 3 },
    { id: "power", name: "Power", maxLevel: 5 },
    { id: "punch", name: "Punch", maxLevel: 2 },
    { id: "flame", name: "Flame", maxLevel: 1 },
    { id: "infinity", name: "Infinity", maxLevel: 1 },
    { id: "loyalty", name: "Loyalty", maxLevel: 3 },
    { id: "riptide", name: "Riptide", maxLevel: 3 },
    { id: "channeling", name: "Channeling", maxLevel: 1 },
    { id: "impaling", name: "Impaling", maxLevel: 5 },
    { id: "multishot", name: "Multishot", maxLevel: 1 },
    { id: "quick_charge", name: "Quick Charge", maxLevel: 3 },
    { id: "piercing", name: "Piercing", maxLevel: 4 },
  ];

  $effect(() => {
    if (visible && item) {
      editedItem = JSON.parse(JSON.stringify(item));
    } else if (visible && !item) {
      editedItem = { id: "", count: 1 };
    }
  });

  $effect(() => {
    if (visible && editedItem && !editedItem.id) {
      queueMicrotask(() => {
        document.querySelector<HTMLInputElement>(".text-input")?.focus();
      });
    }
  });

  function formatItemName(id: string): string {
    return id.replace("minecraft:", "").replace(/_/g, " ").toUpperCase();
  }

  function handleSave() {
    if (editedItem && editedItem.id) {
      onsave(editedItem);
    }
    visible = false;
    onclose();
  }

  function handleDelete() {
    ondelete();
    visible = false;
    onclose();
  }

  function handleClose() {
    visible = false;
    onclose();
  }

  function selectItem(itemId: string) {
    if (editedItem) {
      editedItem.id = itemId;
    }
    showItemPicker = false;
    searchQuery = "";
  }

  function addEnchantment() {
    if (!editedItem || !newEnchantId) return;
    if (!editedItem.enchantments) editedItem.enchantments = [];

    // Check if already exists
    const existing = editedItem.enchantments.find(
      (e) => e.id === `minecraft:${newEnchantId}`
    );
    if (existing) {
      existing.level = newEnchantLevel;
    } else {
      editedItem.enchantments.push({
        id: `minecraft:${newEnchantId}`,
        level: newEnchantLevel,
      });
    }

    editedItem = { ...editedItem };
    newEnchantId = "";
    newEnchantLevel = 1;
  }

  $effect(() => {
    newEnchantLevel = Math.min(newEnchantLevel, 10);
  });

  function removeEnchantment(enchId: string) {
    if (!editedItem?.enchantments) return;
    editedItem.enchantments = editedItem.enchantments.filter(
      (e) => e.id !== enchId
    );
    editedItem = { ...editedItem };
  }

  let filteredItems = $derived(
    commonItems.filter((i) =>
      formatItemName(i).toLowerCase().includes(searchQuery.toLowerCase())
    )
  );

  let availableEnchants = $derived(
    enchantments.filter(
      (e) =>
        !editedItem?.enchantments?.some((ex) => ex.id === `minecraft:${e.id}`)
    )
  );
</script>

{#if visible}
  <div class="modal-backdrop" onclick={handleClose}>
    <div class="item-editor" onclick={(e) => e.stopPropagation()}>
      <div class="editor-header">
        <h3 class="editor-title">{item ? "Edit Item" : "Add Item"}</h3>
        <button class="close-btn" onclick={handleClose}>✕</button>
      </div>

      {#if editedItem}
        <div class="editor-content">
          <!-- Item ID -->
          <div class="field-group">
            <label class="field-label">Item ID</label>
            <div class="item-id-field">
              <input
                type="text"
                class="text-input"
                bind:value={editedItem.id}
                placeholder="minecraft:diamond_sword"
              />
              <button
                class="picker-btn"
                onclick={() => (showItemPicker = !showItemPicker)}
              >
                {showItemPicker ? "✕" : "📦"}
              </button>
            </div>

            {#if showItemPicker}
              <div class="item-picker">
                <input
                  type="text"
                  class="search-input"
                  placeholder="Search items..."
                  bind:value={searchQuery}
                />
                <div class="item-grid">
                  {#each filteredItems as itemId}
                    <button
                      class="item-option"
                      class:selected={editedItem.id === itemId}
                      onclick={() => selectItem(itemId)}
                    >
                      {formatItemName(itemId)}
                    </button>
                  {/each}
                </div>
              </div>
            {/if}
          </div>

          <!-- Count & Damage -->
          <div class="field-row">
            <div class="field-group">
              <label class="field-label">Count</label>
              <input
                type="number"
                class="number-input"
                bind:value={editedItem.count}
                min="1"
                max="64"
              />
            </div>

            <div class="field-group">
              <label class="field-label">Damage</label>
              <input
                type="number"
                class="number-input"
                bind:value={editedItem.damage}
                min="0"
              />
            </div>
          </div>

          <!-- Custom Name -->
          <div class="field-group">
            <label class="field-label">Custom Name (optional)</label>
            <input
              type="text"
              class="text-input"
              bind:value={editedItem.customName}
              placeholder="Custom display name"
            />
          </div>

          <!-- Enchantments -->
          <div class="field-group">
            <label class="field-label">Enchantments</label>

            {#if editedItem.enchantments && editedItem.enchantments.length > 0}
              <div class="enchant-list">
                {#each editedItem.enchantments as ench}
                  <div class="enchant-row">
                    <span class="enchant-name">
                      {enchantments.find((e) => `minecraft:${e.id}` === ench.id)
                        ?.name || ench.id}
                    </span>
                    <span class="enchant-level">{ench.level}</span>
                    <button
                      class="remove-btn"
                      onclick={() => removeEnchantment(ench.id)}>✕</button
                    >
                  </div>
                {/each}
              </div>
            {/if}

            <div class="add-enchant">
              <select class="enchant-select" bind:value={newEnchantId}>
                <option value="">Add enchantment...</option>
                {#each availableEnchants as ench}
                  <option value={ench.id}>{ench.name}</option>
                {/each}
              </select>
              <input
                type="number"
                class="level-input"
                bind:value={newEnchantLevel}
                min="1"
                max="10"
              />
              <button
                class="add-btn"
                onclick={addEnchantment}
                disabled={!newEnchantId}>+</button
              >
            </div>
          </div>
        </div>

        <div class="editor-footer">
          {#if item}
            <button class="delete-btn" onclick={handleDelete}>🗑 Delete</button>
          {/if}
          <div class="footer-right">
            <button class="cancel-btn" onclick={handleClose}>Cancel</button>
            <button
              class="save-btn"
              onclick={handleSave}
              disabled={!editedItem.id}
            >
              Save
            </button>
          </div>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    backdrop-filter: blur(20px); /* Slightly stronger blur for Tahoe feel */
    background: rgba(0, 0, 0, 0.4);
  }

  .item-editor {
    width: 90%;
    max-width: 500px;
    max-height: 85vh;
    max-height: 85vh;
    background: var(--color-bg-1); /* System BG */
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-xl);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border: 1px solid var(--color-bg-3); /* Subtle border */
  }

  .editor-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid var(--color-bg-3);
    background: var(--color-bg-2);
  }

  .editor-title {
    margin: 0;
    font-size: 0.95rem;
    font-weight: 600;
    letter-spacing: 0.02em;
  }

  .close-btn {
    width: 28px;
    height: 28px;
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 1rem;
    background: transparent;
    border: none;
    color: var(--color-text-3);
    cursor: pointer;
    border-radius: 4px;
    transition: all 0.15s ease;
  }

  .close-btn:hover {
    background: var(--color-bg-3);
    color: var(--color-text-1);
  }

  .editor-content {
    flex: 1;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    overflow-y: auto;
  }

  .field-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding-bottom: 4px;
  }

  .field-group:not(:last-child) {
    border-bottom: 1px dashed var(--color-bg-3);
  }

  .field-row {
    display: flex;
    gap: 16px;
  }

  .field-row .field-group {
    flex: 1;
  }

  .field-label {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--color-text-3);
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  .text-input,
  .number-input {
    padding: 10px 12px;
    font-size: 0.9rem;
    font-family: "SF Mono", monospace;
    background: var(--color-bg-2);
    border: 1px solid var(--color-bg-4);
    border-radius: var(--radius-sm);
    color: var(--color-text-1);
  }

  .text-input:focus,
  .number-input:focus {
    outline: none;
    border-color: var(--color-primary);
  }

  .number-input {
    width: 100%;
  }

  /* Item ID with picker */
  .item-id-field {
    display: flex;
    gap: 8px;
  }

  .item-id-field .text-input {
    flex: 1;
  }

  .picker-btn {
    width: 40px;
    padding: 0;
    font-size: 1.1rem;
    background: var(--color-bg-3);
    border: 1px solid var(--color-bg-4);
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .picker-btn:hover {
    background: var(--color-bg-4);
  }

  /* Item Picker */
  .item-picker {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px;
    background: var(--color-bg-2);
    border: 1px solid var(--color-bg-3);
    border-radius: var(--radius-sm);
  }

  .search-input {
    padding: 8px 10px;
    font-size: 0.85rem;
    background: var(--color-bg-2);
    border: 1px solid var(--color-bg-4);
    border-radius: 4px;
    color: var(--color-text-1);
    box-shadow: inset 0 0 0 1px var(--color-bg-4);
  }

  .item-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px;
    max-height: 150px;
    overflow-y: auto;
  }

  .item-option {
    padding: 8px;
    font-size: 0.75rem;
    font-weight: 500;
    text-align: left;
    background: var(--color-bg-3);
    border: 1px solid transparent;
    border-radius: 4px;
    cursor: pointer;
    line-height: 1.2;
    transition: all 0.1s ease;
  }

  .item-option:hover {
    background: var(--color-bg-4);
  }

  .item-option.selected {
    background: var(--color-primary);
    border-color: var(--color-primary);
    color: white;
  }

  /* Enchantments */
  .enchant-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 8px;
  }

  .enchant-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    background: var(--color-bg-2); /* Remove custom gradient */
    border: 1px solid var(--color-bg-3);
    border-radius: 4px;
  }

  .enchant-name {
    flex: 1;
    font-size: 0.8rem;
    color: var(--color-text-1); /* Use system text color */
    font-style: normal; /* Remove italic */
  }

  .enchant-level {
    font-size: 0.75rem;
    font-weight: 700;
    color: var(--enchant-color);
    padding: 2px 6px;
    background: rgba(167, 139, 250, 0.15);
    border-radius: 4px;
  }

  .enchant-row .remove-btn {
    width: 20px;
    height: 20px;
    padding: 0;
    font-size: 0.7rem;
    background: transparent;
    border: none;
    color: var(--color-text-3);
    cursor: pointer;
    opacity: 0.5;
    transition: opacity 0.15s ease;
  }

  .enchant-row:hover .remove-btn {
    opacity: 1;
  }

  .add-enchant {
    display: flex;
    gap: 8px;
  }

  .enchant-select {
    flex: 1;
    padding: 8px 10px;
    font-size: 0.85rem;
    background: var(--color-bg-2);
    border: 1px solid var(--color-bg-4);
    border-radius: 4px;
    color: var(--color-text-1);
    cursor: pointer;
  }

  .level-input {
    width: 50px;
    padding: 8px;
    font-size: 0.85rem;
    font-family: "SF Mono", monospace;
    text-align: center;
    background: var(--color-bg-2);
    border: 1px solid var(--color-bg-4);
    border-radius: 4px;
    color: var(--color-text-1);
  }

  .add-btn {
    width: 36px;
    padding: 0;
    font-size: 1.2rem;
    font-weight: 600;
    background: var(--color-green);
    border: none;
    border-radius: 4px;
    color: white;
    cursor: pointer;
    transition: filter 0.15s ease;
  }

  .add-btn:hover:not(:disabled) {
    filter: brightness(1.1);
  }

  .add-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Footer */
  .editor-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-top: 1px solid var(--color-bg-3);
    background: var(--color-bg-1);
  }

  .footer-right {
    display: flex;
    gap: 8px;
    margin-left: auto;
  }

  .delete-btn,
  .cancel-btn,
  .save-btn {
    padding: 8px 16px;
    font-size: 0.85rem;
    font-weight: 500;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .delete-btn {
    background: transparent;
    border: 1px solid var(--color-red);
    color: var(--color-red);
    opacity: 0.7;
  }

  .delete-btn:hover {
    background: var(--color-red);
    color: white;
    opacity: 1;
  }

  .cancel-btn {
    background: transparent;
    border: 1px solid var(--color-bg-4);
    color: var(--color-text-2);
  }

  .cancel-btn:hover {
    background: var(--color-bg-3);
  }

  .save-btn {
    background: var(--color-primary);
    border: none;
    color: white;
  }

  .save-btn:hover:not(:disabled) {
    filter: brightness(1.1);
  }

  .save-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
