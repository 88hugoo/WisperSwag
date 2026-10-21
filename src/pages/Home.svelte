<script lang="ts">
  import { onMount } from "svelte";
  import { api, errorText, on } from "../lib/api";
  import { appName, minutes, number, relativeDate } from "../lib/format";
  import { router } from "../lib/router.svelte";
  import { app, tr } from "../lib/store.svelte";
  import type { HistoryEntry, ModelsView, Stats } from "../lib/types";
  import Button from "../components/Button.svelte";
  import Icon from "../components/Icon.svelte";
  import Kbd from "../components/Kbd.svelte";
  import TextArea from "../components/TextArea.svelte";

  let stats = $state<Stats | null>(null);
  let recent = $state<HistoryEntry[]>([]);
  let models = $state<ModelsView | null>(null);
  let shortcut = $state("");
  let testText = $state("");
  let hover = $state<number | null>(null);

  async function load() {
    try {
      [stats, models] = await Promise.all([api.stats(), api.listModels()]);
      recent = (await api.historyQuery({ search: "", favorites_only: false, oldest_first: false, limit: 4, offset: 0 })).entries;
    } catch (e) {
      app.toast(errorText(e), "error");
    }
  }

  onMount(() => {
    load();
    const offs = [on("history-changed", load)];
    return () => offs.forEach((p) => p.then((f) => f()));
  });

  $effect(() => {
    const hk = app.settings.dictation.hotkeys.dictate;
    if (hk) api.hotkeyDisplay(hk).then((d) => (shortcut = d));
  });

  const greeting = $derived.by(() => {
    const h = new Date().getHours();
    if (h < 6) return tr("Buenas noches", "Good evening");
    if (h < 14) return tr("Buenos días", "Good morning");
    if (h < 21) return tr("Buenas tardes", "Good afternoon");
    return tr("Buenas noches", "Good evening");
  });

  const maxWords = $derived(Math.max(1, ...(stats?.last_30_days.map((d) => d.words) ?? [1])));
  const needsModel = $derived(app.settings.engine.kind === "local" && models !== null && models.installed.length === 0);
  const activation = $derived(app.settings.dictation.activation);

  function copy(e: HistoryEntry) {
    api.historyCopy(e.id).then(() => app.toast(tr("Copiado al portapapeles", "Copied to clipboard"), "success"));
  }
</script>

<header class="hero-head">
  <h1>{greeting}</h1>
  <p class="muted">
    {tr("Dicta en cualquier aplicación de Windows.", "Dictate in any Windows app.")}
    {#if shortcut}<span class="inline">{tr("Pulsa", "Press")} <Kbd keys={shortcut} size="sm" /></span>{/if}
  </p>
</header>

{#if needsModel}
  <button class="callout" onclick={() => router.go("settings/engine")}>
    <Icon name="download" size={18} />
    <div>
      <div class="ct">{tr("Descarga un modelo de voz para empezar", "Download a speech model to get started")}</div>
      <div class="cd">{tr("El reconocimiento local funciona sin conexión. Parakeet V3 (465 MB) es el recomendado para español.", "Local recognition works offline. Parakeet V3 (465 MB) is recommended for Spanish.")}</div>
    </div>
    <Icon name="chevron-right" size={16} />
  </button>
{/if}
{#if app.boot && !app.boot.hotkeys_available && app.boot.platform === "windows"}
  <div class="callout static">
    <Icon name="alert" size={18} />
    <div>
      <div class="ct">{tr("Los atajos globales no están activos", "Global shortcuts are not active")}</div>
      <div class="cd">{app.boot.hotkey_error ?? ""}</div>
    </div>
  </div>
{/if}
{#if app.settings.dictation.hotkeys_paused}
  <div class="callout static">
    <Icon name="pause" size={18} />
    <div>
      <div class="ct">{tr("Atajos en pausa", "Shortcuts paused")}</div>
      <div class="cd">{tr("Los atajos no responden hasta que los reactives.", "Shortcuts won't respond until you resume them.")}</div>
    </div>
    <Button size="sm" onclick={() => api.togglePause()}>{tr("Reanudar", "Resume")}</Button>
  </div>
{/if}

<section class="hero">
  <div class="hero-main">
    <div class="state">
      <span class="ring" class:rec={app.status.phase === "recording"} class:proc={app.status.phase === "processing"}>
        <Icon name={app.status.phase === "recording" ? "wave" : "mic"} size={20} />
      </span>
      <div>
        <div class="st">
          {#if app.status.phase === "recording"}{tr("Escuchando", "Listening")}
          {:else if app.status.phase === "processing"}{tr("Procesando…", "Processing…")}
          {:else}{tr("Listo para dictar", "Ready to dictate")}{/if}
        </div>
        <div class="sd muted">
          {#if activation === "hybrid"}
            {tr("Mantén pulsado para hablar · toque corto para manos libres · Esc cancela", "Hold to talk · quick tap for hands-free · Esc cancels")}
          {:else if activation === "toggle"}
            {tr("Pulsa para empezar y otra vez para terminar · Esc cancela", "Press to start and again to finish · Esc cancels")}
          {:else}
            {tr("Mantén pulsado mientras hablas · Esc cancela", "Hold while you speak · Esc cancels")}
          {/if}
        </div>
      </div>
    </div>
    <Button variant={app.status.phase === "recording" ? "primary" : "secondary"} icon={app.status.phase === "recording" ? "stop" : "mic"} onclick={() => api.toggleDictation()}>
      {app.status.phase === "recording" ? tr("Terminar", "Finish") : tr("Dictar ahora", "Dictate now")}
    </Button>
  </div>
  <div class="try">
    <TextArea bind:value={testText} rows={2} placeholder={tr("Haz clic aquí, pulsa el atajo y habla: el texto aparecerá en este campo.", "Click here, press the shortcut and speak: the text will appear in this field.")} label={tr("Zona de prueba", "Test area")} />
  </div>
</section>

{#if stats}
  <section class="tiles">
    <div class="tile">
      <div class="tv num">{number(stats.today_words)}</div>
      <div class="tl">{tr("palabras hoy", "words today")}</div>
    </div>
    <div class="tile">
      <div class="tv num">{number(stats.week_words)}</div>
      <div class="tl">{tr("esta semana", "this week")}</div>
    </div>
    <div class="tile">
      <div class="tv num">{minutes(stats.minutes_saved)}</div>
      <div class="tl">{tr("tiempo ahorrado", "time saved")}</div>
    </div>
    <div class="tile">
      <div class="tv num">{stats.streak_days} <span class="unit">{stats.streak_days === 1 ? tr("día", "day") : tr("días", "days")}</span></div>
      <div class="tl">{tr("racha", "streak")}</div>
    </div>
  </section>

  <section class="chart-card">
    <header>
      <div>
        <h3>{tr("Últimos 30 días", "Last 30 days")}</h3>
        <p class="faint">
          {number(stats.total_words)} {tr("palabras en total", "words in total")} · {Math.round(stats.speaking_wpm)} {tr("palabras/min al hablar", "words/min speaking")}
        </p>
      </div>
      {#if hover !== null}
        <div class="tip num">
          <strong>{number(stats.last_30_days[hover].words)}</strong>
          {tr("palabras", "words")} · {new Date(stats.last_30_days[hover].date).toLocaleDateString(app.lang === "en" ? "en-US" : "es-ES", { day: "numeric", month: "short" })}
        </div>
      {/if}
    </header>
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="chart" onmouseleave={() => (hover = null)}>
      {#each stats.last_30_days as d, i}
        <!-- svelte-ignore a11y_mouse_events_have_key_events -->
        <div class="col" onmouseover={() => (hover = i)}>
          <div class="bar" class:hl={hover === i} class:zero={d.words === 0} style:height="{Math.max(3, (d.words / maxWords) * 100)}%"></div>
        </div>
      {/each}
    </div>
  </section>
{/if}

<section class="recent">
  <header>
    <h3>{tr("Recientes", "Recent")}</h3>
    <Button size="sm" variant="ghost" iconRight="chevron-right" onclick={() => router.go("history")}>{tr("Ver historial", "View history")}</Button>
  </header>
  {#if recent.length === 0}
    <p class="faint empty">{tr("Tus dictados aparecerán aquí.", "Your dictations will appear here.")}</p>
  {:else}
    <ul>
      {#each recent as e (e.id)}
        <li>
          <div class="txt">{e.final_text}</div>
          <div class="meta faint">
            {relativeDate(e.created_at)}{#if e.app} · {appName(e.app)}{/if}
          </div>
          <Button size="sm" variant="ghost" icon="copy" title={tr("Copiar", "Copy")} onclick={() => copy(e)} />
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .hero-head {
    margin-bottom: 24px;
  }
  .hero-head p {
    margin-top: 6px;
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }
  .inline {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .callout {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 14px;
    text-align: left;
    padding: 14px 16px;
    margin-bottom: 14px;
    border-radius: var(--radius);
    border: 1px solid var(--border-strong);
    background: var(--surface-2);
    cursor: pointer;
    transition: background var(--fast) var(--ease);
  }
  .callout:hover:not(.static) {
    background: var(--active);
  }
  .callout.static {
    cursor: default;
  }
  .callout > div {
    flex: 1;
  }
  .ct {
    font-weight: 560;
  }
  .cd {
    font-size: var(--text-sm);
    color: var(--text-2);
    margin-top: 2px;
  }
  .hero {
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface-2);
    padding: 20px;
    margin-bottom: 18px;
  }
  .hero-main {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }
  .state {
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .ring {
    width: 44px;
    height: 44px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    border: 1px solid var(--border-strong);
    background: var(--surface);
    transition: all var(--normal) var(--ease);
  }
  .ring.rec {
    background: var(--text);
    color: var(--bg);
    border-color: var(--text);
    animation: breathe 1.6s ease-in-out infinite;
  }
  .ring.proc {
    border-style: dashed;
    animation: spin 2.4s linear infinite;
  }
  @keyframes breathe {
    50% {
      box-shadow: 0 0 0 7px var(--active);
    }
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .st {
    font-size: var(--text-lg);
    font-weight: 600;
    letter-spacing: -0.01em;
  }
  .sd {
    font-size: var(--text-sm);
    margin-top: 1px;
  }
  .try {
    margin-top: 16px;
  }
  .tiles {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 10px;
    margin-bottom: 18px;
  }
  .tile {
    padding: 16px 18px;
    border-radius: var(--radius);
    border: 1px solid var(--border);
    background: var(--surface);
  }
  .tv {
    font-size: 24px;
    font-weight: 600;
    letter-spacing: -0.02em;
  }
  .unit {
    font-size: var(--text-md);
    font-weight: 500;
    color: var(--text-2);
  }
  .tl {
    font-size: var(--text-sm);
    color: var(--text-2);
    margin-top: 2px;
  }
  .chart-card {
    padding: 18px 20px 16px;
    border-radius: var(--radius);
    border: 1px solid var(--border);
    margin-bottom: 26px;
  }
  .chart-card header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 16px;
  }
  .chart-card p {
    font-size: var(--text-sm);
    margin-top: 2px;
  }
  .tip {
    font-size: var(--text-sm);
    color: var(--text-2);
  }
  .tip strong {
    color: var(--text);
  }
  .chart {
    height: 96px;
    display: flex;
    align-items: flex-end;
    gap: 4px;
  }
  .col {
    flex: 1;
    height: 100%;
    display: flex;
    align-items: flex-end;
    cursor: default;
  }
  .bar {
    width: 100%;
    border-radius: 3px;
    background: var(--text-3);
    transition:
      background var(--fast) var(--ease),
      height 300ms var(--ease);
  }
  .bar.zero {
    background: var(--surface-3);
  }
  .bar.hl,
  .col:last-child .bar:not(.zero) {
    background: var(--text);
  }
  .recent header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 8px;
  }
  .recent h3 {
    font-size: var(--text-sm);
    color: var(--text-2);
  }
  .empty {
    font-size: var(--text-sm);
    padding: 12px 0;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  li {
    display: grid;
    grid-template-columns: 1fr auto auto;
    align-items: center;
    gap: 14px;
    padding: 11px 10px 11px 16px;
  }
  li + li {
    border-top: 1px solid var(--border);
  }
  .txt {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: var(--text-md);
  }
  .meta {
    font-size: var(--text-xs);
    white-space: nowrap;
  }
</style>
