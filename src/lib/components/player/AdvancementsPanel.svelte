<script lang="ts">
  export interface Advancement {
    id: string;
    title: string;
    description: string;
    completed: boolean;
    criteria: { id: string; name: string; done: boolean }[];
    category?: string;
  }

  export interface AdvancementCategory {
    id: string;
    name: string;
    icon: string;
    advancements: Advancement[];
  }

  let {
    categories = [],
    searchQuery = $bindable(""),
    ongrant = () => {},
    onrevoke = () => {},
  } = $props<{
    categories: AdvancementCategory[];
    searchQuery?: string;
    ongrant?: (advancementId: string) => void;
    onrevoke?: (advancementId: string) => void;
  }>();

  let expandedCategories = $state<Set<string>>(new Set(["minecraft:story"]));
  let expandedAdvancements = $state<Set<string>>(new Set());

  function toggleCategory(catId: string) {
    if (expandedCategories.has(catId)) {
      expandedCategories.delete(catId);
    } else {
      expandedCategories.add(catId);
    }
    expandedCategories = new Set(expandedCategories);
  }

  function toggleAdvancement(advId: string) {
    if (expandedAdvancements.has(advId)) {
      expandedAdvancements.delete(advId);
    } else {
      expandedAdvancements.add(advId);
    }
    expandedAdvancements = new Set(expandedAdvancements);
  }

  function formatProgress(adv: Advancement): string {
    const done = adv.criteria.filter((c) => c.done).length;
    const total = adv.criteria.length;
    return `${done}/${total}`;
  }

  // Filter advancements
  function getFilteredAdvancements(advs: Advancement[]): Advancement[] {
    if (!searchQuery) return advs;
    const query = searchQuery.toLowerCase();
    return advs.filter(
      (a) =>
        a.title.toLowerCase().includes(query) ||
        a.description.toLowerCase().includes(query)
    );
  }

  function hasMatchingAdvancements(cat: AdvancementCategory): boolean {
    if (!searchQuery) return true;
    return getFilteredAdvancements(cat.advancements).length > 0;
  }

  function countCompleted(advs: Advancement[]): {
    done: number;
    total: number;
  } {
    const done = advs.filter((a) => a.completed).length;
    return { done, total: advs.length };
  }
</script>

<div class="advancements-panel">
  <div class="panel-header">
    <h4 class="panel-title">🏆 Advancements</h4>
    <input
      type="text"
      class="search-input"
      placeholder="Search advancements..."
      bind:value={searchQuery}
    />
  </div>

  <div class="categories-list">
    {#each categories as cat}
      {#if hasMatchingAdvancements(cat)}
        {@const isExpanded = expandedCategories.has(cat.id)}
        {@const filtered = getFilteredAdvancements(cat.advancements)}
        {@const progress = countCompleted(filtered)}

        <div class="category">
          <button
            class="category-header"
            onclick={() => toggleCategory(cat.id)}
          >
            <span class="expand-icon">{isExpanded ? "▼" : "▶"}</span>
            <span class="category-icon">{cat.icon}</span>
            <span class="category-name">{cat.name}</span>
            <span class="category-progress">
              {progress.done}/{progress.total}
              <span class="progress-bar">
                <span
                  class="progress-fill"
                  style="width: {(progress.done / progress.total) * 100}%"
                ></span>
              </span>
            </span>
          </button>

          {#if isExpanded}
            <div class="advancements-list">
              {#each filtered as adv}
                {@const isAdvExpanded = expandedAdvancements.has(adv.id)}

                <div class="advancement-row" class:completed={adv.completed}>
                  <button
                    class="advancement-header"
                    onclick={() => toggleAdvancement(adv.id)}
                  >
                    <span class="adv-status">{adv.completed ? "✓" : "○"}</span>
                    <div class="adv-info">
                      <span class="adv-title">{adv.title}</span>
                      <span class="adv-desc">{adv.description}</span>
                    </div>
                    {#if !adv.completed && adv.criteria.length > 1}
                      <span class="adv-progress">{formatProgress(adv)}</span>
                    {/if}
                    <div
                      class="adv-action"
                      onclick={(e) => e.stopPropagation()}
                    >
                      {#if adv.completed}
                        <button
                          class="action-btn revoke"
                          onclick={() => onrevoke(adv.id)}
                        >
                          Revoke
                        </button>
                      {:else}
                        <button
                          class="action-btn grant"
                          onclick={() => ongrant(adv.id)}
                        >
                          Grant
                        </button>
                      {/if}
                    </div>
                  </button>

                  {#if isAdvExpanded && adv.criteria.length > 1}
                    <div class="criteria-list">
                      {#each adv.criteria as criterion}
                        <div class="criterion" class:done={criterion.done}>
                          <span class="criterion-status"
                            >{criterion.done ? "✓" : "○"}</span
                          >
                          <span class="criterion-name">{criterion.name}</span>
                        </div>
                      {/each}
                    </div>
                  {/if}
                </div>
              {/each}

              {#if filtered.length === 0}
                <p class="no-results">No matching advancements</p>
              {/if}
            </div>
          {/if}
        </div>
      {/if}
    {/each}

    {#if categories.length === 0}
      <p class="no-results">No advancements loaded</p>
    {/if}
  </div>
</div>

<style>
  .advancements-panel {
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

  .search-input {
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

  /* Categories */
  .categories-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
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
    padding: 12px 14px;
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
    font-size: 1.1rem;
  }

  .category-name {
    font-weight: 600;
    font-size: 0.95rem;
    color: var(--color-text-1);
  }

  .category-progress {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.75rem;
    color: var(--color-text-3);
  }

  .progress-bar {
    width: 60px;
    height: 4px;
    background: var(--color-bg-4);
    border-radius: 2px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: var(--color-green);
    transition: width 0.3s ease;
  }

  /* Advancements List */
  .advancements-list {
    padding: 4px 8px 12px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .advancement-row {
    border-radius: var(--radius-sm);
    overflow: hidden;
    transition: all 0.15s ease;
  }

  .advancement-header {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 10px 12px;
    background: var(--color-bg-3);
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    cursor: pointer;
    text-align: left;
    transition: all 0.15s ease;
  }

  .advancement-header:hover {
    background: var(--color-bg-4);
  }

  .advancement-row.completed .advancement-header {
    border-left: 3px solid var(--color-green);
  }

  .adv-status {
    font-size: 0.9rem;
    width: 18px;
    text-align: center;
  }

  .advancement-row.completed .adv-status {
    color: var(--color-green);
  }

  .adv-info {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .adv-title {
    font-weight: 600;
    font-size: 0.85rem;
    color: var(--color-text-1);
  }

  .advancement-row.completed .adv-title {
    color: var(--color-green);
  }

  .adv-desc {
    font-size: 0.75rem;
    color: var(--color-text-3);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .adv-progress {
    font-size: 0.7rem;
    font-weight: 600;
    color: var(--color-text-3);
    padding: 2px 6px;
    background: var(--color-bg-2);
    border-radius: 4px;
  }

  .adv-action {
    flex-shrink: 0;
  }

  .action-btn {
    padding: 4px 10px;
    font-size: 0.7rem;
    font-weight: 600;
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .action-btn.grant {
    background: var(--color-green);
    border: none;
    color: white;
  }

  .action-btn.grant:hover {
    filter: brightness(1.1);
  }

  .action-btn.revoke {
    background: transparent;
    border: 1px solid var(--color-text-4);
    color: var(--color-text-2);
  }

  .action-btn.revoke:hover {
    background: var(--color-red);
    border-color: var(--color-red);
    color: white;
  }

  /* Criteria */
  .criteria-list {
    padding: 8px 12px 8px 40px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    background: var(--color-bg-2);
    border-top: 1px solid var(--color-bg-3);
  }

  .criterion {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.75rem;
    color: var(--color-text-3);
  }

  .criterion.done {
    color: var(--color-green);
  }

  .criterion-status {
    width: 14px;
    text-align: center;
  }

  .criterion-name {
    flex: 1;
  }

  .no-results {
    text-align: center;
    color: var(--color-text-3);
    font-size: 0.85rem;
    padding: 16px;
  }
</style>
