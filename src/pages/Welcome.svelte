<script lang="ts">
  import { onMount } from "svelte";
  import { api, errorText, on } from "../lib/api";
  import defaults from "../lib/defaults.json";
  import { bytes } from "../lib/format";
  import { router } from "../lib/router.svelte";
  import { app, tr } from "../lib/store.svelte";
  import type { DownloadProgress, InputDevice, ModelsView } from "../lib/types";
  import Button from "../components/Button.svelte";
  import HotkeyField from "../components/HotkeyField.svelte";
  import Icon from "../components/Icon.svelte";
  import Kbd from "../components/Kbd.svelte";
  import MicMeter from "../components/MicMeter.svelte";
  import Progress from "../components/Progress.svelte";
  import Segmented from "../components/Segmented.svelte";
  import Select from "../components/Select.svelte";
  import Toggle from "../components/Toggle.svelte";

  const STEPS = 5;
  let step = $state(0);

  // Micrófono
  let devices = $state<InputDevice[]>([]);
  let testing = $state(false);
  let db = $state(-90);
  let heard = $state(false);
  let micError = $state<string | null>(null);

  // Modelo
  let models = $state<ModelsView | null>(null);
  let progress = $state<DownloadProgress | null>(null);
  let downloading = $state(false);
  let dlError = $state<string | null>(null);

  // Prueba
  let tryText = $state("");
  let result = $state<string | null>(null);
  let area: HTMLTextAreaElement | undefined = $state();
  let shortcut = $state("Ctrl + Win");

  const recommended = $derived(models?.catalog.find((m) => m.recommended) ?? models?.catalog[0] ?? null);
  const installed = $derived(models?.installed[0] ?? null);
  const needsModel = $derived(app.settings.engine.kind === "local" && !installed);

  onMount(() => {
    api.listDevices().then((d) => (devices = d)).catch(() => {});
    refreshModels();
    const offs = [
      on("mic-level", (p: any) => {
        if (p.stopped) {
          testing = false;
          db = -90;
          return;
        }
        db = p.db;
        if (p.db > -42) heard = true;
        micError = p.error ?? null;
      }),
      on("model-progress", (p: DownloadProgress) => {
        progress = p;
        if (p.phase === "done") refreshModels();
      }),
      on("dictation-result", (p: { text: string }) => {
        if (step === 4) result = p.text;
      }),
    ];
    return () => {
      offs.forEach((o) => o.then((f) => f()));
      if (testing) api.micTestStop();
    };
  });

  $effect(() => {
    const v = app.settings.dictation.hotkeys.dictate;
    if (v) api.hotkeyDisplay(v).then((d) => (shortcut = d));
  });

  async function refreshModels() {
    try {
      models = await api.listModels();
      if (models.downloading.length) downloading = true;
    } catch {
      models = null;
    }
  }

  function go(n: number) {
    if (testing) {
      api.micTestStop();
      testing = false;
    }
    step = Math.max(0, Math.min(STEPS - 1, n));
    if (step === 4) setTimeout(() => area?.focus(), 80);
  }

  async function toggleTest() {
    if (testing) {
      await api.micTestStop();
      testing = false;
      return;
    }
    micError = null;
    try {
      await api.micTestStart(app.settings.audio.input_device);
      testing = true;
    } catch (e) {
      micError = errorText(e);
    }
  }

  function chooseDevice(id: string | null) {
    app.settings.audio.input_device = id;
    app.settings.audio.input_device_name = devices.find((d) => d.id === id)?.name ?? null;
    if (testing) api.micTestStop().then(() => api.micTestStart(id));
  }

  async function download() {
    if (!recommended) return;
    downloading = true;
    dlError = null;
    progress = null;
    try {
      await api.downloadModel(recommended.id);
      await refreshModels();
    } catch (e) {
      dlError = errorText(e);
    } finally {
      downloading = false;
    }
  }

  async function finish() {
    if (testing) await api.micTestStop();
    await app.saveNow();
    try {
      await api.completeOnboarding();
      await app.reload();
    } catch (e) {
      app.toast(errorText(e), "error");
    }
    router.go("");
  }

  const deviceOptions = $derived([
    { value: null, label: tr("Predeterminado del sistema", "System default") },
    ...devices.map((d) => ({ value: d.id, label: d.name })),
  ]);
  const pct = $derived(progress && progress.total_bytes ? progress.done_bytes / progress.total_bytes : 0);
</script>

<div class="welcome">
  <div class="top">
    <span></span>
    {#if step < STEPS - 1}
      <button class="skip" onclick={finish}>{tr("Omitir configuración", "Skip setup")}</button>
    {/if}
  </div>

  <div class="stage">
    {#key step}
      <div class="card">
        {#if step === 0}
          <h1>{tr("Habla. Wispr Swag escribe.", "Speak. Wispr Swag writes.")}</h1>
          <p class="lead">{tr("Dicta en cualquier aplicación de Windows con un atajo de teclado. Texto limpio, con puntuación y en el tono adecuado.", "Dictate into any Windows app with a keyboard shortcut. Clean text, punctuated, in the right tone.")}</p>
          <ul class="points">
            <li><Icon name="lock" size={16} /><div><strong>{tr("Privado por defecto", "Private by default")}</strong><span>{tr("La voz se reconoce en tu equipo, sin conexión.", "Speech is recognized on your computer, offline.")}</span></div></li>
            <li><Icon name="zap" size={16} /><div><strong>{tr("Rápido y ligero", "Fast and light")}</strong><span>{tr("En reposo apenas usa memoria ni CPU.", "Idle, it barely uses memory or CPU.")}</span></div></li>
            <li><Icon name="apps" size={16} /><div><strong>{tr("En todas partes", "Everywhere")}</strong><span>{tr("Correo, chats, documentos, código…", "Email, chats, documents, code…")}</span></div></li>
          </ul>
          <div class="lang">
            <Segmented size="sm" value={app.settings.general.ui_language === "auto" ? app.lang : app.settings.general.ui_language} onchange={(v) => (app.settings.general.ui_language = v)} options={[
              { value: "es", label: "Español" },
              { value: "en", label: "English" },
            ]} />
          </div>
          <div class="nav"><span></span><Button variant="primary" size="lg" iconRight="arrow" onclick={() => go(1)}>{tr("Empezar", "Get started")}</Button></div>

        {:else if step === 1}
          <span class="kicker">1 / 4</span>
          <h1>{tr("Tu micrófono", "Your microphone")}</h1>
          <p class="lead">{tr("Elige el micrófono y habla para comprobar que llega la señal. Solo se escucha mientras la prueba está activa.", "Choose the microphone and speak to check the signal. It only listens while the test is running.")}</p>
          <div class="field">
            <Select value={app.settings.audio.input_device} options={deviceOptions} width="100%" onchange={chooseDevice} />
          </div>
          <div class="meter-row">
            <Button variant={testing ? "primary" : "secondary"} icon={testing ? "stop" : "mic"} onclick={toggleTest}>{testing ? tr("Detener", "Stop") : tr("Probar", "Test")}</Button>
            <div class="meter"><MicMeter {db} active={testing} /></div>
          </div>
          <p class="status" class:ok={heard}>
            {#if micError}<Icon name="alert" size={14} />{micError}
            {:else if heard}<Icon name="check" size={14} />{tr("Te oímos bien.", "We hear you.")}
            {:else if testing}{tr("Di algo en voz alta…", "Say something out loud…")}
            {:else}&nbsp;{/if}
          </p>
          <button class="link" onclick={() => api.openSystemSettings("microphone")}>{tr("¿No aparece la señal? Revisa los permisos del micrófono en Windows", "No signal? Check microphone permissions in Windows")}</button>
          <div class="nav">
            <Button variant="ghost" icon="arrow-left" onclick={() => go(0)}>{tr("Atrás", "Back")}</Button>
            <Button variant="primary" iconRight="arrow" onclick={() => go(2)}>{tr("Continuar", "Continue")}</Button>
          </div>

        {:else if step === 2}
          <span class="kicker">2 / 4</span>
          <h1>{tr("Motor de voz", "Speech engine")}</h1>
          {#if installed}
            <p class="lead">{tr("Todo listo: el reconocimiento funciona en tu equipo y sin conexión.", "All set: recognition runs on your computer, offline.")}</p>
            <div class="model ready">
              <Icon name="check" size={18} />
              <div><strong>{installed.name.replace(" (recomendado)", "")}</strong><span>{tr("Instalado", "Installed")} · {bytes(installed.size_bytes)}</span></div>
            </div>
          {:else}
            <p class="lead">{tr("Descarga una vez el modelo de reconocimiento. Después todo funciona sin Internet.", "Download the recognition model once. After that everything works offline.")}</p>
            {#if recommended}
              <div class="model">
                <Icon name="wave" size={18} />
                <div>
                  <strong>{recommended.name.replace(" (recomendado)", "")}</strong>
                  <span>{recommended.download_mb} MB · {tr("español, inglés y 23 idiomas más · detección automática", "Spanish, English and 23 more languages · auto-detect")}</span>
                </div>
              </div>
              {#if downloading}
                <div class="dl">
                  <Progress value={pct} indeterminate={!progress || progress.phase === "extracting"} />
                  <span class="faint num">
                    {#if progress?.phase === "extracting"}{tr("Verificando e instalando…", "Verifying and installing…")}
                    {:else if progress}{bytes(progress.done_bytes)} / {bytes(progress.total_bytes)}{:else}{tr("Conectando…", "Connecting…")}{/if}
                  </span>
                </div>
                <p class="hint faint">{tr("Puedes continuar: la descarga sigue en segundo plano.", "You can continue: the download keeps going in the background.")}</p>
              {:else}
                {#if dlError}<p class="status"><Icon name="alert" size={14} />{dlError}</p>{/if}
                <div class="dl-actions">
                  <Button variant="primary" icon="download" onclick={download}>{dlError ? tr("Reintentar", "Retry") : tr("Descargar", "Download")}</Button>
                </div>
              {/if}
            {:else}
              <p class="status"><Icon name="alert" size={14} />{tr("No se pudo leer el catálogo de modelos.", "Couldn't read the model catalog.")}</p>
            {/if}
            <p class="hint faint">{tr("¿Prefieres la nube (OpenAI, Groq) o importar un modelo? Podrás hacerlo en Ajustes › Motor de voz.", "Prefer the cloud (OpenAI, Groq) or importing a model? You can in Settings › Speech engine.")}</p>
          {/if}
          <div class="nav">
            <Button variant="ghost" icon="arrow-left" onclick={() => go(1)}>{tr("Atrás", "Back")}</Button>
            <Button variant="primary" iconRight="arrow" onclick={() => go(3)}>{tr("Continuar", "Continue")}</Button>
          </div>

        {:else if step === 3}
          <span class="kicker">3 / 4</span>
          <h1>{tr("Tu atajo", "Your shortcut")}</h1>
          <div class="big-kbd"><Kbd keys={shortcut} size="lg" /></div>
          <ul class="how">
            <li><span class="n">1</span>{tr("Mantén pulsado mientras hablas y suelta: el texto aparece donde esté el cursor.", "Hold while you speak and release: text appears at the cursor.")}</li>
            <li><span class="n">2</span>{tr("Púlsalo una vez para dictar sin manos. Vuelve a pulsarlo para terminar.", "Tap once to dictate hands-free. Tap again to finish.")}</li>
            <li><span class="n">3</span><span>{tr("Pulsa", "Press")} <Kbd keys="Esc" size="sm" /> {tr("para cancelar en cualquier momento.", "to cancel at any time.")}</span></li>
          </ul>
          {#if app.boot && !app.boot.hotkeys_available}
            <p class="status"><Icon name="alert" size={14} />{app.boot.hotkey_error ?? tr("Los atajos globales no están disponibles.", "Global shortcuts are unavailable.")}</p>
          {/if}
          <div class="change">
            <span class="faint">{tr("¿Prefieres otra combinación?", "Prefer another combination?")}</span>
            <HotkeyField bind:value={app.settings.dictation.hotkeys.dictate} action="dictate" fallback={defaults.dictation.hotkeys.dictate} clearable={false} align="start" />
          </div>
          <div class="nav">
            <Button variant="ghost" icon="arrow-left" onclick={() => go(2)}>{tr("Atrás", "Back")}</Button>
            <Button variant="primary" iconRight="arrow" onclick={() => go(4)}>{tr("Probarlo", "Try it")}</Button>
          </div>

        {:else}
          <span class="kicker">4 / 4</span>
          <h1>{result ? tr("¡Funciona!", "It works!") : tr("Pruébalo", "Try it")}</h1>
          {#if needsModel && !downloading}
            <p class="status"><Icon name="alert" size={14} />{tr("Antes necesitas el modelo de voz (paso 2).", "You need the speech model first (step 2).")}</p>
          {:else if needsModel}
            <p class="lead">{tr("Espera a que termine la descarga del modelo y prueba aquí.", "Wait for the model download to finish and try here.")}</p>
          {:else}
            <p class="lead"><span>{tr("Haz clic en el cuadro, mantén", "Click the box, hold")} <Kbd keys={shortcut} size="sm" /> {tr("y di, por ejemplo:", "and say, for example:")}</span> <em>{tr("«Hola, esto es una prueba de dictado y funciona muy bien.»", "“Hello, this is a dictation test and it works really well.”")}</em></p>
          {/if}
          <textarea bind:this={area} bind:value={tryText} rows="4" placeholder={tr("El texto dictado aparecerá aquí…", "Dictated text will appear here…")}></textarea>
          {#if result}
            <p class="status ok"><Icon name="check" size={14} />{tr("Dictado insertado. Ya puedes usarlo en cualquier aplicación.", "Dictation inserted. You can now use it in any app.")}</p>
          {/if}
          <label class="startup">
            <span><strong>{tr("Abrir al iniciar Windows", "Open when Windows starts")}</strong><em>{tr("En segundo plano, lista para dictar. Puedes cambiarlo en Ajustes.", "In the background, ready to dictate. You can change it in Settings.")}</em></span>
            <Toggle bind:checked={app.settings.general.launch_at_startup} label={tr("Abrir al iniciar Windows", "Open when Windows starts")} />
          </label>
          <div class="nav">
            <Button variant="ghost" icon="arrow-left" onclick={() => go(3)}>{tr("Atrás", "Back")}</Button>
            <Button variant="primary" iconRight="check" onclick={finish}>{tr("Terminar", "Finish")}</Button>
          </div>
        {/if}
      </div>
    {/key}
  </div>

  <div class="dots" aria-hidden="true">
    {#each Array(STEPS) as _, i}<span class:on={i === step} class:done={i < step}></span>{/each}
  </div>
</div>

<style>
  .welcome {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg);
  }
  .top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 24px 0;
  }
  .skip {
    all: unset;
    cursor: pointer;
    font-size: var(--text-sm);
    color: var(--text-3);
    padding: 4px 6px;
    border-radius: 6px;
  }
  .skip:hover {
    color: var(--text);
  }
  .skip:focus-visible,
  .link:focus-visible {
    outline: 2px solid var(--focus);
  }
  .stage {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: grid;
    place-items: center;
    padding: 20px 24px;
  }
  .card {
    width: 100%;
    max-width: 520px;
    display: flex;
    flex-direction: column;
    animation: enter 260ms var(--ease);
  }
  @keyframes enter {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
  }
  .kicker {
    font-size: var(--text-xs);
    color: var(--text-3);
    letter-spacing: 0.06em;
    margin-bottom: 10px;
  }
  h1 {
    font-size: var(--text-2xl);
    font-weight: 640;
    letter-spacing: -0.03em;
    line-height: 1.15;
    margin: 0 0 12px;
  }
  .lead {
    margin: 0 0 22px;
    color: var(--text-2);
    font-size: var(--text-lg);
    line-height: 1.55;
  }
  .lead em {
    display: block;
    margin-top: 8px;
    font-style: normal;
    color: var(--text);
  }
  .points {
    list-style: none;
    margin: 4px 0 26px;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .points li {
    display: flex;
    gap: 14px;
    align-items: flex-start;
  }
  .points li > :global(svg) {
    margin-top: 2px;
    flex-shrink: 0;
  }
  .points div,
  .model div {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .points strong,
  .model strong {
    font-size: var(--text-md);
    font-weight: 560;
  }
  .points span,
  .model span {
    font-size: var(--text-sm);
    color: var(--text-2);
  }
  .lang {
    margin-bottom: 8px;
  }
  .field {
    margin-bottom: 12px;
  }
  .meter-row {
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .meter {
    flex: 1;
  }
  .status {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 20px;
    margin: 12px 0 0;
    font-size: var(--text-sm);
    color: var(--text-2);
  }
  .status.ok {
    color: var(--text);
  }
  .status :global(svg) {
    flex-shrink: 0;
  }
  .link {
    all: unset;
    cursor: pointer;
    margin-top: 14px;
    font-size: var(--text-sm);
    color: var(--text-3);
    text-decoration: underline;
    text-underline-offset: 3px;
    align-self: flex-start;
  }
  .link:hover {
    color: var(--text);
  }
  .model {
    display: flex;
    gap: 14px;
    align-items: center;
    padding: 14px 16px;
    border-radius: var(--radius);
    border: 1px solid var(--border-strong);
    background: var(--surface);
  }
  .dl {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 16px;
  }
  .dl span {
    font-size: var(--text-xs);
  }
  .dl-actions {
    margin-top: 16px;
  }
  .hint {
    font-size: var(--text-sm);
    margin: 14px 0 0;
    line-height: 1.5;
  }
  .big-kbd {
    margin: 6px 0 22px;
  }
  .how {
    list-style: none;
    margin: 0 0 18px;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 12px;
    font-size: var(--text-md);
    color: var(--text-2);
  }
  .how li {
    display: flex;
    gap: 12px;
    align-items: flex-start;
    line-height: 1.5;
  }
  .n {
    width: 20px;
    height: 20px;
    border-radius: 10px;
    background: var(--surface-3);
    color: var(--text);
    font-size: var(--text-xs);
    display: grid;
    place-items: center;
    flex-shrink: 0;
    margin-top: 1px;
  }
  .change {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding-top: 14px;
    border-top: 1px solid var(--border);
    font-size: var(--text-sm);
  }
  textarea {
    width: 100%;
    box-sizing: border-box;
    resize: none;
    font: inherit;
    font-size: var(--text-lg);
    line-height: 1.5;
    color: var(--text);
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    padding: 14px 16px;
    outline: none;
  }
  textarea:focus {
    border-color: var(--text-3);
    box-shadow: 0 0 0 3px var(--focus);
  }
  .startup {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    margin-top: 18px;
    padding: 12px 14px;
    border-radius: var(--radius);
    border: 1px solid var(--border);
    cursor: pointer;
  }
  .startup span {
    display: flex;
    flex-direction: column;
    gap: 1px;
    font-size: var(--text-sm);
  }
  .startup strong {
    font-weight: 540;
    font-size: var(--text-md);
  }
  .startup em {
    font-style: normal;
    color: var(--text-2);
  }
  .nav {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-top: 30px;
  }
  .dots {
    display: flex;
    justify-content: center;
    gap: 6px;
    padding: 0 0 22px;
  }
  .dots span {
    width: 6px;
    height: 6px;
    border-radius: 3px;
    background: var(--surface-3);
    transition: all var(--normal) var(--ease);
  }
  .dots span.done {
    background: var(--text-3);
  }
  .dots span.on {
    width: 18px;
    background: var(--text);
  }
</style>
