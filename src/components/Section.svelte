<script lang="ts">
  import type { Snippet } from "svelte";

  let { title, description, children, actions }: { title?: string; description?: string; children: Snippet; actions?: Snippet } = $props();
</script>

<section>
  {#if title || actions}
    <header>
      <div>
        {#if title}<h3>{title}</h3>{/if}
        {#if description}<p class="muted">{description}</p>{/if}
      </div>
      {#if actions}<div class="actions">{@render actions()}</div>{/if}
    </header>
  {/if}
  <div class="card">{@render children()}</div>
</section>

<style>
  section {
    margin-bottom: 28px;
  }
  header {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 16px;
    margin: 0 2px 10px;
  }
  h3 {
    font-size: var(--text-sm);
    font-weight: 600;
    letter-spacing: 0.01em;
    color: var(--text-2);
    text-transform: none;
  }
  header p {
    font-size: var(--text-sm);
    margin-top: 2px;
  }
  .actions {
    display: flex;
    gap: 8px;
  }
  .card {
    border: 1px solid var(--border);
    background: var(--surface);
    border-radius: var(--radius);
    overflow: hidden;
  }
</style>
