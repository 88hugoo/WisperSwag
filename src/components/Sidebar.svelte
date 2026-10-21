<script lang="ts">
  import { api } from "../lib/api";
  import { router } from "../lib/router.svelte";
  import { app, tr } from "../lib/store.svelte";
  import Icon from "./Icon.svelte";
  import Kbd from "./Kbd.svelte";

  const items = $derived([
    { path: "", icon: "home", label: tr("Inicio", "Home") },
    { path: "history", icon: "history", label: tr("Historial", "History") },
    { path: "vocabulary", icon: "book", label: tr("Vocabulario", "Vocabulary") },
    { path: "snippets", icon: "snippet", label: tr("Fragmentos", "Snippets") },
    { path: "modes", icon: "modes", label: tr("Modos de escritura", "Writing modes") },
    { path: "apps", icon: "apps", label: tr("Aplicaciones", "Apps") },
    { path: "commands", icon: "command", label: tr("Comandos de voz", "Voice commands") },
  ]);

  let shortcut = $state("");
  $effect(() => {
    const hk = app.settings.dictation.hotkeys.dictate;
    if (hk) api.hotkeyDisplay(hk).then((d) => (shortcut = d));
    else shortcut = "";
  });

  const engineLabel = $derived(
    app.settings.engine.kind === "local" ? tr("Local · sin conexión", "Local · offline") : tr("Nube", "Cloud"),
  );
</script>

<nav class="sidebar">
  <div class="items">
    {#each items as it}
      <a href="#/{it.path}" class:active={router.section === it.path} aria-current={router.section === it.path ? "page" : undefined}>
        <Icon name={it.icon} size={16} />
        <span>{it.label}</span>
      </a>
    {/each}
  </div>
  <div class="bottom">
    <a href="#/settings" class:active={router.section === "settings"}>
      <Icon name="settings" size={16} />
      <span>{tr("Ajustes", "Settings")}</span>
    </a>
    <div class="status">
      <div class="line">
        <span class="dot" class:rec={app.status.phase === "recording"} class:paused={app.settings.dictation.hotkeys_paused}></span>
        <span class="state">
          {#if app.status.phase === "recording"}{tr("Escuchando…", "Listening…")}
          {:else if app.status.phase === "processing"}{tr("Procesando…", "Processing…")}
          {:else if app.settings.dictation.hotkeys_paused}{tr("Atajos en pausa", "Shortcuts paused")}
          {:else}{tr("Listo para dictar", "Ready to dictate")}{/if}
        </span>
      </div>
      {#if shortcut}<div class="kbd"><Kbd keys={shortcut} size="sm" /></div>{/if}
      <div class="engine faint">{engineLabel}</div>
    </div>
  </div>
</nav>

<style>
  .sidebar {
    width: 228px;
    flex: none;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    padding: 8px 12px 14px;
  }
  .items,
  .bottom {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  a {
    display: flex;
    align-items: center;
    gap: 11px;
    height: 34px;
    padding: 0 11px;
    border-radius: var(--radius-sm);
    color: var(--text-2);
    text-decoration: none;
    font-size: var(--text-md);
    font-weight: 500;
    transition:
      background var(--fast) var(--ease),
      color var(--fast) var(--ease);
  }
  a:hover {
    background: var(--hover);
    color: var(--text);
  }
  a.active {
    background: var(--active);
    color: var(--text);
  }
  .status {
    margin-top: 10px;
    padding: 12px 12px 11px;
    border-radius: var(--radius);
    border: 1px solid var(--border);
    background: var(--surface);
  }
  .line {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: var(--text-sm);
    font-weight: 540;
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--text-3);
    box-shadow: 0 0 0 3px var(--hover);
  }
  .dot.rec {
    background: var(--text);
    animation: pulse 1.2s ease-in-out infinite;
  }
  .dot.paused {
    background: transparent;
    border: 1.5px solid var(--text-3);
  }
  @keyframes pulse {
    50% {
      box-shadow: 0 0 0 5px var(--active);
    }
  }
  .kbd {
    margin-top: 9px;
  }
  .engine {
    margin-top: 8px;
    font-size: var(--text-xs);
  }
</style>
