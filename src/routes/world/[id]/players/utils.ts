import type { InventoryItem } from "$lib/components/player/ItemSlot.svelte";
import type { PotionEffect } from "$lib/components/player/EffectsPanel.svelte";
import type { PlayerStats } from "$lib/components/player/StatsEditor.svelte";
import type {
  FullPlayerData,
  BackendInventoryItem,
  BackendPotionEffect,
  PlayerData,
} from "./types";

// Note: PlayerStats import might be problematic if it's not exported from StatsEditor properly or if I should use my local definition.
// I'll define PlayerStats locally or import if available.
// Actually, PlayerStats is defined in StatsEditor.svelte in the original code.

export function transformItem(item: BackendInventoryItem): InventoryItem {
  return {
    id: item.id,
    count: item.count,
    damage: item.damage,
    maxDamage: item.max_damage,
    customName: item.custom_name,
    enchantments: item.enchantments?.map((e) => ({
      id: e.id,
      level: e.level,
    })),
    tag: item.tag,
  };
}

export function transformEffect(effect: BackendPotionEffect): PotionEffect {
  return {
    id: effect.id,
    amplifier: effect.amplifier,
    duration: effect.duration,
    showParticles: effect.show_particles,
  };
}

export function getInventoryItems(
  data: FullPlayerData
): (InventoryItem | null)[] {
  const result: (InventoryItem | null)[] = new Array(27).fill(null);
  for (const item of data.inventory.main) {
    const slotIndex = item.slot - 9; // slots 9-35 -> index 0-26
    if (slotIndex >= 0 && slotIndex < 27) {
      result[slotIndex] = transformItem(item);
    }
  }
  return result;
}

export function getHotbarItems(data: FullPlayerData): (InventoryItem | null)[] {
  const result: (InventoryItem | null)[] = new Array(9).fill(null);
  for (const item of data.inventory.hotbar) {
    if (item.slot >= 0 && item.slot < 9) {
      result[item.slot] = transformItem(item);
    }
  }
  return result;
}

export function getArmorItems(data: FullPlayerData): (InventoryItem | null)[] {
  // slots 100-103: boots, legs, chest, helmet
  const result: (InventoryItem | null)[] = new Array(4).fill(null);
  for (const item of data.inventory.armor) {
    const slotIndex = item.slot - 100;
    if (slotIndex >= 0 && slotIndex < 4) {
      result[slotIndex] = transformItem(item);
    }
  }
  return result;
}

export function getOffhandItem(data: FullPlayerData): InventoryItem | null {
  return data.inventory.offhand ? transformItem(data.inventory.offhand) : null;
}

export function getEnderItems(data: FullPlayerData): (InventoryItem | null)[] {
  const result: (InventoryItem | null)[] = new Array(27).fill(null);
  for (const item of data.ender_items) {
    if (item.slot >= 0 && item.slot < 27) {
      result[item.slot] = transformItem(item);
    }
  }
  return result;
}

export function getActiveEffects(data: FullPlayerData): PotionEffect[] {
  return data.active_effects.map(transformEffect);
}

export function buildPlayerStats(player: PlayerData): PlayerStats {
  return {
    health: player.health,
    maxHealth: 20, // Default or derived
    foodLevel: player.food_level,
    saturation: 5, // Default
    xpProgress: 0.45, // Placeholder
    xpLevel: player.xp_level,
    xpTotal: player.xp_total,
    gameMode: player.game_mode,
    position: player.position,
    dimension: player.dimension,
  };
}
