<script lang="ts">
  import StatsEditor from "$lib/components/player/StatsEditor.svelte";
  import type { PlayerStats } from "$lib/components/player/StatsEditor.svelte";

  export let stats: PlayerStats;
  export let onchange: (field: string, value: unknown) => void;
</script>

<StatsEditor {stats} {onchange} />
