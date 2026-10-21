<script lang="ts">
  import { api, errorText, on } from "../lib/api";
  import { app, tr } from "../lib/store.svelte";
  import type { HotkeyConflict } from "../lib/types";
  import Button from "./Button.svelte";
  import Kbd from "./Kbd.svelte";

  let {
    value = $bindable(),
    action,
    fallback,
    clearable = true,
    align = "end",
  }: { value: string | null; action: string; fallback: string | null; clearable?: boolean; align?: "start" | "end" } = $props();

  let display = $state("");
  let capturing = $state(false);
  let conflicts = $state<HotkeyConflict[]>([]);
  let unlisten: (() => void) | null = null;

  $effect(() => {
    const v = value;
    if (!v) {
      display = "";
      return;
    }
    api.hotkeyDisplay(v).then((d) => (display = d));
  });

  async function start() {
    conflicts = [];
    try {
      unlisten = await on("hotkey-captured", onCaptured);
      await api.hotkeyCaptureStart();
      capturing = true;
    } catch (e) {
      unlisten?.();
      app.toast(errorText(e), "error");
    }
  }

  async function onCaptured(p: { hotkey: string | null }) {
    stop();
    if (!p.hotkey) return;
    const check = await api.hotkeyCheck(action, p.hotkey);
    conflicts = check.conflicts;
    if (check.valid) value = p.hotkey;
  }

  function stop() {
    capturing = false;
    unlisten?.();
    unlisten = null;
    api.hotkeyCaptureStop();
  }

  $effect(() => () => {
    if (capturing) stop();
  });
</script>

<div class="field" class:start={align === "start"}>
<div class="hk">
  <div class="current">
    {#if capturing}
      <span class="listening"><span class="dot"></span>{tr("Pulsa la combinación… (Esc para cancelar)", "Press the shortcut… (Esc to cancel)")}</span>
    {:else if value}
      <Kbd keys={display} />
    {:else}
      <span class="faint">{tr("Sin asignar", "Not set")}</span>
    {/if}
  </div>
  <div class="btns">
    {#if capturing}
      <Button size="sm" variant="ghost" onclick={stop}>{tr("Cancelar", "Cancel")}</Button>
    {:else}
      <Button size="sm" onclick={start}>{tr("Cambiar", "Change")}</Button>
      {#if fallback && value !== fallback}
        <Button size="sm" variant="ghost" icon="refresh" title={tr("Restablecer", "Reset")} onclick={() => ((value = fallback), (conflicts = []))} />
      {/if}
      {#if clearable && value}
        <Button size="sm" variant="ghost" icon="x" title={tr("Quitar atajo", "Remove shortcut")} onclick={() => ((value = null), (conflicts = []))} />
      {/if}
    {/if}
  </div>
</div>
{#if conflicts.length}
  <ul class="conflicts">
    {#each conflicts as c}
      <li class={c.severity}>{c.severity === "error" ? tr("No disponible: ", "Unavailable: ") : tr("Aviso: ", "Note: ")}{c.message}</li>
    {/each}
  </ul>
{/if}
</div>

<style>
  .field {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    max-width: 100%;
  }
  .field.start {
    align-items: flex-start;
  }
  .hk {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .current {
    display: flex;
    justify-content: flex-end;
  }
  .btns {
    display: flex;
    gap: 4px;
  }
  .listening {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: var(--text-sm);
    color: var(--text);
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--text);
    animation: blink 1s ease-in-out infinite;
  }
  @keyframes blink {
    50% {
      opacity: 0.25;
    }
  }
  .conflicts {
    list-style: none;
    margin: 8px 0 0;
    padding: 0;
    max-width: 340px;
    font-size: var(--text-sm);
    line-height: 1.45;
    text-align: right;
  }
  .start .conflicts {
    text-align: left;
  }
  .conflicts .error {
    color: var(--text);
    font-weight: 540;
  }
  .conflicts .warning {
    color: var(--text-2);
  }
</style>
