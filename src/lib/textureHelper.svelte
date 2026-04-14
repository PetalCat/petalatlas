<script lang="ts" context="module">
  import { invoke } from "@tauri-apps/api/core";

  // Cache for loaded textures
  const textureCache = new Map<string, string | null>();
  const pendingRequests = new Map<string, Promise<string | null>>();

  /**
   * Get the JAR path from localStorage if configured
   */
  export function getConfiguredJarPath(): string | null {
    if (typeof window === "undefined") return null;
    return localStorage.getItem("selectedJavaJar");
  }

  /**
   * Load an item texture from the configured JAR
   * Returns a base64 data URL or null if not available
   */
  export async function loadItemTexture(
    itemId: string
  ): Promise<string | null> {
    const jarPath = getConfiguredJarPath();
    if (!jarPath) return null;

    // Normalize item ID
    const normalizedId = itemId.replace("minecraft:", "");
    const cacheKey = `${jarPath}:${normalizedId}`;

    // Check cache
    if (textureCache.has(cacheKey)) {
      return textureCache.get(cacheKey) ?? null;
    }

    // Check if request already pending
    if (pendingRequests.has(cacheKey)) {
      return pendingRequests.get(cacheKey)!;
    }

    // Make request
    const promise = (async () => {
      try {
        // Try item texture first, then block texture
        const texturePaths = [`item/${normalizedId}`, `block/${normalizedId}`];

        for (const texturePath of texturePaths) {
          try {
            const result = await invoke<string>("extract_texture", {
              jarPath,
              textureId: texturePath,
            });
            textureCache.set(cacheKey, result);
            return result;
          } catch {
            // Try next path
          }
        }

        textureCache.set(cacheKey, null);
        return null;
      } catch (e) {
        console.error(`Failed to load texture for ${itemId}:`, e);
        textureCache.set(cacheKey, null);
        return null;
      } finally {
        pendingRequests.delete(cacheKey);
      }
    })();

    pendingRequests.set(cacheKey, promise);
    return promise;
  }

  /**
   * Clear the texture cache
   */
  export function clearTextureCache() {
    textureCache.clear();
  }
</script>
