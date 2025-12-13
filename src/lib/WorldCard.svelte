<script lang="ts">
  import type { WorldInfo } from "$lib/types";

  let { world } = $props<{ world: WorldInfo }>();

  function formatBytes(bytes: number, decimals = 2) {
    if (!+bytes) return "0 Bytes";
    const k = 1024;
    const dm = decimals < 0 ? 0 : decimals;
    const sizes = ["Bytes", "KB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(dm))} ${sizes[i]}`;
  }

  function formatDate(timestamp: number) {
    if (timestamp === 0) return "Never";
    const date = new Date(timestamp);
    const now = Date.now();
    const diff = now - timestamp;
    const days = Math.floor(diff / (1000 * 60 * 60 * 24));

    if (days === 0) return "Today";
    if (days === 1) return "Yesterday";
    if (days < 7) return `${days} days ago`;
    if (days < 30) return `${Math.floor(days / 7)} weeks ago`;
    if (days < 365) return `${Math.floor(days / 30)} months ago`;
    return date.toLocaleDateString();
  }

  // Platform emoji mapping
  const platformEmoji: Record<string, string> = {
    Java: "☕",
    Bedrock: "🪨",
    Unknown: "❓",
  };
</script>

<div class="world-card">
  <div class="card-background"></div>
  <div class="card-content">
    <div class="icon-container">
      {#if world.icon}
        <img
          src="data:image/png;base64,{world.icon}"
          alt="{world.name} icon"
          class="world-icon"
        />
      {:else}
        <div class="world-icon placeholder">
          <span>🌍</span>
        </div>
      {/if}
      {#if world.is_zip}
        <div class="zip-badge">🗜️</div>
      {/if}
    </div>

    <div class="info">
      <div class="header">
        <h3>{world.name}</h3>
        <span class="platform-badge"
          >{platformEmoji[world.platform] || "❓"}</span
        >
      </div>
      <p class="folder-name">{world.folder_name}</p>

      <div class="meta">
        <span class="badge version">{world.version_id}</span>
        <span class="badge mode">{world.game_mode}</span>
      </div>

      <div class="details">
        <div class="detail-row">
          <span class="detail-icon">💾</span>
          <span>{formatBytes(world.size)}</span>
        </div>
        <div class="detail-row">
          <span class="detail-icon">🕐</span>
          <span>{formatDate(world.last_played)}</span>
        </div>
        {#if world.seed}
          <div class="detail-row seed">
            <span class="detail-icon">🌱</span>
            <span class="seed-value">{world.seed}</span>
          </div>
        {/if}
      </div>
    </div>
  </div>
</div>

<style>
  .world-card {
    position: relative;
    border-radius: var(--radius-lg);
    overflow: hidden;
    transition:
      transform 0.3s cubic-bezier(0.4, 0, 0.2, 1),
      box-shadow 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    cursor: pointer;
    box-shadow: var(--shadow-md);
  }

  .world-card:hover {
    transform: translateY(-8px) scale(1.02);
    box-shadow: var(--glow-card);
  }

  .world-card:active {
    transform: translateY(-4px) scale(1.01);
  }

  /* macOS Tahoe translucent background */
  .card-background {
    position: absolute;
    inset: 0;
    background: var(--material-bg-2);
    -webkit-backdrop-filter: blur(30px) saturate(180%);
    backdrop-filter: blur(30px) saturate(180%);
    border: 1px solid rgba(255, 255, 255, 0.18);
  }

  @media (prefers-color-scheme: dark) {
    .card-background {
      background: var(--material-bg-1);
      border: 1px solid rgba(255, 255, 255, 0.06);
    }
  }

  .card-content {
    position: relative;
    display: flex;
    gap: 1.25rem;
    padding: 1.25rem;
  }

  .icon-container {
    position: relative;
    flex-shrink: 0;
    filter: drop-shadow(0 4px 12px rgba(0, 0, 0, 0.1));
  }

  .world-card:hover .icon-container {
    animation: iconFloat 0.6s ease-out;
  }

  @keyframes iconFloat {
    0%,
    100% {
      transform: translateY(0);
    }
    50% {
      transform: translateY(-6px);
    }
  }

  .world-icon {
    width: 96px;
    height: 96px;
    object-fit: cover;
    border-radius: var(--radius-md);
    background: linear-gradient(
      135deg,
      var(--color-bg-2) 0%,
      var(--color-bg-3) 100%
    );
    transition: transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .world-card:hover .world-icon {
    transform: scale(1.05);
  }

  .placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 3rem;
    background: linear-gradient(
      135deg,
      var(--color-primary-light) 0%,
      var(--color-bg-3) 100%
    );
  }

  .zip-badge {
    position: absolute;
    bottom: -6px;
    right: -6px;
    background: var(--color-orange);
    color: white;
    font-size: 1.25rem;
    padding: 0.375rem;
    border-radius: var(--radius-sm);
    box-shadow: var(--shadow-md);
    line-height: 1;
  }

  .info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.625rem;
  }

  .header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 0.75rem;
  }

  h3 {
    margin: 0;
    font-size: 1.25rem;
    font-weight: 600;
    color: var(--color-text-1);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    letter-spacing: -0.02em;
    line-height: 1.3;
  }

  .platform-badge {
    font-size: 1.5rem;
    line-height: 1;
    flex-shrink: 0;
    opacity: 0.7;
    transition: all 0.2s;
  }

  .world-card:hover .platform-badge {
    opacity: 1;
    transform: scale(1.1);
  }

  .folder-name {
    margin: 0;
    font-size: 0.875rem;
    color: var(--color-text-3);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-weight: 500;
  }

  .meta {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .badge {
    font-size: 0.75rem;
    padding: 0.375rem 0.75rem;
    border-radius: var(--radius-md);
    font-weight: 600;
    letter-spacing: -0.01em;
    transition: all 0.2s;
  }

  .badge.version {
    background: var(--color-primary-light);
    color: var(--color-primary);
  }

  .badge.mode {
    background: rgba(52, 199, 89, 0.1);
    color: var(--color-green);
  }

  .world-card:hover .badge {
    transform: translateY(-2px);
  }

  .details {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    margin-top: auto;
  }

  .detail-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.875rem;
    color: var(--color-text-2);
    font-weight: 500;
  }

  .detail-icon {
    font-size: 1rem;
    opacity: 0.6;
  }

  .seed {
    margin-top: 0.25rem;
  }

  .seed-value {
    font-family: "SF Mono", "Monaco", "Cascadia Code", "Courier New", monospace;
    font-size: 0.8125rem;
    color: var(--color-text-3);
    user-select: all;
    padding: 0.25rem 0.5rem;
    background: var(--color-bg-3);
    border-radius: var(--radius-sm);
  }

  /* Responsive adjustments */
  @media (max-width: 640px) {
    .card-content {
      flex-direction: column;
      align-items: center;
      text-align: center;
    }

    .header {
      flex-direction: column;
      align-items: center;
    }

    h3 {
      text-align: center;
    }

    .meta,
    .details {
      align-items: center;
      justify-content: center;
    }
  }
</style>
