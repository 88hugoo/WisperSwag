<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    title,
    description,
    stacked = false,
    disabled = false,
    children,
    extra,
  }: { title: string; description?: string; stacked?: boolean; disabled?: boolean; children?: Snippet; extra?: Snippet } = $props();
</script>

<div class="row" class:stacked class:disabled>
  <div class="text">
    <div class="title">{title}</div>
    {#if description}<div class="desc">{description}</div>{/if}
    {#if extra}{@render extra()}{/if}
  </div>
  {#if children}<div class="control">{@render children()}</div>{/if}
</div>

<style>
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 10px 24px;
    padding: 14px 18px;
    min-height: 58px;
  }
  .row + :global(.row) {
    border-top: 1px solid var(--border);
  }
  .text {
    min-width: 0;
    /* Si el control no cabe al lado de un texto legible, pasa a la línea siguiente. */
    flex: 1 1 220px;
  }
  .title {
    font-size: var(--text-md);
    font-weight: 520;
    letter-spacing: -0.005em;
  }
  .desc {
    margin-top: 2px;
    font-size: var(--text-sm);
    color: var(--text-2);
    line-height: 1.45;
    max-width: 560px;
  }
  .control {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
    flex: none;
    margin-left: auto;
    max-width: 100%;
  }
  .stacked {
    flex-direction: column;
    align-items: stretch;
    gap: 12px;
  }
  .stacked .text {
    flex: none;
  }
  .stacked .control {
    flex: 1;
    flex-wrap: wrap;
    justify-content: flex-start;
    margin-left: 0;
  }
  .disabled {
    opacity: 0.5;
    pointer-events: none;
  }
</style>
