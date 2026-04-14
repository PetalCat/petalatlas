export interface PlayerData {
  edition: string; // "java" or "bedrock"
  primary_id: string; // UUID for Java, XUID for Bedrock
  display_name?: string; // Cached name (not authoritative)
  position: { x: number; y: number; z: number };
  health: number;
  food_level: number;
  xp_level: number;
  xp_total: number;
  game_mode: number;
  dimension: string;
  last_seen?: number;
}

export interface BackendInventoryItem {
  id: string;
  count: number;
  slot: number;
  damage?: number;
  max_damage?: number;
  custom_name?: string;
  enchantments?: { id: string; level: number }[];
  tag?: any;
}

export interface BackendPotionEffect {
  id: string;
  amplifier: number;
  duration: number;
  show_particles: boolean;
}

export interface BackendAdvancementCriterion {
  id: string;
  name: string;
  done: boolean;
}

export interface BackendAdvancement {
  id: string;
  title: string;
  description: string;
  completed: boolean;
  criteria: BackendAdvancementCriterion[];
}

export interface BackendAdvancementCategory {
  id: string;
  name: string;
  icon: string;
  advancements: BackendAdvancement[];
}

export interface PlayerInventory {
  main: BackendInventoryItem[];
  hotbar: BackendInventoryItem[];
  armor: BackendInventoryItem[];
  offhand: BackendInventoryItem | null;
}

export interface FullPlayerData {
  edition: string;
  primary_id: string;
  display_name?: string;
  position: { x: number; y: number; z: number };
  health: number;
  max_health: number;
  food_level: number;
  saturation: number;
  xp_level: number;
  xp_progress: number;
  xp_total: number;
  game_mode: number;
  dimension: string;
  inventory: PlayerInventory;
  ender_items: BackendInventoryItem[];
  active_effects: BackendPotionEffect[];
  spawn_x?: number;
  spawn_y?: number;
  spawn_z?: number;
  spawn_dimension?: string;
}
