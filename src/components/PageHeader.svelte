<script lang="ts">
  import type { Snippet } from "svelte";

  let { title, description, actions }: { title: string; description?: string; actions?: Snippet } = $props();
</script>

<header class="ph">
  <div>
    <h1>{title}</h1>
    {#if description}<p class="muted">{description}</p>{/if}
  </div>
  {#if actions}<div class="actions">{@render actions()}</div>{/if}
</header>

<style>
  .ph {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 20px;
    margin-bottom: 28px;
  }
  h1 {
    font-size: 26px;
  }
  p {
    margin-top: 6px;
    font-size: var(--text-md);
    max-width: 620px;
  }
  .actions {
    display: flex;
    gap: 8px;
    flex: none;
  }
</style>
