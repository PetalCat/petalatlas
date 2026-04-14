<script lang="ts">
  import type { TypedNbt } from "$lib/types";

  // props
  let {
    data = $bindable(null),
    originalData = null,
    showDiff = true,
    onsave = () => {},
    onchange = () => {},
  } = $props<{
    data: TypedNbt | null;
    originalData?: TypedNbt | null;
    showDiff?: boolean;
    onsave?: () => void;
    onchange?: () => void;
  }>();

  let expandedNodes = $state<Set<string>>(new Set());

  // Adding State
  let addingToPath = $state<string | null>(null);
  let newTagKey = $state("");
  let newTagType = $state("string");
  let newTagValue = $state<string>("");

  // Search State
  let searchQuery = $state("");
  let replaceValue = $state("");
  let searchResults = $state<string[]>([]); // Array of node paths that match
  let currentSearchIndex = $state(0);
  let showSearchBar = $state(false);
  let useRegex = $state(false);
  let searchInKeys = $state(true);
  let searchInValues = $state(true);

  // Multiselect State
  let selectedNodes = $state<Set<string>>(new Set());
  let lastSelectedPath = $state<string | null>(null);
  let allNodePaths = $state<string[]>([]); // Flat list of all paths for shift+click range

  function isSelected(pathString: string): boolean {
    return selectedNodes.has(pathString);
  }

  function selectNode(pathString: string, event: MouseEvent) {
    if (event.ctrlKey || event.metaKey) {
      // Toggle selection
      if (selectedNodes.has(pathString)) {
        selectedNodes.delete(pathString);
      } else {
        selectedNodes.add(pathString);
      }
      selectedNodes = new Set(selectedNodes);
      lastSelectedPath = pathString;
    } else if (event.shiftKey && lastSelectedPath) {
      // Range selection
      const startIdx = allNodePaths.indexOf(lastSelectedPath);
      const endIdx = allNodePaths.indexOf(pathString);
      if (startIdx !== -1 && endIdx !== -1) {
        const [from, to] =
          startIdx < endIdx ? [startIdx, endIdx] : [endIdx, startIdx];
        for (let i = from; i <= to; i++) {
          selectedNodes.add(allNodePaths[i]);
        }
        selectedNodes = new Set(selectedNodes);
      }
    } else {
      // Single selection (clear others)
      selectedNodes = new Set([pathString]);
      lastSelectedPath = pathString;
    }
  }

  function clearSelection() {
    selectedNodes = new Set();
    lastSelectedPath = null;
  }

  function deleteSelectedNodes() {
    if (selectedNodes.size === 0) return;

    pushHistory();

    const clonedData = JSON.parse(JSON.stringify(data));

    // Sort paths by depth (deepest first) to avoid parent deletion issues
    const sortedPaths = Array.from(selectedNodes).sort((a, b) => {
      return b.split(".").length - a.split(".").length;
    });

    for (const pathStr of sortedPaths) {
      const path = pathStr.split(".");
      let current: TypedNbt = clonedData;

      // Navigate to parent
      for (let i = 1; i < path.length - 1; i++) {
        const key = path[i];
        if (current.type === "compound") current = current.value[key];
        else if (current.type === "list") current = current.value[Number(key)];
      }

      const lastKey = path[path.length - 1];
      if (current.type === "compound") {
        delete current.value[lastKey];
      } else if (current.type === "list") {
        current.value.splice(Number(lastKey), 1);
      }
    }

    data = clonedData;
    clearSelection();
    onchange();
  }

  // Undo/Redo History
  let historyStack = $state<string[]>([]);
  let historyIndex = $state(-1);
  const MAX_HISTORY = 50;

  function pushHistory() {
    if (!data) return;
    const snapshot = JSON.stringify(data);
    // If we're not at the end of history, truncate forward history
    if (historyIndex < historyStack.length - 1) {
      historyStack = historyStack.slice(0, historyIndex + 1);
    }
    historyStack.push(snapshot);
    if (historyStack.length > MAX_HISTORY) {
      historyStack = historyStack.slice(-MAX_HISTORY);
    }
    historyIndex = historyStack.length - 1;
  }

  function undo() {
    if (historyIndex <= 0) return;
    historyIndex--;
    data = JSON.parse(historyStack[historyIndex]);
    onchange();
  }

  function redo() {
    if (historyIndex >= historyStack.length - 1) return;
    historyIndex++;
    data = JSON.parse(historyStack[historyIndex]);
    onchange();
  }

  // Initialize history when data first loads
  $effect(() => {
    if (data && historyStack.length === 0) {
      pushHistory();
    }
  });

  const TAG_TYPES = [
    "string",
    "byte",
    "short",
    "int",
    "long",
    "float",
    "double",
    "compound",
    "list",
    "byte_array",
    "int_array",
    "long_array",
  ];

  const RANGES = {
    byte: { min: -128, max: 127 },
    short: { min: -32768, max: 32767 },
    int: { min: -2147483648, max: 2147483647 },
  };

  // Helper function to auto-expand root if needed
  $effect(() => {
    if (data && data.type === "compound" && expandedNodes.size === 0) {
      // Only if fresh
      for (const k of Object.keys(data.value)) {
        expandedNodes.add(`root.${k}`);
      }
      expandedNodes = new Set(expandedNodes); // reactivity
    }
  });

  function toggleNode(path: string) {
    if (expandedNodes.has(path)) expandedNodes.delete(path);
    else expandedNodes.add(path);
    expandedNodes = new Set(expandedNodes);
  }

  function isExpanded(path: string): boolean {
    return expandedNodes.has(path);
  }

  function getNodeType(node: TypedNbt): string {
    return node?.type || "unknown";
  }

  function getNodeIcon(node: TypedNbt): string {
    const type = getNodeType(node);
    switch (type) {
      case "compound":
        return "📦";
      case "list":
        return "📋";
      case "string":
        return "📝";
      case "int":
      case "float":
      case "byte":
      case "short":
      case "long":
      case "double":
        return "🔢";
      case "byte_array":
      case "int_array":
      case "long_array":
        return "🔢[]";
      default:
        return "•";
    }
  }

  function validateInput(
    value: string | number,
    type: string
  ): { valid: boolean; msg?: string } {
    if (type === "string" || type === "compound" || type === "list")
      return { valid: true };

    if (type === "long") {
      if (!/^-?\d+$/.test(String(value)))
        return { valid: false, msg: "Must be a valid integer" };
      return { valid: true };
    }

    const num = Number(value);
    if (isNaN(num)) return { valid: false, msg: "Not a number" };

    if (["byte", "short", "int"].includes(type)) {
      if (!Number.isInteger(num))
        return { valid: false, msg: "Must be an integer" };
      const range = RANGES[type as keyof typeof RANGES];
      if (range && (num < range.min || num > range.max)) {
        return { valid: false, msg: `Range: ${range.min} to ${range.max}` };
      }
    }
    return { valid: true };
  }

  function updateValue(path: string[], newValue: any, type: string) {
    if (!validateInput(newValue, type).valid) return;
    if (!data) return;

    pushHistory(); // Save state before mutation

    const clonedData = JSON.parse(JSON.stringify(data));
    let current: TypedNbt = clonedData;

    for (let i = 1; i < path.length - 1; i++) {
      const key = path[i];
      if (current.type === "compound") {
        current = current.value[key];
      } else if (current.type === "list") {
        current = current.value[Number(key)];
      }
    }

    const lastKey = path[path.length - 1];
    let targetNode: TypedNbt;

    if (current.type === "compound") {
      targetNode = current.value[lastKey];
      if (type === "long") targetNode.value = String(newValue);
      else if (["byte", "short", "int", "float", "double"].includes(type))
        targetNode.value = Number(newValue);
      else targetNode.value = newValue;
      current.value[lastKey] = targetNode;
    } else if (current.type === "list") {
      targetNode = current.value[Number(lastKey)];
      if (type === "long") targetNode.value = String(newValue);
      else if (["byte", "short", "int", "float", "double"].includes(type))
        targetNode.value = Number(newValue);
      else targetNode.value = newValue;
      current.value[Number(lastKey)] = targetNode;
    }

    data = clonedData;
    onchange();
  }

  function deleteNode(path: string[]) {
    // No confirm/alert as per previous fix
    if (!data) return;

    pushHistory(); // Save state before mutation
    try {
      const clonedData = JSON.parse(JSON.stringify(data));
      let current: TypedNbt = clonedData;

      for (let i = 1; i < path.length - 1; i++) {
        const key = path[i];
        if (current.type === "compound") current = current.value[key];
        else if (current.type === "list") current = current.value[Number(key)];
      }

      const lastKey = path[path.length - 1];
      if (current.type === "compound") delete current.value[lastKey];
      else if (current.type === "list")
        current.value.splice(Number(lastKey), 1);

      data = clonedData;
      onchange();
    } catch (e) {
      console.error("Msg", e);
    }
  }

  function startAdding(pathString: string) {
    addingToPath = pathString;
    newTagKey = "";
    newTagType = "string";
    newTagValue = "";
  }

  function cancelAdding() {
    addingToPath = null;
  }

  function confirmAddTag(targetPath: string[]) {
    if (!data) return;

    pushHistory(); // Save state before mutation

    const clonedData = JSON.parse(JSON.stringify(data));
    let current: TypedNbt = clonedData;

    for (let i = 1; i < targetPath.length; i++) {
      const key = targetPath[i];
      if (current.type === "compound") current = current.value[key];
      else if (current.type === "list") current = current.value[Number(key)];
    }

    let defaultValue: any;
    // Use provided value or generate default
    if (["byte", "short", "int", "float", "double"].includes(newTagType)) {
      defaultValue = newTagValue ? Number(newTagValue) : 0;
    } else if (newTagType === "long") {
      defaultValue = newTagValue || "0";
    } else if (newTagType === "compound") {
      defaultValue = {};
    } else if (newTagType === "list") {
      defaultValue = [];
    } else if (["byte_array", "int_array", "long_array"].includes(newTagType)) {
      defaultValue = [];
    } else {
      // string
      defaultValue = newTagValue || "";
    }

    const newNode: TypedNbt = { type: newTagType as any, value: defaultValue };

    if (current.type === "list") {
      current.value.push(newNode);
    } else if (current.type === "compound") {
      if (!newTagKey) return;
      if (current.value[newTagKey] !== undefined) return;
      current.value[newTagKey] = newNode;
    }

    data = clonedData;
    onchange();
    expandedNodes.add(targetPath.join("."));
    expandedNodes = new Set(expandedNodes);
    addingToPath = null;
    newTagValue = "";
  }

  function getDiffState(
    current: TypedNbt | undefined,
    original: TypedNbt | undefined
  ): "added" | "deleted" | "modified" | "unchanged" {
    if (!showDiff) return "unchanged";
    if (!original && current) return "added";
    if (original && !current) return "deleted";
    if (!original || !current) return "unchanged"; // should not happen

    if (current.type !== original.type) return "modified";
    if (current.value !== original.value) {
      // Deep compare for objects? No, traversal handles children.
      // For primitives, value check is enough.
      // For compound/list, we mark as modified only if type changed (handled above)
      // or let recursion handle children.
      if (
        ["compound", "list", "byte_array", "int_array", "long_array"].includes(
          current.type
        )
      ) {
        return "unchanged"; // container itself is unchanged, children differ
      }
      return "modified";
    }
    return "unchanged";
  }
  function handleKeydown(
    e: KeyboardEvent,
    path: string[],
    isObject: boolean,
    expanded: boolean
  ) {
    if (e.target !== e.currentTarget) return; // Ignore events from children (inputs)

    const nodePath = path.join(".");

    if (e.key === "Enter") {
      e.preventDefault();
      // Find input and focus it
      const input = (e.currentTarget as HTMLElement).querySelector(
        "input.node-value"
      );
      if (input instanceof HTMLElement) {
        input.focus();
      }
    } else if (e.key === " " || e.key === "Spacebar") {
      e.preventDefault();
      if (isObject) toggleNode(nodePath);
    } else if (e.key === "Delete" || e.key === "Backspace") {
      e.preventDefault();
      // Use bulk delete if multiple selected, otherwise single delete
      if (selectedNodes.size > 1) {
        deleteSelectedNodes();
      } else {
        deleteNode(path);
      }
    } else if (e.key === "ArrowRight") {
      if (isObject && !expanded) {
        e.preventDefault();
        toggleNode(nodePath);
      }
    } else if (e.key === "ArrowLeft") {
      if (isObject && expanded) {
        e.preventDefault();
        toggleNode(nodePath);
      }
    }
  }

  function handleGlobalKeydown(e: KeyboardEvent) {
    // Undo: Ctrl+Z (or Cmd+Z on Mac)
    if ((e.ctrlKey || e.metaKey) && e.key === "z" && !e.shiftKey) {
      e.preventDefault();
      undo();
    }
    // Redo: Ctrl+Shift+Z or Ctrl+Y
    if (
      (e.ctrlKey || e.metaKey) &&
      (e.key === "y" || (e.key === "z" && e.shiftKey))
    ) {
      e.preventDefault();
      redo();
    }
    // Search: Ctrl+F
    if ((e.ctrlKey || e.metaKey) && e.key === "f") {
      e.preventDefault();
      showSearchBar = !showSearchBar;
      if (!showSearchBar) {
        searchQuery = "";
        searchResults = [];
      }
    }
    // Escape: clear selection first, then close search
    if (e.key === "Escape") {
      if (selectedNodes.size > 0) {
        clearSelection();
      } else if (showSearchBar) {
        showSearchBar = false;
        searchQuery = "";
        searchResults = [];
      }
    }
  }

  // Search functions
  function performSearch() {
    if (!data || !searchQuery.trim()) {
      searchResults = [];
      return;
    }

    const results: string[] = [];

    function matchesQuery(text: string): boolean {
      if (!text) return false;
      const strText = String(text);
      if (useRegex) {
        try {
          const regex = new RegExp(searchQuery, "i");
          return regex.test(strText);
        } catch {
          return false;
        }
      }
      return strText.toLowerCase().includes(searchQuery.toLowerCase());
    }

    function traverse(node: TypedNbt, path: string[], key: string) {
      // Check key
      if (searchInKeys && matchesQuery(key)) {
        results.push(path.join("."));
      }
      // Check value (for primitives)
      if (searchInValues && !["compound", "list"].includes(node.type)) {
        if (matchesQuery(String(node.value))) {
          const fullPath = path.join(".");
          if (!results.includes(fullPath)) results.push(fullPath);
        }
      }
      // Recurse
      if (node.type === "compound") {
        for (const k of Object.keys(node.value)) {
          traverse(node.value[k], [...path, k], k);
        }
      } else if (node.type === "list") {
        for (let i = 0; i < node.value.length; i++) {
          traverse(node.value[i], [...path, String(i)], String(i));
        }
      }
    }

    // Start traversal
    if (data.type === "compound") {
      for (const k of Object.keys(data.value)) {
        traverse(data.value[k], ["root", k], k);
      }
    }

    searchResults = results;
    currentSearchIndex = 0;

    // Auto-expand to first result
    if (results.length > 0) {
      expandToPath(results[0]);
    }
  }

  function expandToPath(pathString: string) {
    const parts = pathString.split(".");
    let current = "";
    for (let i = 0; i < parts.length - 1; i++) {
      current = i === 0 ? parts[i] : current + "." + parts[i];
      expandedNodes.add(current);
    }
    expandedNodes = new Set(expandedNodes);
  }

  function navigateSearch(direction: 1 | -1) {
    if (searchResults.length === 0) return;
    currentSearchIndex =
      (currentSearchIndex + direction + searchResults.length) %
      searchResults.length;
    expandToPath(searchResults[currentSearchIndex]);
  }

  function replaceCurrentMatch() {
    if (searchResults.length === 0 || !replaceValue) return;
    const pathStr = searchResults[currentSearchIndex];
    const path = pathStr.split(".");

    pushHistory();

    // Navigate to the node and replace its value
    const clonedData = JSON.parse(JSON.stringify(data));
    let current: TypedNbt = clonedData;

    for (let i = 1; i < path.length - 1; i++) {
      const key = path[i];
      if (current.type === "compound") current = current.value[key];
      else if (current.type === "list") current = current.value[Number(key)];
    }

    const lastKey = path[path.length - 1];
    let targetNode: TypedNbt;
    if (current.type === "compound") {
      targetNode = current.value[lastKey];
    } else if (current.type === "list") {
      targetNode = current.value[Number(lastKey)];
    } else {
      return;
    }

    // Replace value based on type
    if (!["compound", "list"].includes(targetNode.type)) {
      if (useRegex) {
        try {
          const regex = new RegExp(searchQuery, "gi");
          targetNode.value = String(targetNode.value).replace(
            regex,
            replaceValue
          );
        } catch {
          targetNode.value = replaceValue;
        }
      } else {
        targetNode.value = String(targetNode.value).replace(
          new RegExp(searchQuery.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"), "gi"),
          replaceValue
        );
      }
    }

    data = clonedData;
    onchange();
    performSearch(); // Re-search to update results
  }

  function replaceAll() {
    if (searchResults.length === 0 || !replaceValue) return;
    pushHistory();

    // Replace all matches
    const clonedData = JSON.parse(JSON.stringify(data));

    for (const pathStr of searchResults) {
      const path = pathStr.split(".");
      let current: TypedNbt = clonedData;

      for (let i = 1; i < path.length - 1; i++) {
        const key = path[i];
        if (current.type === "compound") current = current.value[key];
        else if (current.type === "list") current = current.value[Number(key)];
      }

      const lastKey = path[path.length - 1];
      let targetNode: TypedNbt;
      if (current.type === "compound") {
        targetNode = current.value[lastKey];
      } else if (current.type === "list") {
        targetNode = current.value[Number(lastKey)];
      } else {
        continue;
      }

      if (!["compound", "list"].includes(targetNode.type)) {
        if (useRegex) {
          try {
            const regex = new RegExp(searchQuery, "gi");
            targetNode.value = String(targetNode.value).replace(
              regex,
              replaceValue
            );
          } catch {
            targetNode.value = replaceValue;
          }
        } else {
          targetNode.value = String(targetNode.value).replace(
            new RegExp(
              searchQuery.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"),
              "gi"
            ),
            replaceValue
          );
        }
      }
    }

    data = clonedData;
    onchange();
    performSearch();
  }

  function isSearchMatch(pathString: string): boolean {
    return searchResults.includes(pathString);
  }

  function isCurrentSearchMatch(pathString: string): boolean {
    return searchResults[currentSearchIndex] === pathString;
  }
</script>

<svelte:window onkeydown={handleGlobalKeydown} />

<div class="nbt-editor">
  <!-- Selection Overlay (floating, no layout shift) -->
  {#if selectedNodes.size > 0}
    <div class="selection-overlay">
      <span class="selection-count">{selectedNodes.size}</span>
      <button
        class="selection-btn delete"
        onclick={deleteSelectedNodes}
        title="Delete selected">🗑</button
      >
      <button
        class="selection-btn clear"
        onclick={clearSelection}
        title="Clear selection">✕</button
      >
    </div>
  {/if}

  <!-- Search Bar -->
  {#if showSearchBar}
    <div class="search-bar">
      <div class="search-row">
        <input
          class="search-input"
          placeholder="Search keys or values..."
          bind:value={searchQuery}
          oninput={() => performSearch()}
          onkeydown={(e) => {
            e.stopPropagation();
            if (e.key === "Enter") navigateSearch(1);
          }}
        />
        <input
          class="search-input replace-input"
          placeholder="Replace with..."
          bind:value={replaceValue}
          onkeydown={(e) => e.stopPropagation()}
        />
        <button
          class="search-btn"
          onclick={() => navigateSearch(-1)}
          title="Previous">◀</button
        >
        <button
          class="search-btn"
          onclick={() => navigateSearch(1)}
          title="Next">▶</button
        >
        <button
          class="search-btn replace-btn"
          onclick={replaceCurrentMatch}
          title="Replace">Replace</button
        >
        <button
          class="search-btn replace-btn"
          onclick={replaceAll}
          title="Replace All">All</button
        >
        <button
          class="search-btn close-btn"
          onclick={() => {
            showSearchBar = false;
            searchResults = [];
          }}>✕</button
        >
      </div>
      <div class="search-options">
        <label
          ><input
            type="checkbox"
            bind:checked={useRegex}
            onchange={() => performSearch()}
          /> Regex</label
        >
        <label
          ><input
            type="checkbox"
            bind:checked={searchInKeys}
            onchange={() => performSearch()}
          /> Keys</label
        >
        <label
          ><input
            type="checkbox"
            bind:checked={searchInValues}
            onchange={() => performSearch()}
          /> Values</label
        >
        <span class="search-count">
          {#if searchResults.length > 0}
            {currentSearchIndex + 1} / {searchResults.length}
          {:else if searchQuery}
            No results
          {/if}
        </span>
      </div>
    </div>
  {/if}

  {#if data && data.type === "compound"}
    <div class="nbt-tree" role="tree">
      {#snippet renderNode(
        key: string,
        value: TypedNbt | undefined,
        originalVal: TypedNbt | undefined,
        path: string[],
        depth: number
      )}
        {@const nodePath = path.join(".")}
        {@const diffState = getDiffState(value, originalVal)}
        {@const effectiveNode = value || originalVal}
        {@const type = effectiveNode?.type || "unknown"}
        {@const isObject = type === "compound" || type === "list"}
        {@const expanded = isExpanded(nodePath)}
        {@const isAdding = addingToPath === nodePath}

        <!-- Skip deleted nodes if we didn't implement full deleted view yet, 
             but plan says show them. -->

        <div
          class="tree-node {diffState}"
          class:search-match={isSearchMatch(nodePath)}
          class:current-match={isCurrentSearchMatch(nodePath)}
          class:selected={isSelected(nodePath)}
          style="padding-left: {depth * 1.5}rem"
          role="treeitem"
          aria-expanded={isObject ? expanded : undefined}
          aria-selected={isSelected(nodePath)}
        >
          <div
            class="node-header"
            tabindex="0"
            role="button"
            onclick={(e) => {
              // Selection handling with modifiers
              selectNode(nodePath, e);
              // Keep focus on header unless clicking interactive elements
              if (e.target === e.currentTarget)
                (e.currentTarget as HTMLElement).focus();
            }}
            onkeydown={(e) => handleKeydown(e, path, isObject, expanded)}
          >
            {#if isObject && diffState !== "deleted"}
              <button
                class="expand-btn"
                onclick={() => toggleNode(nodePath)}
                tabindex="-1">{expanded ? "▼" : "▶"}</button
              >
            {:else}
              <span class="expand-spacer"></span>
            {/if}

            <span class="node-icon"
              >{effectiveNode ? getNodeIcon(effectiveNode) : "?"}</span
            >
            <span class="node-key">{key}:</span>
            <span class="node-type">{type}</span>

            {#if !isObject && value}
              <!-- Edit Mode -->
              {@const validation = validateInput(value.value, type)}
              <div class="value-container">
                <input
                  class="node-value"
                  class:invalid={!validation.valid}
                  class:modified={diffState === "modified"}
                  type={[
                    "int",
                    "float",
                    "byte",
                    "short",
                    "long",
                    "double",
                  ].includes(type)
                    ? "number"
                    : "text"}
                  value={value.value}
                  readonly={diffState === "deleted"}
                  oninput={(e) =>
                    updateValue(
                      path,
                      (e.target as HTMLInputElement).value,
                      type
                    )}
                  onkeydown={(e) => e.stopPropagation()}
                  title={validation.msg}
                  disabled={type.endsWith("_array")}
                />
              </div>
            {:else if !isObject && !value}
              <!-- Deleted Primitive -->
              <span class="node-value deleted">{originalVal?.value}</span>
            {:else if isObject}
              <span class="node-count">
                {type === "list"
                  ? `[${(value || originalVal)?.value.length}]`
                  : `{${Object.keys((value || originalVal)?.value).length}}`}
              </span>
              {#if diffState !== "deleted"}
                <button
                  class="action-btn add"
                  onclick={() => startAdding(nodePath)}>+</button
                >
              {/if}
            {/if}

            {#if diffState !== "deleted"}
              <button class="action-btn delete" onclick={() => deleteNode(path)}
                >×</button
              >
            {:else}
              <button
                class="action-btn restore"
                onclick={() => {
                  /* Restore logic todo */
                }}>↺</button
              >
            {/if}
          </div>

          <!-- Adding Row -->
          {#if isAdding}
            <div
              class="adding-row"
              style="padding-left: {(depth + 1) * 1.5}rem"
            >
              <span class="node-icon">✨</span>
              {#if type === "compound"}
                <input
                  class="add-input add-name"
                  placeholder="Key"
                  bind:value={newTagKey}
                  onkeydown={(e) => {
                    e.stopPropagation();
                    if (e.key === "Enter") confirmAddTag(path);
                    if (e.key === "Escape") cancelAdding();
                  }}
                />
                <span class="sep">:</span>
              {/if}
              <select
                class="add-select"
                bind:value={newTagType}
                onkeydown={(e) => e.stopPropagation()}
              >
                {#each TAG_TYPES as t}<option value={t}>{t}</option>{/each}
              </select>
              {#if !["compound", "list", "byte_array", "int_array", "long_array"].includes(newTagType)}
                <span class="sep">=</span>
                <input
                  class="add-input add-value"
                  placeholder="Value"
                  type={[
                    "byte",
                    "short",
                    "int",
                    "float",
                    "double",
                    "long",
                  ].includes(newTagType)
                    ? "number"
                    : "text"}
                  bind:value={newTagValue}
                  onkeydown={(e) => {
                    e.stopPropagation();
                    if (e.key === "Enter") confirmAddTag(path);
                    if (e.key === "Escape") cancelAdding();
                  }}
                />
              {/if}
              <button
                class="icon-btn confirm"
                onclick={() => confirmAddTag(path)}>✓</button
              >
              <button class="icon-btn cancel" onclick={cancelAdding}>✕</button>
            </div>
          {/if}

          <!-- Children -->
          {#if (expanded || diffState === "added") && isObject && value}
            <div class="node-children" role="group">
              {#if type === "list"}
                <!-- List Diff: Simple index matching for now -->
                {@const len = Math.max(
                  value.value.length,
                  originalVal?.value?.length || 0
                )}
                {#each Array(len) as _, index}
                  {@render renderNode(
                    String(index),
                    value.value[index],
                    originalVal?.type === "list"
                      ? originalVal.value[index]
                      : undefined,
                    [...path, String(index)],
                    depth + 1
                  )}
                {/each}
              {:else}
                <!-- Compound Diff: Union of keys -->
                {@const currentKeys = Object.keys(value.value)}
                {@const originalKeys =
                  originalVal?.type === "compound"
                    ? Object.keys(originalVal.value)
                    : []}
                {@const allKeys = Array.from(
                  new Set([...currentKeys, ...originalKeys])
                ).sort()}

                {#each allKeys as k}
                  {@render renderNode(
                    k,
                    value.value[k],
                    originalVal?.type === "compound"
                      ? originalVal.value[k]
                      : undefined,
                    [...path, k],
                    depth + 1
                  )}
                {/each}
              {/if}
            </div>
          {/if}
        </div>
      {/snippet}

      {#if data.value}
        <!-- Root Diff -->
        {@const currentKeys = Object.keys(data.value)}
        {@const originalKeys =
          originalData?.type === "compound"
            ? Object.keys(originalData.value)
            : []}
        {@const allKeys = Array.from(
          new Set([...currentKeys, ...originalKeys])
        ).sort()}

        {#each allKeys as key}
          {@render renderNode(
            key,
            data.value[key],
            originalData?.type === "compound"
              ? originalData.value[key]
              : undefined,
            ["root", key],
            0
          )}
        {/each}
      {/if}
    </div>
  {:else if data}
    <div class="empty-state">
      Root must be a Compound Tag (Found {data.type})
    </div>
  {:else}
    <div class="empty-state">No data loaded</div>
  {/if}
</div>

<style>
  /* Tree Styles */
  .nbt-editor {
    position: relative;
    flex: 1;
    overflow-y: auto;
    font-family: "Menlo", "Monaco", "Courier New", monospace;
    font-size: 0.85rem;
    padding: 1rem;
    height: 100%;
    box-sizing: border-box;
  }

  /* Selection Overlay (floating, no layout shift) */
  .selection-overlay {
    position: absolute;
    top: 0.5rem;
    right: 0.5rem;
    background: rgba(59, 130, 246, 0.95);
    border-radius: 6px;
    padding: 4px 8px;
    display: flex;
    align-items: center;
    gap: 6px;
    z-index: 10;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
  }
  .selection-count {
    font-weight: 600;
    color: white;
    font-size: 0.8rem;
    min-width: 16px;
    text-align: center;
  }
  .selection-btn {
    width: 24px;
    height: 24px;
    padding: 0;
    border: none;
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.15);
    color: white;
    cursor: pointer;
    font-size: 0.85rem;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.15s;
  }
  .selection-btn:hover {
    background: rgba(255, 255, 255, 0.3);
  }
  .selection-btn.delete:hover {
    background: rgba(220, 38, 38, 0.8);
  }
  .selection-btn.clear:hover {
    background: rgba(255, 255, 255, 0.25);
  }

  /* Diff Styles */
  .tree-node.added .node-key {
    color: #16a34a;
  }
  .tree-node.added .node-header {
    background: rgba(22, 163, 74, 0.1);
  }

  .tree-node.deleted .node-key {
    color: #dc2626;
    text-decoration: line-through;
  }
  .tree-node.deleted .node-value {
    color: #dc2626;
    text-decoration: line-through;
  }
  .tree-node.deleted .node-header {
    background: rgba(220, 38, 38, 0.1);
    opacity: 0.7;
  }

  .node-value.modified {
    background: #fef9c3;
    border-color: #facc15;
  }

  .nbt-tree {
    display: flex;
    flex-direction: column;
  }

  .tree-node {
    margin: 1px 0;
  }

  .tree-node.selected > .node-header {
    background: rgba(59, 130, 246, 0.15);
    box-shadow: inset 0 0 0 1px rgba(59, 130, 246, 0.4);
  }
  .tree-node.selected > .node-header:hover {
    background: rgba(59, 130, 246, 0.2);
  }

  .node-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 2px 4px;
    border-radius: 4px;
    height: 24px;
  }
  .node-header:hover {
    background: var(--color-bg-3);
  }
  .node-header:focus {
    outline: none; /* Custom focus ring */
    background: var(--color-bg-3);
    box-shadow: inset 0 0 0 2px var(--color-primary);
  }
  .node-header:hover .action-btn,
  .node-header:focus .action-btn {
    opacity: 1;
  }

  .expand-btn {
    background: none;
    border: none;
    cursor: pointer;
    width: 16px;
    height: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--color-text-2);
    font-size: 0.7rem;
    padding: 0;
  }
  .expand-spacer {
    width: 16px;
  }

  .node-icon {
    font-size: 1rem;
    line-height: 1;
  }

  .node-key {
    color: var(--color-primary);
    font-weight: 600;
  }

  .node-type {
    font-size: 0.7rem;
    color: var(--color-text-3);
    /* background: var(--color-bg-3); */
    /* padding: 0 4px; */
    /* border-radius: 3px; */
    opacity: 0.7;
  }

  .value-container {
    flex: 1;
    display: flex;
  }

  .node-value {
    background: transparent;
    border: 1px solid transparent;
    padding: 0 4px;
    border-radius: 3px;
    color: var(--color-text-1);
    font-family: inherit;
    font-size: inherit;
    min-width: 60px;
    width: 100%;
    transition:
      border-color 0.1s,
      background-color 0.1s;
  }
  .node-value:hover,
  .node-value:focus {
    background: var(--color-bg-1);
    border-color: var(--color-bg-4);
  }
  .node-value:focus {
    outline: none;
    border-color: var(--color-primary);
  }

  .node-value.invalid {
    border-color: #ef4444;
    background: #fef2f2;
    color: #ef4444;
  }

  .node-count {
    color: var(--color-text-3);
    font-size: 0.8em;
  }

  .action-btn {
    opacity: 0;
    background: none;
    border: none;
    cursor: pointer;
    color: var(--color-text-3);
    width: 20px;
    height: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: opacity 0.1s;
    font-size: 1.1em;
  }
  .action-btn:hover {
    color: var(--color-text-1);
    background: rgba(0, 0, 0, 0.05);
    border-radius: 3px;
  }
  .action-btn.delete:hover {
    color: #ef4444;
    background: rgba(239, 68, 68, 0.1);
  }

  /* Adding UI */
  .adding-row {
    background: var(--color-primary-light);
    padding: 4px 8px;
    border: 1px dashed var(--color-primary);
    border-radius: 4px;
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 2px 0;
    height: 28px;
  }
  .add-input {
    padding: 2px 6px;
    border: 1px solid var(--color-bg-4);
    border-radius: 3px;
    font-size: 0.85rem;
    font-family: inherit;
    background: var(--color-bg-1);
    color: var(--color-text-1);
  }
  .add-input.add-name {
    width: 100px;
  }
  .add-input.add-value {
    flex: 1;
    min-width: 80px;
    max-width: 200px;
  }
  .add-select {
    padding: 2px 6px;
    border: 1px solid var(--color-bg-4);
    border-radius: 3px;
    font-size: 0.75rem;
    font-family: inherit;
    background: var(--color-bg-1);
    color: var(--color-text-1);
  }
  .add-input:focus,
  .add-select:focus {
    outline: none;
    border-color: var(--color-primary);
  }
  .sep {
    color: var(--color-text-3);
    font-size: 0.8em;
  }

  .icon-btn {
    width: 20px;
    height: 20px;
    border: none;
    border-radius: 3px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 0.9em;
  }
  .icon-btn.confirm {
    background: transparent;
    color: #16a34a;
  }
  .icon-btn.confirm:hover {
    background: #dcfce7;
  }
  .icon-btn.cancel {
    background: transparent;
    color: #ef4444;
  }
  .icon-btn.cancel:hover {
    background: #fee2e2;
  }

  .empty-state {
    padding: 2rem;
    text-align: center;
    color: var(--color-text-3);
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
  }

  /* Search Bar Styles */
  .search-bar {
    background: var(--color-bg-2);
    border-bottom: 1px solid var(--color-bg-3);
    padding: 0.5rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    flex-shrink: 0;
  }
  .search-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .search-input {
    padding: 4px 8px;
    border: 1px solid var(--color-bg-4);
    border-radius: 4px;
    font-size: 0.85rem;
    font-family: inherit;
    background: var(--color-bg-1);
    color: var(--color-text-1);
    flex: 1;
    min-width: 100px;
  }
  .search-input:focus {
    outline: none;
    border-color: var(--color-primary);
  }
  .replace-input {
    max-width: 150px;
  }
  .search-btn {
    padding: 4px 8px;
    border: 1px solid var(--color-bg-4);
    border-radius: 4px;
    background: var(--color-bg-3);
    color: var(--color-text-1);
    cursor: pointer;
    font-size: 0.8rem;
    transition: background 0.15s;
  }
  .search-btn:hover {
    background: var(--color-bg-4);
  }
  .replace-btn {
    background: var(--color-primary-light);
    border-color: var(--color-primary);
    color: var(--color-primary);
  }
  .replace-btn:hover {
    background: var(--color-primary);
    color: white;
  }
  .close-btn {
    color: var(--color-text-3);
  }
  .close-btn:hover {
    background: rgba(239, 68, 68, 0.1);
    color: #ef4444;
  }
  .search-options {
    display: flex;
    align-items: center;
    gap: 1rem;
    font-size: 0.75rem;
    color: var(--color-text-2);
  }
  .search-options label {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    cursor: pointer;
  }
  .search-options input[type="checkbox"] {
    accent-color: var(--color-primary);
  }
  .search-count {
    margin-left: auto;
    color: var(--color-text-3);
  }

  /* Search Match Highlighting */
  .tree-node.search-match .node-header {
    background: rgba(250, 204, 21, 0.2);
  }
  .tree-node.current-match .node-header {
    background: rgba(250, 204, 21, 0.4);
    box-shadow: inset 0 0 0 2px #facc15;
  }
</style>
