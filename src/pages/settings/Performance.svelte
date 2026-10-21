<script lang="ts">
  import { onMount } from "svelte";
  import { api, errorText, openDialog, saveDialog } from "../../lib/api";
  import { bytes, duration } from "../../lib/format";
  import { app, tr } from "../../lib/store.svelte";
  import type { Diagnostics } from "../../lib/types";
  import Button from "../../components/Button.svelte";
  import Dialog from "../../components/Dialog.svelte";
  import Row from "../../components/Row.svelte";
  import Section from "../../components/Section.svelte";
  import Select from "../../components/Select.svelte";
  import Toggle from "../../components/Toggle.svelte";

  const pf = $derived(app.settings.performance);
  let diag = $state<Diagnostics | null>(null);
  let measuring = $state(false);
  let resetOpen = $state(false);

  onMount(measure);

  async function measure() {
    measuring = true;
    try {
      diag = await api.diagnostics();
    } catch (e) {
      app.toast(errorText(e), "error");
    } finally {
      measuring = false;
    }
  }

  function report(d: Diagnostics): string {
    return [
      `Wispr Swag ${d.version} · ${d.os}`,
      `Tiempo activo: ${duration(d.uptime_secs * 1000)}`,
      `Memoria (conjunto de trabajo): ${bytes(d.process.working_set)}`,
      `Memoria privada: ${bytes(d.process.private_bytes)}`,
      `CPU (0,5 s): ${d.cpu_percent.toFixed(1)} %`,
      `Hilos del proceso: ${d.process.threads}`,
      `Motor: ${d.engine_kind} · modelo cargado: ${d.engine.loaded_model ?? "ninguno"}${d.engine.load_ms ? ` (${d.engine.load_ms} ms)` : ""}`,
      `Modelo instalado: ${d.model_installed ? "sí" : "no"}`,
      `Atajos activos: ${d.hotkeys_running ? "sí" : "no"}${d.hotkey_error ? ` (${d.hotkey_error})` : ""}`,
      `Administrador: ${d.elevated ? "sí" : "no"}`,
      `Micrófonos: ${d.devices}`,
      `Núcleos lógicos: ${d.threads_available}`,
      `Conexiones activas: ${d.network_features.length ? d.network_features.join("; ") : "ninguna"}`,
    ].join("\n");
  }

  async function copyReport() {
    if (!diag) return;
    try {
      await navigator.clipboard.writeText(report(diag));
      app.toast(tr("Informe copiado (sin datos personales)", "Report copied (no personal data)"), "success");
    } catch (e) {
      app.toast(errorText(e), "error");
    }
  }

  async function exportSettings() {
    const path = await saveDialog("wispr-swag-ajustes.json", [{ name: "JSON", extensions: ["json"] }]);
    if (!path) return;
    try {
      await app.saveNow();
      await api.settingsExport(path);
      app.toast(tr("Ajustes exportados. No incluyen claves de API.", "Settings exported. API keys aren't included."), "success", 4500);
    } catch (e) {
      app.toast(errorText(e), "error");
    }
  }

  async function importSettings() {
    const path = await openDialog({ filters: [{ name: "JSON", extensions: ["json"] }] });
    if (!path) return;
    try {
      await api.settingsImport(path);
      await app.reload();
      app.toast(tr("Ajustes importados", "Settings imported"), "success");
    } catch (e) {
      app.toast(errorText(e), "error", 6000);
    }
  }

  async function backupNow() {
    try {
      await app.saveNow();
      await api.settingsBackupNow();
      app.toast(tr("Copia de seguridad creada", "Backup created"), "success");
    } catch (e) {
      app.toast(errorText(e), "error");
    }
  }

  async function reset() {
    try {
      await api.settingsReset();
      await app.reload();
      resetOpen = false;
      app.toast(tr("Ajustes restablecidos (se guardó una copia antes)", "Settings reset (a backup was saved first)"), "success", 4500);
    } catch (e) {
      app.toast(errorText(e), "error");
    }
  }
</script>

<h2 class="h">{tr("Rendimiento", "Performance")}</h2>

<Section title={tr("Transcripción", "Transcription")}>
  <Row title={tr("Transcribir mientras hablas", "Transcribe while you speak")} description={tr("Procesa cada frase en cuanto haces una pausa, así el texto llega casi al instante al terminar. Solo con el motor local.", "Processes each sentence as soon as you pause, so text arrives almost instantly when you finish. Local engine only.")}>
    <Toggle bind:checked={app.settings.performance.incremental_transcription} />
  </Row>
  <Row title={tr("Pausa que separa frases", "Pause between sentences")} disabled={!pf.incremental_transcription} description={tr("Más larga = frases más completas y más contexto para el modelo.", "Longer = more complete sentences and more context for the model.")}>
    <Select bind:value={app.settings.performance.segment_pause_ms} width="140px" options={[600, 900, 1200, 1600].map((v) => ({ value: v, label: `${(v / 1000).toFixed(1).replace(".", app.lang === "es" ? "," : ".")} s` }))} />
  </Row>
</Section>

<Section title={tr("Consumo", "Power use")}>
  <Row title={tr("Modo de bajo consumo", "Low power mode")} description={tr("Para portátiles con batería: el motor usa 2 hilos, la barra se dibuja a 20 fps y sin animaciones.", "For laptops on battery: the engine uses 2 threads and the bar renders at 20 fps without animations.")}>
    <Toggle bind:checked={app.settings.performance.low_power} />
  </Row>
  <Row title={tr("Fluidez de la barra", "Bar smoothness")} disabled={pf.low_power} description={tr("Solo se dibuja mientras cambia; en reposo no consume CPU.", "It only redraws while changing; idle it uses no CPU.")}>
    <Select bind:value={app.settings.performance.pill_fps} width="140px" options={[
      { value: 20, label: "20 fps" },
      { value: 30, label: "30 fps" },
      { value: 60, label: "60 fps" },
    ]} />
  </Row>
  <Row title={tr("Tu velocidad al teclear", "Your typing speed")} description={tr("Para calcular el tiempo ahorrado en las estadísticas.", "Used to estimate time saved in statistics.")}>
    <Select bind:value={app.settings.performance.typing_wpm} width="170px" options={[20, 30, 40, 50, 60, 80, 100].map((v) => ({ value: v, label: `${v} ${tr("palabras/min", "words/min")}` }))} />
  </Row>
</Section>

<Section title={tr("Diagnóstico", "Diagnostics")} description={tr("Medido ahora en este equipo. La ventana de ajustes (WebView2) es un proceso aparte y se cierra al salir de ella.", "Measured now on this computer. The settings window (WebView2) is a separate process that closes when you leave it.")}>
  {#snippet actions()}
    <Button size="sm" variant="ghost" icon="copy" disabled={!diag} onclick={copyReport}>{tr("Copiar informe", "Copy report")}</Button>
    <Button size="sm" variant="ghost" icon="refresh" loading={measuring} onclick={measure}>{tr("Medir", "Measure")}</Button>
  {/snippet}
  {#if diag}
    <div class="metrics">
      <div><span class="num">{bytes(diag.process.working_set)}</span><em>{tr("memoria en uso", "memory in use")}</em></div>
      <div><span class="num">{bytes(diag.process.private_bytes)}</span><em>{tr("memoria privada", "private memory")}</em></div>
      <div><span class="num">{diag.cpu_percent.toFixed(1).replace(".", app.lang === "es" ? "," : ".")} %</span><em>{tr("CPU ahora", "CPU now")}</em></div>
      <div><span class="num">{diag.process.threads}</span><em>{tr("hilos", "threads")}</em></div>
    </div>
    <dl class="facts">
      <dt>{tr("Versión", "Version")}</dt><dd>{diag.version} · {diag.os}</dd>
      <dt>{tr("Tiempo activo", "Uptime")}</dt><dd>{duration(diag.uptime_secs * 1000)}</dd>
      <dt>{tr("Motor de voz", "Speech engine")}</dt><dd>{diag.engine_kind === "local" ? tr("Local", "Local") : tr("Nube", "Cloud")} · {diag.engine.loaded_model ? `${tr("en memoria", "in memory")}: ${diag.engine.loaded_model}${diag.engine.load_ms ? ` (${tr("cargado en", "loaded in")} ${(diag.engine.load_ms / 1000).toFixed(1)} s)` : ""}` : tr("sin cargar (memoria libre)", "not loaded (memory free)")}</dd>
      <dt>{tr("Atajos globales", "Global shortcuts")}</dt><dd>{diag.hotkeys_running ? tr("Activos", "Active") : (diag.hotkey_error ?? tr("Inactivos", "Inactive"))}</dd>
      <dt>{tr("Permisos", "Privileges")}</dt><dd>{diag.elevated ? tr("Administrador: puede escribir en cualquier aplicación", "Administrator: can type into any app") : tr("Usuario estándar: en apps abiertas como administrador el texto se copia al portapapeles", "Standard user: in apps run as administrator, text is copied to the clipboard")}</dd>
      <dt>{tr("Micrófonos", "Microphones")}</dt><dd>{diag.devices}</dd>
      <dt>{tr("Núcleos lógicos", "Logical cores")}</dt><dd>{diag.threads_available}</dd>
      <dt>{tr("Conexiones", "Connections")}</dt><dd>{diag.network_features.length ? diag.network_features.join(" · ") : tr("Ninguna activa", "None active")}</dd>
    </dl>
  {/if}
  <Row title={tr("Registros", "Logs")} description={tr("Para resolver problemas. Se guardan 7 días y no incluyen audio, textos ni claves.", "For troubleshooting. Kept 7 days; they never include audio, text or keys.")}>
    <Button variant="ghost" icon="folder" onclick={() => api.openFolder("logs")}>{tr("Abrir carpeta", "Open folder")}</Button>
  </Row>
</Section>

<Section title={tr("Ajustes", "Settings")}>
  <Row title={tr("Exportar e importar", "Export & import")} description={tr("Todos los ajustes, modos, vocabulario, fragmentos y perfiles en un archivo JSON. Las claves de API no se exportan.", "All settings, modes, vocabulary, snippets and profiles in one JSON file. API keys are not exported.")}>
    <Button icon="download" onclick={exportSettings}>{tr("Exportar", "Export")}</Button>
    <Button icon="upload" onclick={importSettings}>{tr("Importar", "Import")}</Button>
  </Row>
  <Row title={tr("Copia de seguridad", "Backup")} description={tr("Se guardan las 10 más recientes.", "The 10 most recent are kept.")}>
    <Button variant="ghost" icon="folder" onclick={() => api.openFolder("backups")}>{tr("Ver copias", "View backups")}</Button>
    <Button onclick={backupNow}>{tr("Crear ahora", "Create now")}</Button>
  </Row>
  <Row title={tr("Restablecer ajustes", "Reset settings")} description={tr("Vuelve a los valores iniciales. Se conservan el historial, los modelos, el micrófono y las claves.", "Back to defaults. History, models, microphone and keys are kept.")}>
    <Button variant="ghost" onclick={() => (resetOpen = true)}>{tr("Restablecer…", "Reset…")}</Button>
  </Row>
</Section>

<Dialog bind:open={resetOpen} title={tr("Restablecer ajustes", "Reset settings")}>
  <p class="dlg-text">{tr("Se perderán los modos, el vocabulario, los fragmentos y los perfiles personalizados (antes se guarda una copia de seguridad).", "Custom modes, vocabulary, snippets and profiles will be lost (a backup is saved first).")}</p>
  {#snippet actions()}
    <Button variant="ghost" onclick={() => (resetOpen = false)}>{tr("Cancelar", "Cancel")}</Button>
    <Button variant="danger" onclick={reset}>{tr("Restablecer", "Reset")}</Button>
  {/snippet}
</Dialog>

<style>
  .h {
    margin-bottom: 20px;
  }
  .metrics {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    border-bottom: 1px solid var(--border);
  }
  .metrics div {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 14px 18px;
  }
  .metrics div + div {
    border-left: 1px solid var(--border);
  }
  .metrics .num {
    font-size: var(--text-xl);
    font-weight: 600;
    letter-spacing: -0.02em;
  }
  .metrics em {
    font-style: normal;
    font-size: var(--text-xs);
    color: var(--text-3);
  }
  .facts {
    display: grid;
    grid-template-columns: 150px 1fr;
    gap: 7px 16px;
    margin: 0;
    padding: 14px 18px;
    font-size: var(--text-sm);
    border-bottom: 1px solid var(--border);
  }
  .facts dt {
    color: var(--text-3);
  }
  .facts dd {
    margin: 0;
    color: var(--text);
    word-break: break-word;
  }
  .dlg-text {
    margin: 0;
    font-size: var(--text-sm);
    color: var(--text-2);
    line-height: 1.55;
  }
</style>
