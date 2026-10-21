<script lang="ts">
  import { onMount } from "svelte";
  import { api, errorText, on } from "../../lib/api";
  import { app, tr } from "../../lib/store.svelte";
  import type { InputDevice, RecordTest } from "../../lib/types";
  import Button from "../../components/Button.svelte";
  import Icon from "../../components/Icon.svelte";
  import MicMeter from "../../components/MicMeter.svelte";
  import Row from "../../components/Row.svelte";
  import Section from "../../components/Section.svelte";
  import Select from "../../components/Select.svelte";
  import Toggle from "../../components/Toggle.svelte";

  let devices = $state<InputDevice[]>([]);
  let monitoring = $state(false);
  let db = $state(-90);
  let peakDb = $state(-90);
  let monitorError = $state<string | null>(null);
  let recording = $state(false);
  let countdown = $state(0);
  let result = $state<RecordTest | null>(null);

  async function loadDevices() {
    devices = await api.listDevices();
  }

  onMount(() => {
    loadDevices();
    const off = on("mic-level", (p: any) => {
      if (p.stopped) {
        monitoring = false;
        db = -90;
        return;
      }
      db = p.db;
      peakDb = Math.max(peakDb - 0.6, 20 * Math.log10(Math.max(p.peak, 1e-6)));
      monitorError = p.error ?? null;
    });
    return () => {
      off.then((f) => f());
      if (monitoring) api.micTestStop();
    };
  });

  const deviceOptions = $derived([
    { value: null, label: tr("Predeterminado del sistema", "System default") },
    ...devices.map((d) => ({ value: d.id, label: d.name + (d.is_default ? tr(" (predeterminado)", " (default)") : "") })),
    ...(app.settings.audio.input_device && !devices.some((d) => d.id === app.settings.audio.input_device)
      ? [{ value: app.settings.audio.input_device, label: `${app.settings.audio.input_device_name ?? "?"} — ${tr("no conectado", "not connected")}` }]
      : []),
  ]);

  function chooseDevice(id: string | null) {
    app.settings.audio.input_device = id;
    app.settings.audio.input_device_name = devices.find((d) => d.id === id)?.name ?? null;
    if (monitoring) restartMonitor();
  }

  async function toggleMonitor() {
    if (monitoring) {
      await api.micTestStop();
      monitoring = false;
      return;
    }
    try {
      monitorError = null;
      await api.micTestStart(app.settings.audio.input_device);
      monitoring = true;
    } catch (e) {
      monitorError = errorText(e);
    }
  }

  async function restartMonitor() {
    await api.micTestStop();
    await api.micTestStart(app.settings.audio.input_device);
  }

  async function recordTest() {
    if (monitoring) {
      await api.micTestStop();
      monitoring = false;
    }
    await app.saveNow();
    recording = true;
    result = null;
    countdown = 5;
    const t = setInterval(() => (countdown = Math.max(0, countdown - 1)), 1000);
    try {
      result = await api.micRecordTest(5);
    } catch (e) {
      app.toast(errorText(e), "error");
    } finally {
      clearInterval(t);
      recording = false;
    }
  }

  const advice = $derived.by(() => {
    if (!result) return [];
    const out: string[] = [];
    if (result.speech_ms < 300) out.push(tr("No se detectó voz. Comprueba que el micrófono correcto está seleccionado y no está silenciado.", "No speech detected. Check the right microphone is selected and not muted."));
    else if (result.peak_db < -30) out.push(tr("El nivel es bajo. La ganancia automática lo compensa, pero acercarte al micrófono mejora la precisión.", "Level is low. Auto gain compensates, but moving closer improves accuracy."));
    if (result.noise_db > -42) out.push(tr("Hay bastante ruido de fondo. El filtro de paso alto y un micrófono de auriculares ayudan.", "There's notable background noise. The high-pass filter and a headset mic help."));
    if (out.length === 0) out.push(tr("El micrófono suena bien.", "Your microphone sounds good."));
    return out;
  });
</script>

<h2 class="h">{tr("Voz y micrófono", "Voice & microphone")}</h2>

<Section title={tr("Micrófono", "Microphone")}>
  <Row title={tr("Dispositivo de entrada", "Input device")} description={tr("Si el elegido no está conectado, se usa el predeterminado y se avisa.", "If the chosen one isn't connected, the default is used and you're notified.")}>
    <Select value={app.settings.audio.input_device} options={deviceOptions} width="300px" onchange={chooseDevice} />
    <Button variant="ghost" icon="refresh" title={tr("Actualizar lista", "Refresh list")} onclick={loadDevices} />
  </Row>
  <Row title={tr("Nivel de entrada", "Input level")} description={monitorError ?? tr("Comprueba que la barra se mueve al hablar. El micrófono se libera al parar la prueba.", "Check the meter moves when you speak. The microphone is released when the test stops.")} stacked>
    <div class="meter-row">
      <Button variant={monitoring ? "primary" : "secondary"} icon={monitoring ? "stop" : "play"} onclick={toggleMonitor}>{monitoring ? tr("Detener", "Stop") : tr("Probar nivel", "Test level")}</Button>
      <div class="meter"><MicMeter {db} active={monitoring} /></div>
      <span class="db num faint">{monitoring ? `${Math.round(db)} dB` : "—"}</span>
    </div>
  </Row>
  <Row title={tr("Prueba completa", "Full test")} description={tr("Graba 5 segundos y los transcribe con el motor actual: verifica micrófono y reconocimiento a la vez.", "Records 5 seconds and transcribes them with the current engine: checks microphone and recognition together.")} stacked>
    <div class="meter-row">
      <Button icon="mic" loading={recording} onclick={recordTest}>{recording ? `${tr("Habla ahora…", "Speak now…")} ${countdown}` : tr("Grabar y transcribir", "Record and transcribe")}</Button>
    </div>
    {#if result}
      <div class="result">
        <div class="said">{result.text || tr("(sin texto)", "(no text)")}</div>
        <div class="metrics faint num">
          {tr("Pico", "Peak")} {result.peak_db.toFixed(0)} dB · {tr("Ruido", "Noise")} {result.noise_db.toFixed(0)} dB · {tr("Voz", "Speech")} {(result.speech_ms / 1000).toFixed(1)} s · {tr("Motor", "Engine")} {result.engine_ms} ms · {result.device}
        </div>
        <ul>
          {#each advice as a}<li><Icon name="info" size={13} />{a}</li>{/each}
        </ul>
      </div>
    {/if}
  </Row>
  <Row title={tr("Permisos del micrófono", "Microphone permissions")} description={tr("Si Windows bloquea el micrófono, actívalo para las aplicaciones de escritorio.", "If Windows blocks the microphone, allow it for desktop apps.")}>
    <Button variant="ghost" iconRight="external" onclick={() => api.openSystemSettings("microphone")}>{tr("Abrir privacidad de Windows", "Open Windows privacy")}</Button>
  </Row>
</Section>

<Section title={tr("Procesamiento de la señal", "Signal processing")}>
  <Row title={tr("Ganancia automática", "Auto gain")} description={tr("Normaliza el volumen de la voz antes de reconocerla.", "Normalizes voice volume before recognition.")}>
    <Toggle bind:checked={app.settings.audio.auto_gain} />
  </Row>
  <Row title={tr("Filtro de ruido grave", "Low-noise filter")} description={tr("Elimina zumbidos, golpes en la mesa y ruido de ventiladores.", "Removes hum, desk bumps and fan rumble.")}>
    <Toggle bind:checked={app.settings.audio.high_pass} />
  </Row>
  <Row title={tr("Silenciar el audio del sistema al dictar", "Mute system audio while dictating")} description={tr("Pausa el sonido de música y vídeos mientras hablas y lo restaura al terminar.", "Mutes music and video while you speak and restores it afterwards.")}>
    <Toggle bind:checked={app.settings.audio.mute_system_audio} />
  </Row>
  <Row title={tr("Duración máxima de un dictado", "Maximum dictation length")} description={tr("Al llegar al límite se termina y se transcribe automáticamente.", "When reached, it stops and transcribes automatically.")}>
    <Select bind:value={app.settings.audio.max_recording_secs} width="150px" options={[
      { value: 60, label: "1 min" },
      { value: 180, label: "3 min" },
      { value: 300, label: "5 min" },
      { value: 600, label: "10 min" },
      { value: 1200, label: "20 min" },
      { value: 1800, label: "30 min" },
    ]} />
  </Row>
</Section>

<style>
  .h {
    margin-bottom: 20px;
  }
  .meter-row {
    display: flex;
    align-items: center;
    gap: 14px;
    width: 100%;
  }
  .meter {
    flex: 1;
  }
  .db {
    width: 56px;
    text-align: right;
    font-size: var(--text-sm);
  }
  .result {
    width: 100%;
    padding: 14px;
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    border: 1px solid var(--border);
  }
  .said {
    font-size: var(--text-lg);
    font-weight: 500;
    user-select: text;
  }
  .metrics {
    margin-top: 4px;
    font-size: var(--text-xs);
  }
  ul {
    list-style: none;
    margin: 10px 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: var(--text-sm);
    color: var(--text-2);
  }
  li {
    display: flex;
    gap: 8px;
    align-items: flex-start;
  }
  li :global(svg) {
    margin-top: 3px;
  }
</style>
