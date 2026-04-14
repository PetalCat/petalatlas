<script lang="ts">
  import { page } from "$app/stores";
  import { goto } from "$app/navigation";
  import { invoke } from "@tauri-apps/api/core";
  import type { WorldInfo, FileNode, TypedNbt } from "$lib/types";
  import NbtEditor from "$lib/components/NbtEditor.svelte";
  import { onMount } from "svelte";
  import { PaneGroup, Pane, PaneResizer } from "paneforge";

  let world = $state<WorldInfo | null>(null);
  let loading = $state(true);
  let error = $state("");
  let saving = $state(false);
  let hasChanges = $state(false);

  // File Browser State
  let rootFiles = $state<FileNode[]>([]);
  let currentFile = $state<FileNode | null>(null);
  let expandedDirs = $state<Set<string>>(
    new Set(["root", "playerdata", "data"])
  );

  // NBT Data
  let nbtData = $state<TypedNbt | null>(null);
  let originalData = $state<TypedNbt | null>(null);
  let nbtFormat = $state<string>("java");

  async function loadWorldAndFiles() {
    loading = true;
    error = "";
    try {
      const worlds = await invoke<WorldInfo[]>("load_worlds_cache");
      const id = $page.params.id;
      world = worlds.find((w) => w.id === id) || null;

      if (!world) {
        throw new Error("World not found");
      }

      // Load Files
      const files = await invoke<FileNode[]>("list_nbt_files", {
        worldPath: world.path,
      });
      rootFiles = files;

      // Select level.dat default
      const levelDat = findFile(files, "level.dat");
      if (levelDat) {
        await selectFile(levelDat);
      }
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  function findFile(nodes: FileNode[], name: string): FileNode | null {
    for (const node of nodes) {
      if (node.name === name) return node;
      if (node.children) {
        const found = findFile(node.children, name);
        if (found) return found;
      }
    }
    return null;
  }

  onMount(() => {
    loadWorldAndFiles();
  });

  function toggleDir(path: string) {
    if (expandedDirs.has(path)) {
      expandedDirs.delete(path);
    } else {
      expandedDirs.add(path);
    }
    expandedDirs = new Set(expandedDirs);
  }

  async function selectFile(file: FileNode) {
    if (hasChanges) {
      if (!confirm("You have unsaved changes. discard them?")) return;
    }

    currentFile = file;
    hasChanges = false;
    nbtData = null;
    originalData = null;

    try {
      const result = await invoke<{ data: TypedNbt; format: string }>(
        "read_nbt_file",
        { path: file.path }
      );
      nbtData = result.data;
      originalData = JSON.parse(JSON.stringify(result.data)); // Deep clone
      nbtFormat = result.format;
    } catch (e) {
      console.error(e);
      alert(`Failed to load ${file.name}: ${e}`);
      currentFile = null;
    }
  }

  // --- NBT Editor Logic ---

  async function saveChanges() {
    if (!world || !currentFile) return;
    saving = true;
    try {
      await invoke("write_nbt_file", {
        path: currentFile.path,
        data: nbtData,
        format: nbtFormat,
      });
      hasChanges = false;
      // Update original data on save
      if (nbtData) {
        originalData = JSON.parse(JSON.stringify(nbtData));
      }
      alert("Saved!");
    } catch (e) {
      alert("Failed to save: " + e);
    } finally {
      saving = false;
    }
  }

  function goBack() {
    if (hasChanges && !confirm("Unsaved changes. Leave?")) return;
    goto(`/world/${world?.id}`);
  }
</script>

<div class="page-container">
  {#if loading}
    <div class="loading">Loading...</div>
  {:else if error}
    <div class="error">{error}</div>
  {:else}
    <PaneGroup direction="horizontal">
      <Pane defaultSize={20} minSize={15} maxSize={40} class="sidebar-pane">
        <!-- Sidebar File Browser -->
        <aside class="file-sidebar">
          <div class="sidebar-header">
            <h3>Files</h3>
          </div>
          <div class="file-list">
            {#snippet renderFileNode(node: FileNode)}
              <div class="file-item" style="padding-left: 1rem">
                {#if node.is_dir}
                  <div
                    class="dir-label"
                    onclick={() => toggleDir(node.relative_path || node.name)}
                    role="button"
                    tabindex="0"
                    onkeydown={(e) =>
                      e.key === "Enter" &&
                      toggleDir(node.relative_path || node.name)}
                  >
                    <span class="icon"
                      >{expandedDirs.has(node.relative_path || node.name)
                        ? "📂"
                        : "📁"}</span
                    >
                    {node.name}
                  </div>
                  {#if expandedDirs.has(node.relative_path || node.name)}
                    <div class="dir-children">
                      {#each node.children as child}
                        {@render renderFileNode(child)}
                      {/each}
                    </div>
                  {/if}
                {:else}
                  <button
                    class="file-btn"
                    class:active={currentFile?.path === node.path}
                    class:disabled={!node.could_be_nbt}
                    disabled={!node.could_be_nbt}
                    onclick={() => selectFile(node)}
                  >
                    <span class="icon">📄</span>
                    {node.name}
                  </button>
                {/if}
              </div>
            {/snippet}

            {#each rootFiles as file}
              {@render renderFileNode(file)}
            {/each}
          </div>
        </aside>
      </Pane>

      <PaneResizer class="resizer" />

      <Pane class="main-pane">
        <!-- Main Editor Area -->
        <main class="editor-main">
          <header class="editor-toolbar">
            <div class="file-info">
              {#if currentFile}
                <h2>{currentFile.name}{hasChanges ? "*" : ""}</h2>
                <span class="badge">{nbtFormat}</span>
              {:else}
                <h2>Select a file</h2>
              {/if}
            </div>

            <div class="actions">
              <button class="btn-cancel" onclick={goBack}>Back</button>
              {#if currentFile}
                <button
                  class="btn-save"
                  disabled={!hasChanges || saving}
                  onclick={saveChanges}
                >
                  {saving ? "Saving..." : "Save"}
                </button>
              {/if}
            </div>
          </header>

          <div class="editor-body">
            {#if currentFile}
              <NbtEditor
                bind:data={nbtData}
                {originalData}
                onchange={() => (hasChanges = true)}
              />
            {:else}
              <div class="empty-state">Select a file to edit</div>
            {/if}
          </div>
        </main>
      </Pane>
    </PaneGroup>
  {/if}
</div>

<style>
  .page-container {
    display: flex;
    height: 100%;
    overflow: hidden;
    background: var(--color-bg-1);
  }

  .loading,
  .error {
    margin: auto;
  }

  .file-sidebar {
    width: 100%;
    height: 100%;
    background: var(--color-bg-2);
    /* border-right: 1px solid var(--color-bg-3); */
    display: flex;
    flex-direction: column;
    overflow-y: auto;
  }

  /* Resizer Styles */
  :global(.resizer) {
    background: var(--color-bg-3);
    width: 1px;
    position: relative;
    z-index: 10;
    cursor: col-resize;
    transition: background 0.2s;
    flex-shrink: 0;
  }

  :global(.resizer::after) {
    content: "";
    position: absolute;
    left: -4px;
    right: -4px;
    top: 0;
    bottom: 0;
    pointer-events: auto; /* Increase hit target */
  }

  :global(.resizer:hover),
  :global(.resizer[data-active]) {
    background: var(--color-primary);
    width: 2px; /* Visual feedback */
  }

  :global(.sidebar-pane) {
    min-width: 0; /* Important for flex/grid children in Panes */
    height: 100%;
    overflow: hidden;
  }

  :global(.main-pane) {
    min-width: 0;
    height: 100%;
    overflow: hidden;
  }

  .sidebar-header {
    padding: 1rem;
    border-bottom: 1px solid var(--color-bg-3);
    font-weight: 600;
    flex-shrink: 0;
  }

  .file-item {
    display: flex;
    flex-direction: column;
  }

  .dir-label {
    padding: 0.5rem 1rem;
    cursor: pointer;
    font-weight: 500;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .dir-label:hover {
    background: var(--color-bg-3);
  }

  .file-btn {
    background: none;
    border: none;
    padding: 0.5rem 1rem;
    text-align: left;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--color-text-1);
    width: 100%;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .file-btn:hover:not(:disabled) {
    background: var(--color-bg-3);
  }

  .file-btn.active {
    background: var(--color-primary-light);
    color: var(--color-primary);
    font-weight: 500;
  }

  .file-btn.disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .editor-main {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    height: 100%;
    overflow: hidden;
  }

  .editor-toolbar {
    height: 60px;
    border-bottom: 1px solid var(--color-bg-3);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 1.5rem;
    background: var(--color-bg-1);
    flex-shrink: 0;
  }

  .file-info {
    display: flex;
    align-items: center;
    gap: 1rem;
    min-width: 0;
  }

  .file-info h2 {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .badge {
    background: var(--color-bg-3);
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
    font-size: 0.75rem;
    text-transform: uppercase;
    font-weight: 600;
    color: var(--color-text-2);
    flex-shrink: 0;
  }

  .actions {
    display: flex;
    gap: 1rem;
    flex-shrink: 0;
  }

  .btn-save {
    padding: 0.5rem 1rem;
    background: var(--color-primary);
    color: white;
    border: none;
    border-radius: 6px;
    cursor: pointer;
  }

  .btn-save:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-cancel {
    padding: 0.5rem 1rem;
    background: var(--color-bg-3);
    border: none;
    border-radius: 6px;
    cursor: pointer;
  }

  .editor-body {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    position: relative;
  }

  .empty-state {
    margin: auto;
    color: var(--color-text-3);
  }
</style>
