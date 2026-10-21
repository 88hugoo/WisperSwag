<script lang="ts">
  import Icon from "./Icon.svelte";

  let {
    value = $bindable(),
    options,
    onchange,
    size = "md",
  }: {
    value: any;
    options: { value: any; label: string; icon?: string }[];
    onchange?: (v: any) => void;
    size?: "sm" | "md";
  } = $props();
</script>

<div class="seg {size}" role="radiogroup">
  {#each options as o}
    <button
      type="button"
      role="radio"
      aria-checked={value === o.value}
      class:active={value === o.value}
      onclick={() => {
        value = o.value;
        onchange?.(o.value);
      }}
    >
      {#if o.icon}<Icon name={o.icon} size={14} />{/if}
      {o.label}
    </button>
  {/each}
</div>

<style>
  .seg {
    display: inline-flex;
    padding: 3px;
    gap: 2px;
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    border: 1px solid var(--border);
  }
  button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 26px;
    padding: 0 11px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--text-2);
    font-size: var(--text-sm);
    font-weight: 520;
    cursor: pointer;
    transition:
      background var(--fast) var(--ease),
      color var(--fast) var(--ease);
  }
  button:hover {
    color: var(--text);
  }
  .active {
    background: var(--surface);
    color: var(--text);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.18), 0 0 0 1px var(--border);
  }
  .sm button {
    height: 22px;
    padding: 0 9px;
    font-size: var(--text-xs);
  }
</style>
