<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { page } from "$app/stores";
  import { invoke } from "@tauri-apps/api/core";

  const TILE_SIZE = 512;
  const MIN_ZOOM = 0.05;
  const MAX_ZOOM = 8;

  type RenderMode = "block" | "texture";

  class Tile {
    x: number;
    z: number;
    image: ImageBitmap | null = null;
    loading = false;

    constructor(x: number, z: number) {
      this.x = x;
      this.z = z;
    }
  }

  let canvas: HTMLCanvasElement;
  let ctx: CanvasRenderingContext2D;

  let worldPath = "";
  let regionSet = new Set<string>();
  let tiles = new Map<string, Tile>();

  let camX = 0;
  let camZ = 0;
  let zoom = 0.5;

  let dragging = false;
  let lastX = 0;
  let lastY = 0;

  let renderMode: RenderMode = "block";

  /* ---------------------------- Lifecycle ---------------------------- */

  onMount(async () => {
    ctx = canvas.getContext("2d", { alpha: false })!;

    window.addEventListener("resize", resizeCanvas);
    window.addEventListener("mouseup", stopDrag);

    resizeCanvas();
    await loadWorld();

    canvas.addEventListener("wheel", onWheel, { passive: false });
    requestAnimationFrame(render);
  });

  onDestroy(() => {
    window.removeEventListener("resize", resizeCanvas);
    window.removeEventListener("mouseup", stopDrag);
  });

  /* ---------------------------- Setup ---------------------------- */

  async function loadWorld() {
    const id = $page.params.id;
    const worlds = await invoke<any[]>("load_worlds_cache");
    const world = worlds.find((w) => w.id === id);
    if (!world) return;

    worldPath = world.path;

    const regions = await invoke<{ x: number; z: number }[]>("get_regions", {
      worldPath,
    });

    let minX = Infinity,
      maxX = -Infinity;
    let minZ = Infinity,
      maxZ = -Infinity;

    for (const r of regions) {
      const key = `${r.x},${r.z}`;
      regionSet.add(key);

      minX = Math.min(minX, r.x);
      maxX = Math.max(maxX, r.x);
      minZ = Math.min(minZ, r.z);
      maxZ = Math.max(maxZ, r.z);
    }

    camX = ((minX + maxX + 1) * TILE_SIZE) / 2;
    camZ = ((minZ + maxZ + 1) * TILE_SIZE) / 2;
  }

  function resizeCanvas() {
    if (!canvas.parentElement) return;
    const r = canvas.parentElement.getBoundingClientRect();
    canvas.width = r.width;
    canvas.height = r.height;
  }

  /* ---------------------------- Input ---------------------------- */

  function startDrag(e: MouseEvent) {
    dragging = true;
    lastX = e.clientX;
    lastY = e.clientY;
  }

  function stopDrag() {
    dragging = false;
  }

  function onMouseMove(e: MouseEvent) {
    if (!dragging) return;

    camX -= (e.clientX - lastX) / zoom;
    camZ -= (e.clientY - lastY) / zoom;

    lastX = e.clientX;
    lastY = e.clientY;
  }

  function onWheel(e: WheelEvent) {
    e.preventDefault();

    const factor = e.deltaY > 0 ? 0.9 : 1.1;
    const rect = canvas.getBoundingClientRect();

    const mx = e.clientX - rect.left;
    const my = e.clientY - rect.top;

    const wx = camX + (mx - canvas.width / 2) / zoom;
    const wz = camZ + (my - canvas.height / 2) / zoom;

    zoom = Math.max(MIN_ZOOM, Math.min(zoom * factor, MAX_ZOOM));

    camX = wx - (mx - canvas.width / 2) / zoom;
    camZ = wz - (my - canvas.height / 2) / zoom;
  }

  /* ---------------------------- Tiles ---------------------------- */

  function getTile(x: number, z: number) {
    const key = `${x},${z}`;
    let tile = tiles.get(key);
    if (!tile) {
      tile = new Tile(x, z);
      tiles.set(key, tile);
      fetchTile(tile);
    }
    return tile;
  }

  async function fetchTile(tile: Tile) {
    if (tile.loading) return;
    tile.loading = true;

    try {
      const bytes = await invoke<number[]>("get_region_map_image", {
        worldPath,
        regionX: tile.x,
        regionZ: tile.z,
        useTextureColors: renderMode === "texture",
        scale: 1,
      });

      const img = new ImageData(
        new Uint8ClampedArray(bytes),
        TILE_SIZE,
        TILE_SIZE
      );

      tile.image = await createImageBitmap(img);
    } catch {
      tile.image = null;
    }
  }

  /* ---------------------------- Render ---------------------------- */

  function render() {
    ctx.fillStyle = "#0b0b0b";
    ctx.fillRect(0, 0, canvas.width, canvas.height);

    const vw = canvas.width / zoom;
    const vh = canvas.height / zoom;

    const left = camX - vw / 2;
    const top = camZ - vh / 2;

    const sx = Math.floor(left / TILE_SIZE);
    const ex = Math.floor((left + vw) / TILE_SIZE);
    const sz = Math.floor(top / TILE_SIZE);
    const ez = Math.floor((top + vh) / TILE_SIZE);

    ctx.imageSmoothingEnabled = false;

    for (let z = sz; z <= ez; z++) {
      for (let x = sx; x <= ex; x++) {
        if (!regionSet.has(`${x},${z}`)) continue;

        const tile = getTile(x, z);
        if (!tile.image) continue;

        const dx = (x * TILE_SIZE - left) * zoom;
        const dy = (z * TILE_SIZE - top) * zoom;
        const size = TILE_SIZE * zoom;

        ctx.drawImage(tile.image, dx, dy, size, size);
      }
    }

    requestAnimationFrame(render);
  }

  /* ---------------------------- Public ---------------------------- */

  function setRenderMode(mode: RenderMode) {
    renderMode = mode;
    tiles.clear();
  }
</script>

<div class="map-root">
  <canvas
    bind:this={canvas}
    on:mousedown={startDrag}
    on:mousemove={onMouseMove}
  />
</div>

<style>
  .map-root {
    flex: 1;
    min-height: 0;
    overflow: hidden;
    padding: 0;
  }

  canvas {
    width: 100%;
    height: 100%;
    display: block;
  }
</style>
