<script lang="ts" module>
  export interface MenuItem {
    label: string;
    icon?: string;
    hint?: string;
    action: () => void;
    separator?: boolean;
    disabled?: boolean;
  }
</script>

<script lang="ts">
  import Icon from "./Icon.svelte";

  let { items, icon = "dots", label = "", title = "" }: { items: MenuItem[]; icon?: string; label?: string; title?: string } = $props();
  let open = $state(false);
  let root: HTMLDivElement | undefined = $state();

  function onDoc(e: MouseEvent) {
    if (open && root && !root.contains(e.target as Node)) open = false;
  }
</script>

<svelte:document onmousedown={onDoc} />

<div class="menu" bind:this={root}>
  <button class="trigger" class:labelled={!!label} class:open aria-haspopup="menu" aria-expanded={open} title={title || label} onclick={() => (open = !open)}>
    <Icon name={icon} size={15} />
    {#if label}<span>{label}</span><Icon name="chevron-down" size={13} />{/if}
  </button>
  {#if open}
    <div class="list" role="menu">
      {#each items as it}
        {#if it.separator}<div class="sep"></div>{/if}
        <button
          role="menuitem"
          disabled={it.disabled}
          onclick={() => {
            open = false;
            it.action();
          }}
        >
          {#if it.icon}<Icon name={it.icon} size={14} />{:else}<span class="sp"></span>{/if}
          <span class="l">{it.label}</span>
          {#if it.hint}<span class="h">{it.hint}</span>{/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .menu {
    position: relative;
    display: inline-flex;
  }
  .trigger {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    min-width: 28px;
    justify-content: center;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-2);
    cursor: pointer;
    font-size: var(--text-xs);
    font-weight: 550;
  }
  .trigger.labelled {
    padding: 0 9px;
    border: 1px solid var(--border-strong);
    height: 28px;
  }
  .trigger:hover,
  .trigger.open {
    background: var(--hover);
    color: var(--text);
  }
  .list {
    position: absolute;
    right: 0;
    top: calc(100% + 6px);
    min-width: 230px;
    padding: 5px;
    border-radius: var(--radius);
    background: var(--surface);
    border: 1px solid var(--border-strong);
    box-shadow: var(--shadow-lg);
    z-index: 40;
    animation: pop var(--fast) var(--ease);
  }
  .list button {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    height: 32px;
    padding: 0 10px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    font-size: var(--text-sm);
    text-align: left;
    cursor: pointer;
    color: var(--text);
  }
  .list button:hover:not(:disabled) {
    background: var(--hover);
  }
  .list button:disabled {
    opacity: 0.4;
  }
  .l {
    flex: 1;
  }
  .h {
    font-size: var(--text-xs);
    color: var(--text-3);
  }
  .sp {
    width: 14px;
  }
  .sep {
    height: 1px;
    margin: 5px 6px;
    background: var(--border);
  }
  @keyframes pop {
    from {
      opacity: 0;
      transform: translateY(-3px);
    }
  }
</style>
