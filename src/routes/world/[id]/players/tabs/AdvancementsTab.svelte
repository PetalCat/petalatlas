<script lang="ts">
  import AdvancementsPanel from "$lib/components/player/AdvancementsPanel.svelte";
  import type { AdvancementCategory } from "$lib/components/player/AdvancementsPanel.svelte";

  export let categories: AdvancementCategory[];

  export let ongrant: (id: string) => void = () => {};
  export let onrevoke: (id: string) => void = () => {};
</script>

<AdvancementsPanel {categories} {ongrant} {onrevoke} />
