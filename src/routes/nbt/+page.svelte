<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { invoke } from "@tauri-apps/api/core";
  import NbtEditor from "$lib/components/NbtEditor.svelte";
  import type { TypedNbt } from "$lib/types";
  import { page } from "$app/stores";
  import { onMount } from "svelte";

  let currentPath = $state<string | null>(null);
  let currentName = $state<string>("");
  let nbtData = $state<TypedNbt | null>(null);
  let originalData = $state<TypedNbt | null>(null);
  let nbtFormat = $state<string>("java");
  let hasChanges = $state(false);
  let loading = $state(false);
  let error = $state("");

  async function openFile() {
    try {
      const selected = await open({
        multiple: false,
        title: "Open NBT File",
        filters: [
          {
            name: "NBT Files",
            extensions: ["dat", "nbt", "mcstructure", "schematic"],
          },
          {
            name: "All Files",
            extensions: ["*"],
          },
        ],
      });

      if (selected && typeof selected === "string") {
        await loadFile(selected);
      }
    } catch (e) {
      console.error(e);
      error = "Failed to open dialog: " + e;
    }
  }

  async function loadFile(path: string) {
    if (hasChanges && !confirm("Discard unsaved changes?")) return;

    loading = true;
    error = "";
    try {
      currentPath = path;
      // Extract filename
      currentName = path.split(/[\\/]/).pop() || path;

      const result = await invoke<{ data: TypedNbt; format: string }>(
        "read_nbt_file",
        { path }
      );
      nbtData = result.data;
      originalData = JSON.parse(JSON.stringify(result.data)); // Deep clone
      nbtFormat = result.format;
      hasChanges = false;
    } catch (e) {
      error = "Failed to read file: " + e;
      currentPath = null;
      nbtData = null;
      originalData = null;
    } finally {
      loading = false;
    }
  }

  async function saveFile() {
    if (!currentPath || !nbtData) return;
    try {
      await invoke("write_nbt_file", {
        path: currentPath,
        data: nbtData,
        format: nbtFormat,
      });
      hasChanges = false;
      if (nbtData) originalData = JSON.parse(JSON.stringify(nbtData)); // Update baseline
      alert("Saved!");
    } catch (e) {
      alert("Failed to save: " + e);
    }
  }

  onMount(() => {
    const pathParam = $page.url.searchParams.get("path");
    if (pathParam) {
      loadFile(pathParam);
    }
  });
</script>

<div class="page-container">
  <header class="toolbar">
    <div class="left">
      <a href="/" class="btn-back">← Home</a>
      <div class="file-info">
        {#if currentPath}
          <h2>{currentName}{hasChanges ? "*" : ""}</h2>
          <span class="badge">{nbtFormat}</span>
        {:else}
          <h2>NBT Editor</h2>
        {/if}
      </div>
    </div>

    <div class="actions">
      <button class="btn-primary" onclick={openFile}>Open File</button>
      {#if currentPath}
        <button class="btn-save" disabled={!hasChanges} onclick={saveFile}
          >Save</button
        >
      {/if}
    </div>
  </header>

  <main class="editor-area">
    {#if loading}
      <div class="center-msg">Loading...</div>
    {:else if error}
      <div class="center-msg error">{error}</div>
    {:else if nbtData}
      <NbtEditor
        bind:data={nbtData}
        {originalData}
        onchange={() => (hasChanges = true)}
      />
    {:else}
      <div class="empty-state">
        <p>No file open</p>
        <button class="btn-primary large" onclick={openFile}
          >Open NBT File</button
        >
      </div>
    {/if}
  </main>
</div>

<style>
  .page-container {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--color-bg-1);
    color: var(--color-text-1);
  }

  .toolbar {
    height: 60px;
    border-bottom: 1px solid var(--color-bg-3);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 1.5rem;
    background: var(--color-bg-1);
  }

  .left {
    display: flex;
    align-items: center;
    gap: 1.5rem;
  }

  .btn-back {
    text-decoration: none;
    color: var(--color-text-2);
    font-weight: 500;
  }
  .btn-back:hover {
    color: var(--color-text-1);
  }

  .file-info {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .badge {
    background: var(--color-bg-3);
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
    font-size: 0.75rem;
    text-transform: uppercase;
    font-weight: 600;
    color: var(--color-text-2);
  }

  .actions {
    display: flex;
    gap: 0.75rem;
  }

  .btn-primary {
    background: var(--color-primary);
    color: white;
    border: none;
    padding: 0.5rem 1rem;
    border-radius: 6px;
    cursor: pointer;
    font-weight: 500;
  }
  .btn-primary:hover {
    opacity: 0.9;
  }

  .btn-primary.large {
    font-size: 1.1rem;
    padding: 0.75rem 1.5rem;
  }

  .btn-save {
    background: var(--color-bg-3);
    color: var(--color-text-1);
    border: none;
    padding: 0.5rem 1rem;
    border-radius: 6px;
    cursor: pointer;
  }
  .btn-save:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .btn-save:hover:not(:disabled) {
    background: var(--color-bg-4);
  }

  .editor-area {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .center-msg {
    margin: auto;
    color: var(--color-text-3);
  }
  .error {
    color: #ef4444;
  }

  .empty-state {
    margin: auto;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1.5rem;
  }
</style>
