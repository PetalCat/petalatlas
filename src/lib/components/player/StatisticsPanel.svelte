<script lang="ts">
  export interface StatCategory {
    id: string;
    name: string;
    icon: string;
    stats: StatEntry[];
  }

  export interface StatEntry {
    id: string;
    name: string;
    value: number;
    modified?: boolean;
  }

  let {
    categories = [],
    searchQuery = $bindable(""),
    sortBy = $bindable<"name" | "value">("value"),
    onchange = () => {},
  } = $props<{
    categories: StatCategory[];
    searchQuery?: string;
    sortBy?: "name" | "value";
    onchange?: (categoryId: string, statId: string, newValue: number) => void;
  }>();

  let expandedCategories = $state<Set<string>>(
    new Set(["minecraft:mined", "minecraft:killed"])
  );
  let editingStat = $state<string | null>(null);
  let editValue = $state(0);

  function toggleCategory(catId: string) {
    if (expandedCategories.has(catId)) {
      expandedCategories.delete(catId);
    } else {
      expandedCategories.add(catId);
    }
    expandedCategories = new Set(expandedCategories);
  }

  function formatStatName(id: string): string {
    // Remove minecraft: prefix and format
    return id
      .replace("minecraft:", "")
      .replace(/_/g, " ")
      .replace(/\b\w/g, (c) => c.toUpperCase());
  }

  function formatNumber(n: number): string {
    return n.toLocaleString();
  }

  function startEdit(catId: string, statId: string, currentValue: number) {
    editingStat = `${catId}:${statId}`;
    editValue = currentValue;
  }

  function saveEdit(catId: string, statId: string) {
    onchange(catId, statId, editValue);
    editingStat = null;
  }

  function cancelEdit() {
    editingStat = null;
  }

  // Filter and sort stats
  function getFilteredStats(stats: StatEntry[]): StatEntry[] {
    let filtered = [...stats]; // Create a copy to avoid mutating the original array

    if (searchQuery) {
      const query = searchQuery.toLowerCase();
      filtered = filtered.filter(
        (s) =>
          formatStatName(s.id).toLowerCase().includes(query) ||
          s.id.toLowerCase().includes(query)
      );
    }

    // Sort a copy, not the original
    return filtered.toSorted((a, b) => {
      if (sortBy === "value") {
        return b.value - a.value;
      }
      return formatStatName(a.id).localeCompare(formatStatName(b.id));
    });
  }

  // Check if category has any matching stats
  function hasMatchingStats(cat: StatCategory): boolean {
    if (!searchQuery) return true;
    const query = searchQuery.toLowerCase();
    return cat.stats.some(
      (s) =>
        formatStatName(s.id).toLowerCase().includes(query) ||
        s.id.toLowerCase().includes(query)
    );
  }
</script>

<div class="statistics-panel">
  <div class="panel-header">
    <h4 class="panel-title">📊 Statistics</h4>
    <div class="header-controls">
      <input
        type="text"
        class="search-input"
        placeholder="Search stats..."
        bind:value={searchQuery}
      />
      <select class="sort-select" bind:value={sortBy}>
        <option value="value">Sort by Value</option>
        <option value="name">Sort by Name</option>
      </select>
    </div>
  </div>

  <div class="categories-list">
    {#each categories as cat}
      {#if hasMatchingStats(cat)}
        {@const isExpanded = expandedCategories.has(cat.id)}
        {@const filteredStats = getFilteredStats(cat.stats)}

        <div class="category">
          <button
            class="category-header"
            onclick={() => toggleCategory(cat.id)}
          >
            <span class="expand-icon">{isExpanded ? "▼" : "▶"}</span>
            <span class="category-icon">{cat.icon}</span>
            <span class="category-name">{cat.name}</span>
            <span class="category-count">{filteredStats.length} stats</span>
          </button>

          {#if isExpanded}
            <div class="stats-list">
              {#each filteredStats as stat}
                {@const editKey = `${cat.id}:${stat.id}`}
                <div class="stat-row" class:modified={stat.modified}>
                  <span class="stat-name">{formatStatName(stat.id)}</span>
                  <span class="stat-dots"></span>

                  {#if editingStat === editKey}
                    <div class="stat-edit">
                      <input
                        type="number"
                        class="stat-input"
                        bind:value={editValue}
                        onkeydown={(e) => {
                          if (e.key === "Enter") saveEdit(cat.id, stat.id);
                          if (e.key === "Escape") cancelEdit();
                        }}
                      />
                      <button
                        class="save-btn"
                        onclick={() => saveEdit(cat.id, stat.id)}>✓</button
                      >
                      <button class="cancel-btn" onclick={cancelEdit}>✕</button>
                    </div>
                  {:else}
                    <span
                      class="stat-value"
                      role="button"
                      tabindex="0"
                      ondblclick={() => startEdit(cat.id, stat.id, stat.value)}
                    >
                      {formatNumber(stat.value)}
                    </span>
                    <button
                      class="edit-btn"
                      onclick={() => startEdit(cat.id, stat.id, stat.value)}
                      title="Edit value">✏️</button
                    >
                  {/if}
                </div>
              {/each}

              {#if filteredStats.length === 0}
                <p class="no-stats">No matching stats</p>
              {/if}
            </div>
          {/if}
        </div>
      {/if}
    {/each}

    {#if categories.length === 0}
      <p class="no-stats">No statistics available</p>
    {/if}
  </div>
</div>

<style>
  .statistics-panel {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .panel-header {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .panel-title {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
    color: var(--color-text-1);
  }

  .header-controls {
    display: flex;
    gap: 8px;
  }

  .search-input {
    flex: 1;
    padding: 8px 12px;
    font-size: 0.85rem;
    background: var(--color-bg-2);
    border: 1px solid var(--color-bg-4);
    border-radius: var(--radius-sm);
    color: var(--color-text-1);
  }

  .search-input:focus {
    outline: none;
    border-color: var(--color-primary);
  }

  .sort-select {
    padding: 8px 12px;
    font-size: 0.8rem;
    background: var(--color-bg-2);
    border: 1px solid var(--color-bg-4);
    border-radius: var(--radius-sm);
    color: var(--color-text-1);
    cursor: pointer;
  }

  /* Categories */
  .categories-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .category {
    background: var(--color-bg-2);
    border: 1px solid var(--color-bg-3);
    border-radius: var(--radius-md);
    overflow: hidden;
  }

  .category-header {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 10px 14px;
    background: transparent;
    border: none;
    cursor: pointer;
    text-align: left;
    transition: background 0.15s ease;
  }

  .category-header:hover {
    background: var(--color-bg-3);
  }

  .expand-icon {
    font-size: 0.7rem;
    color: var(--color-text-3);
    width: 12px;
  }

  .category-icon {
    font-size: 1rem;
  }

  .category-name {
    font-weight: 600;
    font-size: 0.9rem;
    color: var(--color-text-1);
  }

  .category-count {
    margin-left: auto;
    font-size: 0.75rem;
    color: var(--color-text-3);
  }

  /* Stats List */
  .stats-list {
    display: flex;
    flex-direction: column;
    padding: 4px 14px 10px;
    max-height: 400px;
    overflow-y: auto;
  }

  .stat-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 0;
    border-bottom: 1px solid var(--color-bg-3);
    font-size: 0.8rem;
  }

  .stat-row:last-child {
    border-bottom: none;
  }

  .stat-row.modified {
    background: rgba(250, 204, 21, 0.1);
    margin: 0 -14px;
    padding-left: 14px;
    padding-right: 14px;
    border-left: 3px solid var(--modified-color);
  }

  .stat-name {
    flex-shrink: 0;
    color: var(--color-text-2);
  }

  .stat-dots {
    flex: 1;
    border-bottom: 1px dotted var(--color-bg-4);
    min-width: 20px;
    margin: 0 8px;
    align-self: flex-end;
    margin-bottom: 4px;
  }

  .stat-value {
    font-family: "SF Mono", monospace;
    font-weight: 600;
    color: var(--color-text-1);
    cursor: pointer;
    padding: 2px 6px;
    border-radius: 4px;
    transition: background 0.15s ease;
  }

  .stat-value:hover {
    background: var(--color-bg-3);
  }

  .edit-btn {
    padding: 2px 4px;
    font-size: 0.75rem;
    background: transparent;
    border: none;
    cursor: pointer;
    opacity: 0;
    transition: opacity 0.15s ease;
  }

  .stat-row:hover .edit-btn {
    opacity: 0.5;
  }

  .stat-row:hover .edit-btn:hover {
    opacity: 1;
  }

  /* Stat Editing */
  .stat-edit {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .stat-input {
    width: 100px;
    padding: 4px 8px;
    font-family: "SF Mono", monospace;
    font-size: 0.8rem;
    background: var(--color-bg-1);
    border: 1px solid var(--color-primary);
    border-radius: 4px;
    color: var(--color-text-1);
    text-align: right;
  }

  .stat-input:focus {
    outline: none;
  }

  .save-btn,
  .cancel-btn {
    width: 24px;
    height: 24px;
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 0.8rem;
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .save-btn {
    background: var(--color-green);
    border: none;
    color: white;
  }

  .cancel-btn {
    background: transparent;
    border: 1px solid var(--color-bg-4);
    color: var(--color-text-2);
  }

  .save-btn:hover {
    filter: brightness(1.1);
  }

  .cancel-btn:hover {
    background: var(--color-bg-3);
  }

  .no-stats {
    text-align: center;
    color: var(--color-text-3);
    font-size: 0.85rem;
    padding: 16px;
  }
</style>
