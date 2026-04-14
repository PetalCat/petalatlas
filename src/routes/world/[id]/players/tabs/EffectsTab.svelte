<script lang="ts">
  import EffectsPanel from "$lib/components/player/EffectsPanel.svelte";
  import type { PotionEffect } from "$lib/components/player/EffectsPanel.svelte";

  export let effects: PotionEffect[];

  export let onchange: (effect: PotionEffect) => void = () => {};
  export let onadd: (id: string) => void = () => {};
  export let onremove: (id: string) => void = () => {};
</script>

<EffectsPanel {effects} {onchange} {onadd} {onremove} />
