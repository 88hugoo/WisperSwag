<script lang="ts">
  import Icon from "./Icon.svelte";

  let {
    value = $bindable(),
    options,
    disabled = false,
    width,
    label = "",
    onchange,
  }: {
    value: any;
    options: { value: any; label: string; disabled?: boolean }[];
    disabled?: boolean;
    width?: string;
    label?: string;
    onchange?: (v: any) => void;
  } = $props();

  const index = $derived(options.findIndex((o) => o.value === value));
</script>

<label class="select" style:width>
  <select
    aria-label={label}
    {disabled}
    value={index}
    onchange={(e) => {
      const o = options[Number((e.target as HTMLSelectElement).value)];
      if (o) {
        value = o.value;
        onchange?.(o.value);
      }
    }}
  >
    {#each options as o, i}
      <option value={i} disabled={o.disabled}>{o.label}</option>
    {/each}
  </select>
  <span class="chev"><Icon name="chevron-down" size={14} /></span>
</label>

<style>
  .select {
    position: relative;
    display: inline-flex;
    min-width: 160px;
  }
  select {
    appearance: none;
    width: 100%;
    height: 32px;
    padding: 0 30px 0 11px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-strong);
    background: var(--surface-2);
    font-size: var(--text-sm);
    cursor: pointer;
    outline: none;
    transition: border-color var(--fast) var(--ease);
  }
  select:hover:not(:disabled) {
    border-color: var(--text-3);
  }
  select:disabled {
    opacity: 0.45;
    cursor: default;
  }
  select option {
    background: var(--surface);
    color: var(--text);
  }
  .chev {
    position: absolute;
    right: 9px;
    top: 50%;
    transform: translateY(-50%);
    pointer-events: none;
    color: var(--text-3);
  }
</style>
