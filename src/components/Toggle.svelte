<script lang="ts">
  let {
    checked = $bindable(false),
    disabled = false,
    label = "",
    controlled = false,
    onchange,
  }: { checked?: boolean; disabled?: boolean; label?: string; controlled?: boolean; onchange?: (v: boolean) => void } = $props();

  // `controlled`: el padre decide el valor en `onchange` (p. ej. pedir consentimiento antes).
  function flip() {
    if (disabled) return;
    const next = !checked;
    if (!controlled) checked = next;
    onchange?.(next);
  }
</script>

<button
  type="button"
  role="switch"
  aria-checked={checked}
  aria-label={label}
  class="toggle"
  class:on={checked}
  {disabled}
  onclick={flip}
>
  <span class="knob"></span>
</button>

<style>
  .toggle {
    position: relative;
    width: 36px;
    height: 21px;
    flex: none;
    border-radius: 999px;
    border: 1px solid var(--border-strong);
    background: var(--surface-3);
    cursor: pointer;
    padding: 0;
    transition:
      background var(--normal) var(--ease),
      border-color var(--normal) var(--ease);
  }
  .toggle:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 15px;
    height: 15px;
    border-radius: 50%;
    background: var(--text-2);
    transition:
      transform var(--normal) var(--ease),
      background var(--normal) var(--ease);
  }
  .on {
    background: var(--accent);
    border-color: var(--accent);
  }
  .on .knob {
    transform: translateX(15px);
    background: var(--accent-fg);
  }
</style>
