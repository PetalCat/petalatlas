<script lang="ts">
  import StatisticsPanel from "$lib/components/player/StatisticsPanel.svelte";
  import type { StatCategory } from "$lib/components/player/StatisticsPanel.svelte";

  export let categories: StatCategory[];

  export let onchange: (
    category: string,
    stat: string,
    value: number
  ) => void = () => {};
</script>

<StatisticsPanel {categories} {onchange} />
