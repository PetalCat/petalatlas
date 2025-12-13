<script lang="ts">
  import { page } from "$app/stores";
  import { goto } from "$app/navigation";
  import { invoke } from "@tauri-apps/api/core";
  import type { WorldInfo } from "$lib/types";
  import { onMount } from "svelte";

  let world = $state<WorldInfo | null>(null);
  let worlds = $state<WorldInfo[]>([]);
  let loading = $state(true);
  let error = $state("");
  let saving = $state(false);
  let hasChanges = $state(false);

  // NBT Data
  let nbtData = $state<any>(null);
  let expandedNodes = $state<Set<string>>(new Set());

  // Load world and NBT data
  async function loadWorldData() {
    loading = true;
    error = "";
    world = null;
    nbtData = null;
    hasChanges = false;
    expandedNodes = new Set();

    try {
      const cached = await invoke<WorldInfo[]>("load_worlds_cache");
      worlds = cached;

      const id = $page.params.id;
      world = cached.find((w) => w.id === id) || null;

      if (!world) {
        error = "World not found";
      } else if (world.is_zip) {
        error = "Cannot edit zipped worlds";
      } else {
        // Load NBT data
        const result = await invoke<{ data: any }>("read_level_dat", {
          path: world.path,
        });
        nbtData = result.data;

        // Auto-expand Data node
        expandedNodes.add("root.Data");
      }
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    loadWorldData();
  });

  // Reload when route changes
  $effect(() => {
    if ($page.params.id) {
      loadWorldData();
    }
  });

  function toggleNode(path: string) {
    if (expandedNodes.has(path)) {
      expandedNodes.delete(path);
    } else {
      expandedNodes.add(path);
    }
    expandedNodes = new Set(expandedNodes);
  }

  function isExpanded(path: string): boolean {
    return expandedNodes.has(path);
  }

  function getNodeType(value: any): string {
    if (value === null) return "null";
    if (Array.isArray(value)) return "array";
    if (typeof value === "object") return "compound";
    if (typeof value === "number") {
      return Number.isInteger(value) ? "int" : "float";
    }
    return typeof value;
  }

  function getNodeIcon(value: any): string {
    const type = getNodeType(value);
    switch (type) {
      case "compound":
        return "📦";
      case "array":
        return "📋";
      case "string":
        return "📝";
      case "int":
      case "float":
        return "🔢";
      case "boolean":
        return "✓";
      default:
        return "•";
    }
  }

  function updateValue(path: string[], newValue: any) {
    if (!nbtData || path.length === 0) return;

    // Create a deep clone to trigger reactivity
    const clonedData = JSON.parse(JSON.stringify(nbtData));

    // Navigate to parent
    let current = clonedData;
    for (let i = 1; i < path.length - 1; i++) {
      // Start at 1 to skip "root"
      if (!current[path[i]]) {
        console.error("Path not found:", path, "at index", i);
        return;
      }
      current = current[path[i]];
    }

    // Update the value
    const lastKey = path[path.length - 1];
    current[lastKey] = newValue;

    nbtData = clonedData;
    hasChanges = true;
  }

  async function saveChanges() {
    if (!world) return;

    saving = true;
    try {
      await invoke("write_level_dat", {
        path: world.path,
        data: nbtData,
      });

      hasChanges = false;

      // Update this specific world in the cache
      try {
        const updatedWorld = await invoke<WorldInfo>("update_world_in_cache", {
          worldPath: world.path,
        });

        // Update local references
        world = updatedWorld;

        // Update the worlds array
        const index = worlds.findIndex((w) => w.id === world.id);
        if (index !== -1) {
          worlds[index] = updatedWorld;
          worlds = [...worlds]; // Trigger reactivity
        }
      } catch (e) {
        console.error("Failed to update cache:", e);
      }

      alert(
        "Changes saved successfully! A backup was created at level.dat.backup"
      );
    } catch (e) {
      error = String(e);
      alert("Failed to save: " + error);
    } finally {
      saving = false;
    }
  }

  function goBack() {
    if (hasChanges) {
      if (
        confirm("You have unsaved changes. Are you sure you want to leave?")
      ) {
        goto(`/world/${world?.id}`);
      }
    } else {
      goto(`/world/${world?.id}`);
    }
  }
</script>

{#if loading}
  <div class="loading">
    <div class="spinner"></div>
    <p>Loading NBT data...</p>
  </div>
{:else if error || !world}
  <div class="error-page">
    <h1>⚠️ Error</h1>
    <p>{error || "World not found"}</p>
    <button onclick={goBack}>Back to Overview</button>
  </div>
{:else}
  <div class="nbt-editor">
    <!-- Save Bar -->
    {#if hasChanges}
      <div class="save-bar">
        <span>⚠️ You have unsaved changes</span>
        <div class="save-actions">
          <button class="btn-cancel" onclick={goBack}>Discard</button>
          <button class="btn-save" onclick={saveChanges} disabled={saving}>
            {saving ? "Saving..." : "💾 Save Changes"}
          </button>
        </div>
      </div>
    {/if}

    <!-- Editor Content -->
    <div class="editor-content">
      <div class="editor-header">
        <h2>📝 NBT Structure</h2>
        <p class="editor-hint">
          Expand nodes to view and edit values. Changes are highlighted in
          yellow.
        </p>
      </div>

      {#if nbtData}
        <div class="nbt-tree">
          {#snippet renderNode(
            key: string,
            value: any,
            path: string[],
            depth: number
          )}
            {@const nodePath = path.join(".")}
            {@const type = getNodeType(value)}
            {@const isObject = type === "compound" || type === "array"}
            {@const expanded = isExpanded(nodePath)}

            <div class="tree-node" style="padding-left: {depth * 1.5}rem">
              <div class="node-header">
                {#if isObject}
                  <button
                    class="expand-btn"
                    onclick={() => toggleNode(nodePath)}
                  >
                    {expanded ? "▼" : "▶"}
                  </button>
                {:else}
                  <span class="expand-spacer"></span>
                {/if}

                <span class="node-icon">{getNodeIcon(value)}</span>
                <span class="node-key">{key}:</span>
                <span class="node-type">{type}</span>

                {#if !isObject}
                  <input
                    class="node-value"
                    type={type === "int" || type === "float"
                      ? "number"
                      : "text"}
                    {value}
                    oninput={(e) => {
                      const target = e.target as HTMLInputElement;
                      const newValue =
                        type === "int" || type === "float"
                          ? Number(target.value)
                          : target.value;
                      updateValue(path, newValue);
                    }}
                  />
                {:else}
                  <span class="node-count">
                    {type === "array"
                      ? `[${value.length} items]`
                      : `{${Object.keys(value).length} fields}`}
                  </span>
                {/if}
              </div>

              {#if expanded && isObject}
                <div class="node-children">
                  {#if Array.isArray(value)}
                    {#each value as item, index}
                      {@render renderNode(
                        String(index),
                        item,
                        [...path, String(index)],
                        depth + 1
                      )}
                    {/each}
                  {:else}
                    {#each Object.entries(value) as [childKey, childValue]}
                      {@render renderNode(
                        childKey,
                        childValue,
                        [...path, childKey],
                        depth + 1
                      )}
                    {/each}
                  {/if}
                </div>
              {/if}
            </div>
          {/snippet}

          {#each Object.entries(nbtData) as [key, value]}
            {@render renderNode(key, value, ["root", key], 0)}
          {/each}
        </div>
      {/if}

      <!-- Info Box -->
      <div class="info-box">
        <h3>💡 Tips</h3>
        <ul>
          <li>Click arrows to expand/collapse nodes</li>
          <li>Edit values directly in the tree</li>
          <li>A backup is automatically created when you save</li>
          <li>Changes are stored in <code>level.dat</code></li>
        </ul>
      </div>
    </div>
  </div>
{/if}

<style>
  .loading,
  .error-page {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: calc(100vh - 200px);
    text-align: center;
    padding: 2rem;
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

  .nbt-editor {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .save-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1rem 2rem;
    background: var(--color-orange);
    border-bottom: 2px solid var(--color-red);
    font-weight: 600;
    color: white;
  }

  .save-actions {
    display: flex;
    gap: 1rem;
  }

  .btn-cancel,
  .btn-save {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: var(--radius-md);
    cursor: pointer;
    font-weight: 600;
    transition: all 0.2s;
  }

  .btn-cancel {
    background: rgba(255, 255, 255, 0.2);
    color: white;
  }

  .btn-cancel:hover {
    background: rgba(255, 255, 255, 0.3);
  }

  .btn-save {
    background: white;
    color: var(--color-orange);
  }

  .btn-save:hover:not(:disabled) {
    background: var(--color-bg-1);
  }

  .btn-save:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .editor-content {
    flex: 1;
    overflow-y: auto;
    padding: 2rem;
    max-width: 1200px;
    margin: 0 auto;
    width: 100%;
  }

  .editor-header {
    margin-bottom: 2rem;
  }

  .editor-header h2 {
    margin: 0 0 0.5rem 0;
    font-size: 1.5rem;
  }

  .editor-hint {
    margin: 0;
    color: var(--color-text-3);
    font-size: 0.875rem;
  }

  .nbt-tree {
    background: var(--color-bg-2);
    border-radius: var(--radius-lg);
    padding: 1rem;
    font-family: "SF Mono", "Monaco", "Menlo", "Courier New", monospace;
    font-size: 0.875rem;
    margin-bottom: 2rem;
  }

  .tree-node {
    position: relative;
  }

  .node-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.375rem 0.5rem;
    border-radius: var(--radius-sm);
    transition: background 0.15s;
  }

  .node-header:hover {
    background: var(--color-bg-3);
  }

  .expand-btn {
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
    width: 1.25rem;
    height: 1.25rem;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--color-text-3);
    font-size: 0.75rem;
    transition: transform 0.2s;
  }

  .expand-spacer {
    width: 1.25rem;
  }

  .node-icon {
    font-size: 1rem;
  }

  .node-key {
    font-weight: 600;
    color: var(--color-primary);
  }

  .node-type {
    font-size: 0.75rem;
    color: var(--color-text-3);
    background: var(--color-bg-3);
    padding: 0.125rem 0.375rem;
    border-radius: var(--radius-sm);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .node-value {
    flex: 1;
    background: var(--color-bg-1);
    border: 1px solid var(--color-bg-3);
    padding: 0.25rem 0.5rem;
    border-radius: var(--radius-sm);
    color: var(--color-text-1);
    font-family: inherit;
    font-size: inherit;
    transition: all 0.2s;
  }

  .node-value:focus {
    outline: none;
    border-color: var(--color-primary);
    background: var(--color-primary-light);
  }

  .node-count {
    color: var(--color-text-3);
    font-size: 0.75rem;
  }

  .node-children {
    border-left: 1px solid var(--color-bg-3);
    margin-left: 0.625rem;
  }

  .info-box {
    background: var(--color-primary-light);
    border-left: 4px solid var(--color-primary);
    border-radius: var(--radius-md);
    padding: 1.5rem;
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

  .info-box code {
    background: var(--color-bg-3);
    padding: 0.125rem 0.375rem;
    border-radius: var(--radius-sm);
    font-family: "SF Mono", monospace;
  }
</style>
