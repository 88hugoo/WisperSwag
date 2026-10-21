<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon from "./Icon.svelte";

  let { icon = "info", title, text, children }: { icon?: string; title: string; text?: string; children?: Snippet } = $props();
</script>

<div class="empty">
  <div class="ic"><Icon name={icon} size={20} /></div>
  <div class="t">{title}</div>
  {#if text}<p class="muted">{text}</p>{/if}
  {#if children}<div class="a">{@render children()}</div>{/if}
</div>

<style>
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    padding: 48px 24px;
  }
  .ic {
    width: 44px;
    height: 44px;
    display: grid;
    place-items: center;
    border-radius: 12px;
    border: 1px solid var(--border-strong);
    color: var(--text-2);
    margin-bottom: 14px;
  }
  .t {
    font-weight: 560;
  }
  p {
    margin-top: 4px;
    font-size: var(--text-sm);
    max-width: 380px;
  }
  .a {
    margin-top: 16px;
    display: flex;
    gap: 8px;
  }
</style>
