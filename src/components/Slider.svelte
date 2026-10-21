<script lang="ts">
  let {
    value = $bindable(0),
    min = 0,
    max = 1,
    step = 0.01,
    format = (v: number) => String(v),
    label = "",
    width = "180px",
  }: { value?: number; min?: number; max?: number; step?: number; format?: (v: number) => string; label?: string; width?: string } = $props();
  const pct = $derived(((value - min) / (max - min)) * 100);
</script>

<div class="slider" style:width>
  <input type="range" aria-label={label} {min} {max} {step} bind:value style:--pct="{pct}%" />
  <span class="val num">{format(value)}</span>
</div>

<style>
  .slider {
    display: inline-flex;
    align-items: center;
    gap: 12px;
  }
  input {
    flex: 1;
    appearance: none;
    height: 4px;
    border-radius: 4px;
    background: linear-gradient(to right, var(--text) var(--pct), var(--surface-3) var(--pct));
    outline: none;
    cursor: pointer;
  }
  input::-webkit-slider-thumb {
    appearance: none;
    width: 15px;
    height: 15px;
    border-radius: 50%;
    background: var(--text);
    border: 3px solid var(--bg);
    box-shadow: 0 0 0 1px var(--border-strong);
  }
  .val {
    min-width: 44px;
    text-align: right;
    font-size: var(--text-sm);
    color: var(--text-2);
  }
</style>
