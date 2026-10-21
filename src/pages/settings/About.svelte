<script lang="ts">
  import { api, errorText, openUrl } from "../../lib/api";
  import { app, tr } from "../../lib/store.svelte";
  import type { UpdateInfo } from "../../lib/types";
  import Button from "../../components/Button.svelte";
  import Row from "../../components/Row.svelte";
  import Section from "../../components/Section.svelte";

  const REPO = "https://github.com/88hugoo/WisperSwag";
  let update = $state<UpdateInfo | null>(null);
  let checking = $state(false);

  async function check() {
    checking = true;
    try {
      update = await api.checkUpdates();
    } catch (e) {
      app.toast(errorText(e), "error");
    } finally {
      checking = false;
    }
  }

  const credits: { name: string; role: [string, string]; license: string; url: string }[] = [
    { name: "sherpa-onnx", role: ["Motor de reconocimiento de voz local", "Local speech recognition engine"], license: "Apache-2.0", url: "https://github.com/k2-fsa/sherpa-onnx" },
    { name: "ONNX Runtime", role: ["Ejecución de los modelos", "Model inference"], license: "MIT", url: "https://github.com/microsoft/onnxruntime" },
    { name: "NVIDIA Parakeet TDT 0.6B v3", role: ["Modelo de voz multilingüe", "Multilingual speech model"], license: "CC-BY-4.0", url: "https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3" },
    { name: "OpenAI Whisper", role: ["Modelos de voz (opcionales)", "Speech models (optional)"], license: "MIT", url: "https://github.com/openai/whisper" },
    { name: "Tauri", role: ["Aplicación de escritorio", "Desktop app framework"], license: "MIT / Apache-2.0", url: "https://tauri.app" },
    { name: "Svelte", role: ["Interfaz", "Interface"], license: "MIT", url: "https://svelte.dev" },
    { name: "tiny-skia · fontdue", role: ["Dibujo de la barra flotante", "Floating bar rendering"], license: "BSD-3-Clause · MIT", url: "https://github.com/linebender/tiny-skia" },
    { name: "Inter", role: ["Tipografía", "Typeface"], license: "SIL OFL 1.1", url: "https://rsms.me/inter/" },
    { name: "SQLite · rusqlite", role: ["Historial local", "Local history"], license: "Dominio público · MIT", url: "https://www.sqlite.org" },
    { name: "cpal · windows-rs · reqwest", role: ["Audio, Windows y red", "Audio, Windows and networking"], license: "Apache-2.0 / MIT", url: "https://github.com/RustAudio/cpal" },
  ];
</script>

<h2 class="h">{tr("Acerca de", "About")}</h2>

<div class="hero">
  <img src="/icon.png" alt="" width="56" height="56" />
  <div>
    <strong>Wispr Swag</strong>
    <span class="faint">{tr("Versión", "Version")} {app.boot?.version ?? "—"} · Windows</span>
  </div>
</div>

<Section>
  <Row title={tr("Actualizaciones", "Updates")} description={update
    ? update.newer
      ? `${tr("Nueva versión disponible", "New version available")}: ${update.latest}`
      : update.latest ? tr("Tienes la última versión.", "You're up to date.") : tr("Aún no hay versiones publicadas.", "No releases published yet.")
    : tr("Se consultan las versiones publicadas en GitHub.", "Checks the releases published on GitHub.")}>
    {#if update?.newer}<Button variant="primary" icon="external" onclick={() => openUrl(update!.url)}>{tr("Descargar", "Download")}</Button>{/if}
    <Button icon="refresh" loading={checking} onclick={check}>{tr("Buscar", "Check")}</Button>
  </Row>
  <Row title={tr("Código fuente", "Source code")} description="github.com/88hugoo/WisperSwag">
    <Button variant="ghost" iconRight="external" onclick={() => openUrl(REPO)}>GitHub</Button>
  </Row>
  <Row title={tr("Informar de un problema", "Report a problem")} description={tr("Adjunta el informe de Rendimiento › Diagnóstico; no contiene datos personales.", "Attach the report from Performance › Diagnostics; it contains no personal data.")}>
    <Button variant="ghost" iconRight="external" onclick={() => openUrl(`${REPO}/issues`)}>{tr("Abrir incidencia", "Open issue")}</Button>
  </Row>
</Section>

<Section title={tr("Componentes de terceros", "Third-party components")}>
  {#each credits as c}
    <div class="credit">
      <div class="cn"><button class="link" onclick={() => openUrl(c.url)}>{c.name}</button><span class="faint">{tr(c.role[0], c.role[1])}</span></div>
      <span class="lic mono">{c.license}</span>
    </div>
  {/each}
</Section>

<p class="legal faint">{tr(
  "Wispr Swag es un proyecto independiente y no está afiliado a Wispr Flow. Los modelos de voz se descargan de sus publicaciones originales y conservan sus licencias.",
  "Wispr Swag is an independent project and is not affiliated with Wispr Flow. Speech models are downloaded from their original releases and keep their licenses.",
)}</p>

<style>
  .h {
    margin-bottom: 20px;
  }
  .hero {
    display: flex;
    align-items: center;
    gap: 16px;
    margin-bottom: 26px;
  }
  .hero img {
    border-radius: 14px;
  }
  .hero div {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .hero strong {
    font-size: var(--text-xl);
    letter-spacing: -0.02em;
  }
  .hero span {
    font-size: var(--text-sm);
  }
  .credit {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 10px 18px;
    border-bottom: 1px solid var(--border);
  }
  .credit:last-child {
    border-bottom: none;
  }
  .cn {
    display: flex;
    flex-direction: column;
    gap: 1px;
    font-size: var(--text-sm);
  }
  .link {
    all: unset;
    cursor: pointer;
    font-size: var(--text-md);
    font-weight: 520;
    color: var(--text);
  }
  .link:hover {
    text-decoration: underline;
    text-underline-offset: 3px;
  }
  .link:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
    border-radius: 4px;
  }
  .lic {
    font-size: var(--text-xs);
    color: var(--text-2);
    white-space: nowrap;
  }
  .legal {
    font-size: var(--text-xs);
    line-height: 1.55;
    max-width: 640px;
    margin: -4px 2px 10px;
  }
</style>
