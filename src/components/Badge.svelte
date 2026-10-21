<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon from "./Icon.svelte";

  let { icon, solid = false, children }: { icon?: string; solid?: boolean; children: Snippet } = $props();
</script>

<span class="badge" class:solid>
  {#if icon}<Icon name={icon} size={12} />{/if}
  {@render children()}
</span>

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 21px;
    padding: 0 8px;
    border-radius: 999px;
    border: 1px solid var(--border-strong);
    font-size: var(--text-xs);
    font-weight: 540;
    color: var(--text-2);
    white-space: nowrap;
  }
  .solid {
    background: var(--text);
    border-color: var(--text);
    color: var(--bg);
  }
</style>
