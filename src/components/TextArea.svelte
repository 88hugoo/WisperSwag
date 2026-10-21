<script lang="ts">
  let {
    value = $bindable(""),
    placeholder = "",
    rows = 4,
    mono = false,
    label = "",
    readonly = false,
    oninput,
    el = $bindable(),
  }: {
    value?: string;
    placeholder?: string;
    rows?: number;
    mono?: boolean;
    label?: string;
    readonly?: boolean;
    oninput?: (e: Event) => void;
    el?: HTMLTextAreaElement;
  } = $props();
</script>

<textarea bind:this={el} bind:value {placeholder} {rows} {readonly} aria-label={label || placeholder} class:mono spellcheck="false" {oninput}></textarea>

<style>
  textarea {
    width: 100%;
    resize: vertical;
    min-height: 64px;
    padding: 10px 12px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-strong);
    background: var(--surface-2);
    font-size: var(--text-sm);
    line-height: 1.55;
    outline: none;
    transition: border-color var(--fast) var(--ease);
  }
  textarea::placeholder {
    color: var(--text-3);
  }
  textarea:focus {
    border-color: var(--text-2);
    background: var(--surface);
  }
  .mono {
    font-family: var(--mono);
  }
</style>
