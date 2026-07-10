<script lang="ts">
  import type { Tab } from "@runyard/common";
  let { tab } = $props<{ tab: Tab }>();
</script>
<div class="placeholder">Placeholder for: {tab.type} ({tab.title})</div>
<style>
  .placeholder { padding: 1rem; color: #666; font-style: italic; }
</style>
