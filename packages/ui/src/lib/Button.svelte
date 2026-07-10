<script lang="ts">
  let { children } = $props();
</script>

<button class="runyard-button">
  {@render children()}
</button>

<style>
  .runyard-button {
    padding: 0.5rem 1rem;
    border-radius: 4px;
    border: 1px solid #ccc;
    background: #fff;
    cursor: pointer;
  }
</style>
