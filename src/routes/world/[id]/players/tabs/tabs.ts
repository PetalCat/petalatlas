import type { SvelteComponent } from "svelte";
import InventoryTab from "./InventoryTab.svelte";
import StatsTab from "./StatsTab.svelte";
import EffectsTab from "./EffectsTab.svelte";
import AdvancementsTab from "./AdvancementsTab.svelte";
import StatisticsTab from "./StatisticsTab.svelte";

export type TabId =
  | "inventory"
  | "stats"
  | "effects"
  | "advancements"
  | "statistics";

export interface TabDefinition {
  id: TabId;
  label: string;
  icon: string;
  component: typeof SvelteComponent<any>;
}

export const TABS: TabDefinition[] = [
  {
    id: "inventory",
    label: "Inventory",
    icon: "📦",
    component: InventoryTab,
  },
  {
    id: "stats",
    label: "Stats",
    icon: "📊",
    component: StatsTab,
  },
  {
    id: "effects",
    label: "Effects",
    icon: "✨",
    component: EffectsTab,
  },
  {
    id: "advancements",
    label: "Advancements",
    icon: "🏆",
    component: AdvancementsTab,
  },
  {
    id: "statistics",
    label: "Statistics",
    icon: "📈",
    component: StatisticsTab,
  },
];
