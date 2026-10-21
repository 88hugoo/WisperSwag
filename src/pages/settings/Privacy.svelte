<script lang="ts">
  import { onMount } from "svelte";
  import { api, errorText } from "../../lib/api";
  import { bytes, number } from "../../lib/format";
  import { app, tr } from "../../lib/store.svelte";
  import type { StorageInfo } from "../../lib/types";
  import Button from "../../components/Button.svelte";
  import Dialog from "../../components/Dialog.svelte";
  import Icon from "../../components/Icon.svelte";
  import Row from "../../components/Row.svelte";
  import Section from "../../components/Section.svelte";
  import Select from "../../components/Select.svelte";
  import Toggle from "../../components/Toggle.svelte";

  const pv = $derived(app.settings.privacy);
  let storage = $state<StorageInfo | null>(null);
  let confirmOpen = $state(false);
  let deleting = $state(false);
  let selection = $state({ history: false, audio: false, stats: false, ai_usage: false, credentials: false, models: false, logs: false });

  onMount(load);

  async function load() {
    storage = await api.storageInfo();
  }

  const items = $derived([
    { key: "history" as const, label: tr("Historial de transcripciones", "Transcript history"), size: storage ? `${number(storage.history_entries)} · ${bytes(storage.history_bytes)}` : "" },
    { key: "audio" as const, label: tr("Audio guardado", "Saved audio"), size: storage ? bytes(storage.audio_bytes) : "" },
    { key: "stats" as const, label: tr("Estadísticas", "Statistics"), size: "" },
    { key: "ai_usage" as const, label: tr("Contadores de uso de la IA", "AI usage counters"), size: "" },
    { key: "credentials" as const, label: tr("Claves de API guardadas", "Saved API keys"), size: "" },
    { key: "models" as const, label: tr("Modelos de voz descargados", "Downloaded speech models"), size: storage ? bytes(storage.models_bytes) : "" },
    { key: "logs" as const, label: tr("Registros de diagnóstico", "Diagnostic logs"), size: storage ? bytes(storage.logs_bytes) : "" },
  ]);
  const anySelected = $derived(Object.values(selection).some(Boolean));

  async function doDelete() {
    deleting = true;
    try {
      await api.deleteData({ ...selection });
      confirmOpen = false;
      selection = { history: false, audio: false, stats: false, ai_usage: false, credentials: false, models: false, logs: false };
      app.toast(tr("Datos borrados", "Data deleted"), "success");
      await load();
    } catch (e) {
      app.toast(errorText(e), "error");
    } finally {
      deleting = false;
    }
  }

  const engineCloud = $derived(app.settings.engine.kind === "cloud");
  const aiOn = $derived(app.settings.ai.enabled && app.settings.ai.consent_given);
  const aiLocal = $derived(app.settings.ai.provider === "ollama" || app.settings.ai.provider === "lm_studio");
  const connections = $derived([
    {
      icon: "download",
      title: tr("Descarga de modelos de voz", "Speech model downloads"),
      text: tr("Solo cuando pulsas «Descargar». Desde GitHub (versiones de sherpa-onnx).", "Only when you click “Download”. From GitHub (sherpa-onnx releases)."),
      on: true,
      state: tr("Bajo demanda", "On demand"),
    },
    {
      icon: "wave",
      title: tr("Transcripción en la nube", "Cloud transcription"),
      text: engineCloud ? tr("El audio de cada dictado se envía al proveedor configurado.", "Each dictation's audio is sent to the configured provider.") : tr("Desactivada: el audio se procesa en tu equipo.", "Off: audio is processed on your computer."),
      on: engineCloud,
      state: engineCloud ? tr("Activa", "On") : tr("Desactivada", "Off"),
    },
    {
      icon: "sparkle",
      title: tr("Procesamiento con IA", "AI processing"),
      text: aiOn ? (aiLocal ? tr("Activa con un servidor local: el texto no sale del equipo.", "On with a local server: text stays on your computer.") : tr("El texto de los dictados que usan IA se envía al proveedor.", "Text of dictations using AI is sent to the provider.")) : tr("Desactivada.", "Off."),
      on: aiOn && !aiLocal,
      state: aiOn ? tr("Activa", "On") : tr("Desactivada", "Off"),
    },
    {
      icon: "refresh",
      title: tr("Comprobación de actualizaciones", "Update check"),
      text: tr("Consulta la última versión publicada en GitHub. No envía datos personales.", "Checks the latest release on GitHub. Sends no personal data."),
      on: app.settings.general.check_updates,
      state: app.settings.general.check_updates ? tr("Al iniciar", "On startup") : tr("Desactivada", "Off"),
    },
  ]);
</script>

<h2 class="h">{tr("Privacidad", "Privacy")}</h2>

<div class="summary">
  <Icon name="shield" size={18} />
  <div>
    <strong>{engineCloud || (aiOn && !aiLocal) ? tr("Funcionamiento mixto", "Mixed operation") : tr("Todo se procesa en tu equipo", "Everything is processed on your computer")}</strong>
    <p>{tr(
      "Sin telemetría ni analíticas. Solo se graba mientras la barra muestra que escuchas. Los datos se guardan en tu perfil de Windows y las claves, en el Administrador de credenciales.",
      "No telemetry or analytics. Recording only happens while the bar shows it's listening. Data stays in your Windows profile and keys in Credential Manager.",
    )}</p>
  </div>
</div>

<Section title={tr("Historial", "History")}>
  <Row title={tr("Guardar el historial", "Save history")} description={tr("Permite buscar, copiar y reutilizar dictados anteriores. Las estadísticas se guardan aparte, sin texto.", "Lets you search, copy and reuse past dictations. Statistics are stored separately, without text.")}>
    <Toggle bind:checked={app.settings.privacy.save_history} />
  </Row>
  <Row title={tr("Conservar durante", "Keep for")} disabled={!pv.save_history} description={tr("Los favoritos se conservan siempre, hasta que los borres.", "Favorites are always kept until you delete them.")}>
    <Select bind:value={app.settings.privacy.history_retention_days} width="150px" options={[
      { value: 1, label: tr("1 día", "1 day") },
      { value: 7, label: tr("7 días", "7 days") },
      { value: 30, label: tr("30 días", "30 days") },
      { value: 90, label: tr("90 días", "90 days") },
      { value: 365, label: tr("1 año", "1 year") },
      { value: 0, label: tr("Siempre", "Forever") },
    ]} />
  </Row>
  <Row title={tr("Guardar también el texto original", "Also save the original text")} disabled={!pv.save_history} description={tr("La transcripción antes de la limpieza y la IA, para compararla o reprocesarla.", "The transcript before cleanup and AI, to compare or reprocess it.")}>
    <Toggle bind:checked={app.settings.privacy.store_raw_text} />
  </Row>
  <Row title={tr("Guardar el nombre de la aplicación", "Save the app name")} disabled={!pv.save_history} description={tr("Solo el ejecutable (p. ej. «outlook.exe»), nunca el título de la ventana.", "Only the executable (e.g. “outlook.exe”), never the window title.")}>
    <Toggle bind:checked={app.settings.privacy.store_app_name} />
  </Row>
</Section>

<Section title={tr("Audio", "Audio")}>
  <Row title={tr("Guardar el audio de los dictados", "Save dictation audio")} disabled={!pv.save_history} description={tr("Desactivado por defecto. Si lo activas, cada dictado se guarda como WAV en tu equipo para escucharlo o reprocesarlo.", "Off by default. If enabled, each dictation is saved as a WAV file on your computer to replay or reprocess it.")}>
    <Toggle bind:checked={app.settings.privacy.save_audio} />
  </Row>
  {#if pv.save_audio && pv.save_history}
    <Row title={tr("Borrar el audio tras", "Delete audio after")}>
      <Select bind:value={app.settings.privacy.audio_retention_days} width="150px" options={[
        { value: 1, label: tr("1 día", "1 day") },
        { value: 7, label: tr("7 días", "7 days") },
        { value: 30, label: tr("30 días", "30 days") },
      ]} />
    </Row>
  {/if}
  <p class="foot faint">{tr(
    "Aunque no se guarde, si una transcripción falla el audio se conserva solo en memoria durante 10 minutos para poder reintentarla.",
    "Even when not saved, if a transcription fails the audio is kept in memory only, for 10 minutes, so you can retry it.",
  )}</p>
</Section>

<Section title={tr("Conexiones a Internet", "Internet connections")} description={tr("Estas son todas las conexiones que puede hacer Wispr Swag.", "These are all the connections Wispr Swag can make.")}>
  {#each connections as c}
    <div class="conn">
      <span class="ci" class:on={c.on}><Icon name={c.icon} size={15} /></span>
      <div class="ct"><strong>{c.title}</strong><span>{c.text}</span></div>
      <span class="cs" class:on={c.on}>{c.state}</span>
    </div>
  {/each}
</Section>

<Section title={tr("Tus datos", "Your data")} description={storage ? tr(`Ocupan ${bytes(storage.history_bytes + storage.audio_bytes + storage.models_bytes + storage.logs_bytes + storage.config_bytes)} en total.`, `${bytes(storage.history_bytes + storage.audio_bytes + storage.models_bytes + storage.logs_bytes + storage.config_bytes)} in total.`) : undefined}>
  {#snippet actions()}
    <Button size="sm" variant="ghost" icon="folder" onclick={() => api.openFolder("data")}>{tr("Abrir carpeta", "Open folder")}</Button>
  {/snippet}
  {#each items as it (it.key)}
    <label class="del">
      <input type="checkbox" bind:checked={selection[it.key]} />
      <span class="box" aria-hidden="true"><Icon name="check" size={11} stroke={2.4} /></span>
      <span class="dl">{it.label}</span>
      <span class="ds faint num">{it.size}</span>
    </label>
  {/each}
  <div class="del-actions">
    <span class="faint">{tr("Los registros nunca contienen audio, transcripciones ni claves.", "Logs never contain audio, transcripts or keys.")}</span>
    <Button variant="danger" icon="trash" disabled={!anySelected} onclick={() => (confirmOpen = true)}>{tr("Borrar seleccionados", "Delete selected")}</Button>
  </div>
</Section>

<Dialog bind:open={confirmOpen} title={tr("Borrar datos", "Delete data")}>
  <p class="dlg-text">{tr("Se borrará de forma permanente:", "This will be permanently deleted:")}</p>
  <ul class="dlg-list">
    {#each items.filter((i) => selection[i.key]) as it}<li>{it.label}</li>{/each}
  </ul>
  {#snippet actions()}
    <Button variant="ghost" onclick={() => (confirmOpen = false)}>{tr("Cancelar", "Cancel")}</Button>
    <Button variant="danger" icon="trash" loading={deleting} onclick={doDelete}>{tr("Borrar", "Delete")}</Button>
  {/snippet}
</Dialog>

<style>
  .h {
    margin-bottom: 14px;
  }
  .summary {
    display: flex;
    gap: 12px;
    align-items: flex-start;
    padding: 16px 18px;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    background: var(--surface);
    margin-bottom: 26px;
  }
  .summary :global(svg) {
    flex-shrink: 0;
    margin-top: 1px;
  }
  .summary strong {
    font-size: var(--text-md);
  }
  .summary p {
    margin: 3px 0 0;
    color: var(--text-2);
    font-size: var(--text-sm);
    line-height: 1.55;
  }
  .foot {
    margin: 0;
    padding: 10px 18px 14px;
    font-size: var(--text-xs);
    line-height: 1.5;
  }
  .conn {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 12px 18px;
    border-bottom: 1px solid var(--border);
  }
  .conn:last-child {
    border-bottom: none;
  }
  .ci {
    width: 30px;
    height: 30px;
    border-radius: 9px;
    display: grid;
    place-items: center;
    background: var(--surface-2);
    color: var(--text-3);
    flex-shrink: 0;
  }
  .ci.on {
    background: var(--surface-3);
    color: var(--text);
  }
  .ct {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: var(--text-sm);
  }
  .ct strong {
    font-weight: 540;
    font-size: var(--text-md);
  }
  .ct span {
    color: var(--text-2);
  }
  .cs {
    font-size: var(--text-xs);
    color: var(--text-3);
    white-space: nowrap;
  }
  .cs.on {
    color: var(--text);
    font-weight: 540;
  }
  .del {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 18px;
    border-bottom: 1px solid var(--border);
    cursor: pointer;
    font-size: var(--text-md);
    position: relative;
  }
  .del:hover {
    background: var(--hover);
  }
  .del input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }
  .box {
    width: 16px;
    height: 16px;
    border-radius: 5px;
    border: 1px solid var(--border-strong);
    display: grid;
    place-items: center;
    color: transparent;
    flex-shrink: 0;
    transition: background var(--fast) var(--ease);
  }
  .del input:checked + .box {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-fg);
  }
  .del input:focus-visible + .box {
    box-shadow: 0 0 0 3px var(--focus);
  }
  .dl {
    flex: 1;
  }
  .ds {
    font-size: var(--text-sm);
  }
  .del-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 12px 18px;
    font-size: var(--text-xs);
  }
  .dlg-text {
    margin: 0 0 8px;
    font-size: var(--text-sm);
    color: var(--text-2);
  }
  .dlg-list {
    margin: 0;
    padding-left: 18px;
    font-size: var(--text-sm);
    line-height: 1.7;
  }
</style>
