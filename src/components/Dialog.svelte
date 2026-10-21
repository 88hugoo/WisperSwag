<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon from "./Icon.svelte";

  let {
    open = $bindable(false),
    title,
    width = "460px",
    children,
    actions,
    onclose,
  }: { open?: boolean; title: string; width?: string; children: Snippet; actions?: Snippet; onclose?: () => void } = $props();

  function close() {
    open = false;
    onclose?.();
  }

  // Lleva el foco al diálogo al abrirlo (teclado y lectores de pantalla).
  function focusOnOpen(node: HTMLElement) {
    const first = node.querySelector<HTMLElement>("input, textarea, select");
    (first ?? node).focus();
  }

  function onkey(e: KeyboardEvent) {
    if (open && e.key === "Escape") close();
  }
</script>

<svelte:window onkeydown={onkey} />

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="scrim" onclick={close}>
    <div class="dialog" style:width role="dialog" aria-modal="true" aria-label={title} tabindex="-1" use:focusOnOpen onclick={(e) => e.stopPropagation()}>
      <header>
        <h3>{title}</h3>
        <button class="x" onclick={close} aria-label="Cerrar"><Icon name="x" size={16} /></button>
      </header>
      <div class="body">{@render children()}</div>
      {#if actions}<footer>{@render actions()}</footer>{/if}
    </div>
  </div>
{/if}

<style>
  .scrim {
    position: fixed;
    inset: 0;
    background: var(--scrim);
    display: grid;
    place-items: center;
    z-index: 50;
    animation: fade var(--normal) var(--ease);
  }
  .dialog {
    max-width: calc(100vw - 48px);
    max-height: calc(100vh - 80px);
    display: flex;
    flex-direction: column;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg);
    animation: pop var(--normal) var(--ease);
    outline: none;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 18px 20px 6px;
  }
  .x {
    border: 0;
    background: transparent;
    color: var(--text-3);
    cursor: pointer;
    padding: 4px;
    border-radius: 6px;
  }
  .x:hover {
    color: var(--text);
    background: var(--hover);
  }
  .body {
    padding: 8px 20px 18px;
    overflow: auto;
    color: var(--text-2);
    font-size: var(--text-md);
  }
  footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 14px 20px;
    border-top: 1px solid var(--border);
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }
  @keyframes pop {
    from {
      opacity: 0;
      transform: translateY(6px) scale(0.985);
    }
  }
</style>
