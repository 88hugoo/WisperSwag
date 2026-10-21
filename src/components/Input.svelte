<script lang="ts">
  import Icon from "./Icon.svelte";

  let {
    value = $bindable(""),
    placeholder = "",
    type = "text",
    icon,
    mono = false,
    width,
    disabled = false,
    label = "",
    invalid = false,
    onkeydown,
    oninput,
    onblur,
    autofocus = false,
  }: {
    value?: string | number;
    placeholder?: string;
    type?: string;
    icon?: string;
    mono?: boolean;
    width?: string;
    disabled?: boolean;
    label?: string;
    invalid?: boolean;
    onkeydown?: (e: KeyboardEvent) => void;
    oninput?: (e: Event) => void;
    onblur?: (e: Event) => void;
    autofocus?: boolean;
  } = $props();

  let el: HTMLInputElement | undefined = $state();
  $effect(() => {
    if (autofocus) el?.focus();
  });
</script>

<label class="input" class:invalid class:has-icon={!!icon} style:width>
  {#if icon}<span class="icon"><Icon name={icon} size={14} /></span>{/if}
  <input
    bind:this={el}
    bind:value
    {type}
    {placeholder}
    {disabled}
    aria-label={label || placeholder}
    class:mono
    spellcheck="false"
    autocomplete="off"
    {onkeydown}
    {oninput}
    {onblur}
  />
</label>

<style>
  .input {
    position: relative;
    display: inline-flex;
    align-items: center;
    min-width: 0;
  }
  input {
    width: 100%;
    height: 32px;
    padding: 0 11px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-strong);
    background: var(--surface-2);
    font-size: var(--text-sm);
    outline: none;
    transition:
      border-color var(--fast) var(--ease),
      background var(--fast) var(--ease);
  }
  input::placeholder {
    color: var(--text-3);
  }
  input:hover:not(:disabled) {
    border-color: var(--text-3);
  }
  input:focus {
    border-color: var(--text-2);
    background: var(--surface);
  }
  input:disabled {
    opacity: 0.45;
  }
  .has-icon input {
    padding-left: 32px;
  }
  .icon {
    position: absolute;
    left: 11px;
    color: var(--text-3);
    pointer-events: none;
  }
  .invalid input {
    border-color: var(--text);
    border-style: dashed;
  }
  .mono {
    font-family: var(--mono);
  }
</style>
