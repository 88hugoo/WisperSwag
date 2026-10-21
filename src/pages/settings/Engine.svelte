<script lang="ts">
  import { onMount } from "svelte";
  import { api, errorText, on, openDialog } from "../../lib/api";
  import { bytes } from "../../lib/format";
  import { app, tr } from "../../lib/store.svelte";
  import type { CatalogItem, CloudSttProvider, DownloadProgress, InstalledModel, ModelsView } from "../../lib/types";
  import Badge from "../../components/Badge.svelte";
  import Button from "../../components/Button.svelte";
  import Dialog from "../../components/Dialog.svelte";
  import Icon from "../../components/Icon.svelte";
  import Input from "../../components/Input.svelte";
  import Progress from "../../components/Progress.svelte";
  import Row from "../../components/Row.svelte";
  import Section from "../../components/Section.svelte";
  import Segmented from "../../components/Segmented.svelte";
  import Select from "../../components/Select.svelte";
  import Toggle from "../../components/Toggle.svelte";

  const e = $derived(app.settings.engine);
  let view = $state<ModelsView | null>(null);
  let progress = $state<Record<string, DownloadProgress>>({});
  let secrets = $state<Record<string, string>>({});
  let keyDraft = $state("");
  let testing = $state(false);
  let testResult = $state<{ ok: boolean; text: string } | null>(null);
  let loadingModel = $state(false);
  let importOpen = $state(false);
  let importFolder = $state("");
  let importName = $state("");
  let importing = $state(false);
  let confirmDelete = $state<{ id: string; name: string } | null>(null);

  const descEn: Record<string, string> = {
    "parakeet-tdt-0.6b-v3": "Very accurate and fast on CPU. 25 European languages (Spanish included) with automatic detection, punctuation and capitalization. Doesn't invent text during silences.",
    "whisper-turbo": "Whisper large-v3-turbo: 99 languages, very good accuracy. Heavier and slower on CPU than Parakeet.",
    "whisper-small": "Balance between size and accuracy for Whisper's 99 languages.",
    "whisper-base": "Lightweight. Limited accuracy; useful on modest computers.",
  };

  const providers: { value: CloudSttProvider; label: string; url: string; model: string; key: string }[] = [
    { value: "open_ai", label: "OpenAI", url: "https://api.openai.com/v1", model: "gpt-4o-transcribe", key: "stt:openai" },
    { value: "groq", label: "Groq", url: "https://api.groq.com/openai/v1", model: "whisper-large-v3-turbo", key: "stt:groq" },
    { value: "custom", label: tr("Servidor propio", "Own server"), url: "http://localhost:8000/v1", model: "whisper-1", key: "stt:custom" },
  ];
  const provider = $derived(providers.find((p) => p.value === e.cloud.provider) ?? providers[0]);
  const savedKey = $derived(secrets[provider.key]);

  async function refresh() {
    try {
      view = await api.listModels();
    } catch (err) {
      app.toast(errorText(err), "error");
    }
  }

  async function loadSecrets() {
    secrets = await api.secretStatus();
  }

  onMount(() => {
    refresh();
    loadSecrets();
    const off = on("model-progress", (p: DownloadProgress) => {
      progress = { ...progress, [p.model_id]: p };
      if (p.phase === "done" || p.phase === "failed" || p.phase === "cancelled") refresh();
    });
    const timer = setInterval(() => {
      // Refleja la carga/descarga automática del modelo en memoria.
      if (!view?.downloading.length) api.listModels().then((v) => view && (view.loaded = v.loaded));
    }, 5000);
    return () => {
      off.then((f) => f());
      clearInterval(timer);
    };
  });

  const installedIds = $derived(new Set(view?.installed.map((m) => m.id) ?? []));
  const active = $derived(view?.installed.find((m) => m.id === e.local_model) ?? view?.installed[0] ?? null);
  const activeInfo = $derived(view?.catalog.find((c) => c.id === active?.id) ?? null);
  const customModels = $derived(view?.installed.filter((m) => m.custom) ?? []);
  const downloading = (id: string) => !!view?.downloading.includes(id) || (progress[id] && ["downloading", "extracting"].includes(progress[id].phase));

  const languageOptions = $derived((app.boot?.languages ?? [["auto", "Auto"], ["es", "Español"]]).map(([value, label]) => ({ value, label })));
  const languageUnsupported = $derived(
    e.kind === "local" && !!activeInfo?.languages && e.language !== "auto" && !activeInfo.languages.includes(e.language),
  );

  const cores = typeof navigator !== "undefined" ? navigator.hardwareConcurrency || 8 : 8;
  const threadOptions = $derived([
    { value: 0, label: `${tr("Automático", "Automatic")} (${view?.recommended_threads ?? 4})` },
    ...[1, 2, 3, 4, 6, 8, 12, 16].filter((n) => n <= cores).map((n) => ({ value: n, label: String(n) })),
  ]);

  async function download(m: CatalogItem) {
    progress = { ...progress, [m.id]: { model_id: m.id, phase: "downloading", done_bytes: 0, total_bytes: m.download_mb * 1_000_000, message: null } };
    try {
      await api.downloadModel(m.id);
      app.toast(tr(`${m.name} instalado`, `${m.name} installed`), "success");
    } catch (err) {
      const msg = errorText(err);
      if (!/cancel/i.test(msg)) app.toast(msg, "error", 6000);
    } finally {
      const { [m.id]: _, ...rest } = progress;
      progress = rest;
      refresh();
    }
  }

  function use(id: string) {
    app.settings.engine.local_model = id;
    app.settings.engine.kind = "local";
  }

  async function remove(id: string) {
    try {
      await api.deleteModel(id);
      confirmDelete = null;
      await refresh();
    } catch (err) {
      app.toast(errorText(err), "error");
    }
  }

  async function pickFolder() {
    const f = await openDialog({ directory: true });
    if (f) {
      importFolder = f;
      if (!importName) importName = f.split(/[\\/]/).filter(Boolean).pop() ?? "";
    }
  }

  async function doImport() {
    importing = true;
    try {
      const m: InstalledModel = await api.importModel(importFolder, importName.trim() || "Modelo importado");
      importOpen = false;
      importFolder = importName = "";
      app.settings.engine.local_model = m.id;
      app.toast(tr("Modelo importado", "Model imported"), "success");
      await refresh();
    } catch (err) {
      app.toast(errorText(err), "error", 7000);
    } finally {
      importing = false;
    }
  }

  async function loadNow() {
    loadingModel = true;
    try {
      await app.saveNow();
      const ms = await api.loadModelNow();
      app.toast(tr(`Modelo cargado en ${(ms / 1000).toFixed(1)} s`, `Model loaded in ${(ms / 1000).toFixed(1)} s`), "success");
    } catch (err) {
      app.toast(errorText(err), "error");
    } finally {
      loadingModel = false;
      refresh();
    }
  }

  async function unloadNow() {
    await api.unloadModel();
    refresh();
  }

  async function saveKey() {
    try {
      await api.setSecret(provider.key, keyDraft);
      keyDraft = "";
      await loadSecrets();
      app.toast(tr("Clave guardada de forma segura", "Key stored securely"), "success");
    } catch (err) {
      app.toast(errorText(err), "error");
    }
  }

  async function deleteKey() {
    await api.deleteSecret(provider.key);
    await loadSecrets();
  }

  async function testCloud() {
    testing = true;
    testResult = null;
    try {
      await app.saveNow();
      const r = await api.testCloudStt();
      testResult = { ok: true, text: r || tr("Conexión correcta", "Connection OK") };
    } catch (err) {
      testResult = { ok: false, text: errorText(err) };
    } finally {
      testing = false;
    }
  }

  function dots(n: number) {
    return "●".repeat(n) + "○".repeat(Math.max(0, 5 - n));
  }

  function pct(p: DownloadProgress | undefined) {
    if (!p || !p.total_bytes) return 0;
    return Math.min(1, p.done_bytes / p.total_bytes);
  }
</script>

<h2 class="h">{tr("Motor de voz", "Speech engine")}</h2>

<Section>
  <Row title={tr("Reconocimiento de voz", "Speech recognition")} description={e.kind === "local"
    ? tr("En tu equipo: el audio nunca sale del ordenador y funciona sin conexión.", "On your computer: audio never leaves it and works offline.")
    : tr(`En la nube: el audio de cada dictado se envía a ${provider.label}.`, `In the cloud: each dictation's audio is sent to ${provider.label}.`)}>
    <Segmented bind:value={app.settings.engine.kind} options={[
      { value: "local", label: tr("Local", "Local"), icon: "lock" },
      { value: "cloud", label: tr("Nube", "Cloud"), icon: "globe" },
    ]} />
  </Row>
  <Row title={tr("Idioma del dictado", "Dictation language")} description={languageUnsupported
    ? tr(`${activeInfo?.name} no admite este idioma. Elige un modelo Whisper o la detección automática.`, `${activeInfo?.name} doesn't support this language. Choose a Whisper model or auto-detect.`)
    : tr("Con «Detección automática» puedes alternar idiomas entre dictados.", "With “Auto-detect” you can switch languages between dictations.")}>
    {#if languageUnsupported}<span class="warn"><Icon name="alert" size={15} /></span>{/if}
    <Select bind:value={app.settings.engine.language} options={languageOptions} width="220px" />
  </Row>
</Section>

<Section title={tr("Modelos locales", "Local models")} description={tr("Se descargan una vez desde GitHub (sherpa-onnx) y se verifican con SHA-256.", "Downloaded once from GitHub (sherpa-onnx) and verified with SHA-256.")}>
  {#snippet actions()}
    <Button size="sm" variant="ghost" icon="folder" onclick={() => api.openFolder("models")}>{tr("Carpeta", "Folder")}</Button>
    <Button size="sm" variant="ghost" icon="upload" onclick={() => (importOpen = true)}>{tr("Importar", "Import")}</Button>
  {/snippet}
  {#if !view}
    <div class="loading"><Progress indeterminate /></div>
  {:else}
    <div class="models">
      {#each view.catalog as m (m.id)}
        {@const installed = installedIds.has(m.id)}
        {@const isActive = installed && active?.id === m.id}
        {@const p = progress[m.id]}
        <div class="model" class:active={isActive && e.kind === "local"}>
          <div class="top">
            <div class="name">
              <strong>{m.name.replace(" (recomendado)", "")}</strong>
              {#if m.recommended}<Badge>{tr("Recomendado", "Recommended")}</Badge>{/if}
              {#if isActive}<Badge solid icon="check">{tr("En uso", "In use")}</Badge>{/if}
              {#if view.loaded === m.id}<Badge icon="cpu">{tr("En memoria", "In memory")}</Badge>{/if}
            </div>
            <div class="acts">
              {#if downloading(m.id)}
                <Button size="sm" variant="ghost" icon="x" onclick={() => api.cancelDownload(m.id)}>{tr("Cancelar", "Cancel")}</Button>
              {:else if installed}
                {#if !isActive}<Button size="sm" variant="primary" onclick={() => use(m.id)}>{tr("Usar", "Use")}</Button>{/if}
                <Button size="sm" variant="ghost" icon="trash" title={tr("Eliminar", "Delete")} onclick={() => (confirmDelete = { id: m.id, name: m.name })} />
              {:else}
                <Button size="sm" variant={m.recommended ? "primary" : "secondary"} icon="download" onclick={() => download(m)}>{tr("Descargar", "Download")} · {m.download_mb} MB</Button>
              {/if}
            </div>
          </div>
          <p class="desc">{app.lang === "en" ? (descEn[m.id] ?? m.description) : m.description}</p>
          {#if downloading(m.id)}
            <div class="prog">
              <div class="bar"><Progress value={pct(p)} indeterminate={!p || p.phase === "extracting"} /></div>
              <span class="faint num">
                {#if p?.phase === "extracting"}{tr("Extrayendo y verificando…", "Extracting and verifying…")}
                {:else if p}{bytes(p.done_bytes)} / {bytes(p.total_bytes)}{:else}{tr("Preparando…", "Preparing…")}{/if}
              </span>
            </div>
          {:else}
            <div class="meta faint">
              <span title={tr("Velocidad en CPU", "CPU speed")}>{tr("Velocidad", "Speed")} <span class="dots">{dots(m.speed)}</span></span>
              <span title={tr("Precisión en español", "Spanish accuracy")}>{tr("Precisión (es)", "Accuracy (es)")} <span class="dots">{dots(m.accuracy_es)}</span></span>
              <span>~{m.ram_mb >= 1000 ? `${(m.ram_mb / 1000).toFixed(1)} GB` : `${m.ram_mb} MB`} {tr("de RAM al usarlo", "RAM while in use")}</span>
              <span>{m.languages ? `${m.languages.length} ${tr("idiomas", "languages")}` : tr("99 idiomas", "99 languages")}</span>
              {#if installed}<span>{m.disk_mb} MB {tr("en disco", "on disk")}</span>{/if}
            </div>
          {/if}
        </div>
      {/each}
      {#each customModels as m (m.id)}
        <div class="model" class:active={active?.id === m.id && e.kind === "local"}>
          <div class="top">
            <div class="name">
              <strong>{m.name}</strong>
              <Badge>{tr("Importado", "Imported")} · {m.family === "parakeet" ? "Transducer" : "Whisper"}</Badge>
              {#if active?.id === m.id}<Badge solid icon="check">{tr("En uso", "In use")}</Badge>{/if}
            </div>
            <div class="acts">
              {#if active?.id !== m.id}<Button size="sm" variant="primary" onclick={() => use(m.id)}>{tr("Usar", "Use")}</Button>{/if}
              <Button size="sm" variant="ghost" icon="trash" title={tr("Eliminar", "Delete")} onclick={() => (confirmDelete = { id: m.id, name: m.name })} />
            </div>
          </div>
          <p class="desc mono faint">{m.dir} · {bytes(m.size_bytes)}</p>
        </div>
      {/each}
    </div>
  {/if}
</Section>

<Section title={tr("Memoria y velocidad", "Memory & speed")}>
  <Row title={tr("Modelo en memoria", "Model in memory")} description={view?.loaded
    ? tr("Cargado: el próximo dictado empieza sin espera.", "Loaded: the next dictation starts instantly.")
    : tr("No cargado: se carga mientras hablas (unos segundos la primera vez).", "Not loaded: it loads while you speak (a few seconds the first time).")}>
    {#if view?.loaded}
      <Button variant="ghost" icon="power" onclick={unloadNow}>{tr("Liberar memoria", "Free memory")}</Button>
    {:else}
      <Button icon="zap" loading={loadingModel} disabled={!active} onclick={loadNow}>{tr("Cargar ahora", "Load now")}</Button>
    {/if}
  </Row>
  <Row title={tr("Mantener cargado tras dictar", "Keep loaded after dictating")} description={tr("Después se libera la memoria (cientos de MB) hasta el siguiente dictado.", "Afterwards the memory (hundreds of MB) is freed until the next dictation.")}>
    <Select bind:value={app.settings.engine.keep_loaded_minutes} width="190px" options={[
      { value: 0, label: tr("Liberar al terminar", "Free when done") },
      { value: 1, label: "1 min" },
      { value: 5, label: "5 min" },
      { value: 10, label: tr("10 min (recomendado)", "10 min (recommended)") },
      { value: 30, label: "30 min" },
      { value: 60, label: "1 h" },
      { value: 1440, label: tr("Siempre cargado", "Always loaded") },
    ]} />
  </Row>
  <Row title={tr("Cargar al iniciar Windows", "Load on startup")} description={tr("El primer dictado es instantáneo a cambio de usar memoria desde el arranque.", "The first dictation is instant, at the cost of using memory from startup.")}>
    <Toggle bind:checked={app.settings.engine.preload_on_start} />
  </Row>
  <Row title={tr("Hilos de CPU", "CPU threads")} description={app.settings.performance.low_power
    ? tr("El modo de bajo consumo limita el motor a 2 hilos.", "Low power mode limits the engine to 2 threads.")
    : tr("Más hilos transcriben antes pero consumen más mientras procesa.", "More threads transcribe faster but use more CPU while processing.")}>
    <Select bind:value={app.settings.engine.threads} options={threadOptions} width="170px" />
  </Row>
</Section>

<Section title={tr("Transcripción en la nube", "Cloud transcription")} description={tr("Opcional. Solo se usa si eliges «Nube» arriba.", "Optional. Only used if you choose “Cloud” above.")}>
  <Row title={tr("Proveedor", "Provider")} description={tr("El audio se envía con tu clave directamente al proveedor; Wispr Swag no tiene servidores.", "Audio goes with your key straight to the provider; Wispr Swag has no servers.")}>
    <Segmented bind:value={app.settings.engine.cloud.provider} options={providers.map((p) => ({ value: p.value, label: p.label }))} onchange={() => (testResult = null)} />
  </Row>
  <Row title={tr("Clave de API", "API key")} description={savedKey
    ? `${tr("Guardada en", "Stored in")} ${app.boot?.secrets_backend ?? ""}: ${savedKey}`
    : e.cloud.provider === "custom" ? tr("Opcional para servidores propios.", "Optional for own servers.") : tr("Se guarda cifrada en el Administrador de credenciales de Windows, nunca en los ajustes.", "Stored encrypted in Windows Credential Manager, never in the settings file.")}>
    <Input bind:value={keyDraft} type="password" placeholder={savedKey ? tr("Reemplazar clave", "Replace key") : "sk-…"} width="220px" mono onkeydown={(ev) => ev.key === "Enter" && keyDraft.trim() && saveKey()} />
    <Button disabled={!keyDraft.trim()} onclick={saveKey}>{tr("Guardar", "Save")}</Button>
    {#if savedKey}<Button variant="ghost" icon="trash" title={tr("Borrar clave", "Delete key")} onclick={deleteKey} />{/if}
  </Row>
  <Row title={tr("Modelo", "Model")} description={tr("Vacío = predeterminado del proveedor.", "Empty = provider default.")}>
    <Input bind:value={app.settings.engine.cloud.model} placeholder={provider.model} width="260px" mono />
  </Row>
  <Row title={tr("Dirección del servicio", "Service URL")} description={tr("Cualquier servidor compatible con /audio/transcriptions de OpenAI.", "Any server compatible with OpenAI's /audio/transcriptions.")}>
    <Input bind:value={app.settings.engine.cloud.base_url} placeholder={provider.url} width="300px" mono />
  </Row>
  <Row title={tr("Enviar el vocabulario como pista", "Send vocabulary as a hint")} description={tr("Mejora la ortografía de nombres propios. Se envían los términos de tu vocabulario, no el historial.", "Improves spelling of names. Your vocabulary terms are sent, not your history.")}>
    <Toggle bind:checked={app.settings.engine.cloud.send_vocabulary_hint} />
  </Row>
  <Row title={tr("Usar el motor local si la nube falla", "Use the local engine if the cloud fails")} description={tr("Sin conexión, con errores del servicio o sin clave, transcribe en el equipo (requiere un modelo local).", "Offline, on service errors or without a key, transcribes on your computer (requires a local model).")}>
    <Toggle bind:checked={app.settings.engine.fallback_to_local} />
  </Row>
  <Row title={tr("Tiempo de espera", "Timeout")}>
    <Select bind:value={app.settings.engine.cloud.timeout_secs} width="120px" options={[10, 20, 30, 60, 120].map((v) => ({ value: v, label: `${v} s` }))} />
  </Row>
  <Row title={tr("Probar conexión", "Test connection")} description={testResult?.text ?? tr("Envía un segundo de silencio al servicio para comprobar la clave y la dirección.", "Sends one second of silence to check the key and URL.")}>
    {#if testResult}<span class:ok={testResult.ok} class:bad={!testResult.ok} class="res"><Icon name={testResult.ok ? "check" : "alert"} size={15} /></span>{/if}
    <Button icon="refresh" loading={testing} onclick={testCloud}>{tr("Probar", "Test")}</Button>
  </Row>
</Section>

<p class="note faint">
  <Icon name="info" size={13} />
  {tr(
    "Vocabulario: los modelos locales no admiten listas de palabras en el reconocimiento; Wispr Swag corrige después los términos parecidos (también por sonido). En la nube, además, se envían como pista. Ningún ajuste reentrena el modelo acústico.",
    "Vocabulary: local models don't accept word lists during recognition; Wispr Swag corrects similar-sounding terms afterwards. In the cloud they're also sent as a hint. No setting retrains the acoustic model.",
  )}
</p>

<Dialog bind:open={importOpen} title={tr("Importar un modelo", "Import a model")} width="520px">
  <p class="dlg-text">
    {tr(
      "Carpeta con un modelo de sherpa-onnx: transducer (encoder, decoder, joiner y tokens.txt, p. ej. Parakeet) o Whisper (encoder, decoder y tokens.txt). Se copia a la carpeta de modelos.",
      "Folder with a sherpa-onnx model: transducer (encoder, decoder, joiner and tokens.txt, e.g. Parakeet) or Whisper (encoder, decoder and tokens.txt). It's copied to the models folder.",
    )}
  </p>
  <div class="dlg-grid">
    <span>{tr("Carpeta", "Folder")}</span>
    <div class="with-btn">
      <Input bind:value={importFolder} mono width="100%" placeholder="C:\modelos\…" />
      <Button icon="folder" onclick={pickFolder}>{tr("Elegir", "Browse")}</Button>
    </div>
    <span>{tr("Nombre", "Name")}</span>
    <Input bind:value={importName} width="100%" placeholder={tr("Mi modelo", "My model")} />
  </div>
  {#snippet actions()}
    <Button variant="ghost" onclick={() => (importOpen = false)}>{tr("Cancelar", "Cancel")}</Button>
    <Button variant="primary" loading={importing} disabled={!importFolder.trim()} onclick={doImport}>{tr("Importar", "Import")}</Button>
  {/snippet}
</Dialog>

<Dialog open={!!confirmDelete} title={tr("Eliminar modelo", "Delete model")} onclose={() => (confirmDelete = null)}>
  <p class="dlg-text">
    {tr(`Se borrará «${confirmDelete?.name}» del disco. Podrás volver a descargarlo.`, `“${confirmDelete?.name}” will be deleted from disk. You can download it again.`)}
  </p>
  {#snippet actions()}
    <Button variant="ghost" onclick={() => (confirmDelete = null)}>{tr("Cancelar", "Cancel")}</Button>
    <Button variant="danger" icon="trash" onclick={() => confirmDelete && remove(confirmDelete.id)}>{tr("Eliminar", "Delete")}</Button>
  {/snippet}
</Dialog>

<style>
  .h {
    margin-bottom: 20px;
  }
  .warn {
    color: var(--text);
    display: inline-flex;
  }
  .loading {
    padding: 22px 18px;
  }
  .models {
    display: flex;
    flex-direction: column;
  }
  .model {
    padding: 14px 18px;
    border-bottom: 1px solid var(--border);
    position: relative;
  }
  .model:last-child {
    border-bottom: none;
  }
  .model.active::before {
    content: "";
    position: absolute;
    left: 0;
    top: 14px;
    bottom: 14px;
    width: 2px;
    border-radius: 2px;
    background: var(--text);
  }
  .top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .name {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    font-size: var(--text-md);
  }
  .acts {
    display: flex;
    gap: 6px;
    flex-shrink: 0;
  }
  .desc {
    margin: 6px 0 0;
    font-size: var(--text-sm);
    color: var(--text-2);
    max-width: 620px;
    line-height: 1.5;
  }
  .desc.mono {
    font-size: var(--text-xs);
    word-break: break-all;
  }
  .meta {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 18px;
    margin-top: 8px;
    font-size: var(--text-xs);
  }
  .dots {
    letter-spacing: 1px;
    color: var(--text-2);
    font-size: 9px;
    vertical-align: 1px;
  }
  .prog {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-top: 10px;
  }
  .prog .bar {
    flex: 1;
  }
  .prog span {
    font-size: var(--text-xs);
    white-space: nowrap;
  }
  .res {
    display: inline-flex;
  }
  .res.bad {
    color: var(--text-2);
  }
  .note {
    display: flex;
    gap: 8px;
    align-items: flex-start;
    font-size: var(--text-xs);
    line-height: 1.55;
    margin: -6px 2px 8px;
    max-width: 700px;
  }
  .note :global(svg) {
    flex-shrink: 0;
    margin-top: 2px;
  }
  .dlg-text {
    margin: 0 0 14px;
    font-size: var(--text-sm);
    color: var(--text-2);
    line-height: 1.55;
  }
  .dlg-grid {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 10px 14px;
    align-items: center;
    font-size: var(--text-sm);
  }
  .with-btn {
    display: flex;
    gap: 6px;
  }
</style>
