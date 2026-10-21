<script lang="ts">
  let { keys, size = "md" }: { keys: string; size?: "sm" | "md" | "lg" } = $props();
  const parts = $derived(keys ? keys.split(" + ") : []);
</script>

<span class="combo {size}">
  {#each parts as p, i}
    {#if i > 0}<span class="plus">+</span>{/if}
    <kbd>{p}</kbd>
  {/each}
</span>

<style>
  .combo {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .plus {
    color: var(--text-3);
    font-size: var(--text-xs);
  }
  .lg :global(kbd) {
    font-size: var(--text-md);
    padding: 4px 11px;
    border-radius: 8px;
  }
  .sm :global(kbd) {
    font-size: 10.5px;
    padding: 1px 6px;
  }
</style>
