<script lang="ts">
  import { windowAction } from "../lib/api";
  import { app, tr } from "../lib/store.svelte";
  import Icon from "./Icon.svelte";
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="titlebar" onmousedown={(e) => e.button === 0 && e.target === e.currentTarget && windowAction("drag")} ondblclick={(e) => e.target === e.currentTarget && windowAction("maximize")}>
  <div class="brand" data-tauri-drag-region>
    <span class="logo" aria-hidden="true">
      <span></span><span></span><span></span><span></span><span></span>
    </span>
    <span class="name">Wispr Swag</span>
    {#if app.status.phase === "recording"}
      <span class="rec">{tr("Grabando", "Recording")}</span>
    {:else if app.status.phase === "processing"}
      <span class="rec dim">{tr("Procesando", "Processing")}</span>
    {/if}
  </div>
  <div class="win">
    <button onclick={() => windowAction("minimize")} aria-label={tr("Minimizar", "Minimize")}><Icon name="minimize" size={14} /></button>
    <button onclick={() => windowAction("maximize")} aria-label={tr("Maximizar", "Maximize")}><Icon name="maximize" size={13} /></button>
    <button class="close" onclick={() => windowAction("close")} aria-label={tr("Cerrar", "Close")}><Icon name="close" size={15} /></button>
  </div>
</div>

<style>
  .titlebar {
    height: 40px;
    flex: none;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-left: 16px;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    pointer-events: none;
  }
  .logo {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    height: 16px;
    padding: 0 5px;
    border-radius: 6px;
    background: var(--text);
  }
  .logo span {
    width: 2px;
    border-radius: 2px;
    background: var(--bg);
  }
  .logo span:nth-child(1),
  .logo span:nth-child(5) {
    height: 4px;
  }
  .logo span:nth-child(2),
  .logo span:nth-child(4) {
    height: 7px;
  }
  .logo span:nth-child(3) {
    height: 10px;
  }
  .name {
    font-size: var(--text-sm);
    font-weight: 600;
    letter-spacing: -0.01em;
  }
  .rec {
    font-size: var(--text-xs);
    font-weight: 560;
    padding: 1px 8px;
    border-radius: 999px;
    background: var(--text);
    color: var(--bg);
  }
  .rec.dim {
    background: var(--surface-3);
    color: var(--text-2);
  }
  .win {
    display: flex;
    height: 100%;
  }
  .win button {
    width: 46px;
    height: 100%;
    display: grid;
    place-items: center;
    border: 0;
    background: transparent;
    color: var(--text-2);
    cursor: pointer;
    transition: background var(--fast) var(--ease);
  }
  .win button:hover {
    background: var(--hover);
    color: var(--text);
  }
  .win .close:hover {
    background: var(--text);
    color: var(--bg);
  }
</style>
