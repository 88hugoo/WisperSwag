<script lang="ts">
  import { router } from "../../lib/router.svelte";
  import { tr } from "../../lib/store.svelte";
  import Icon from "../../components/Icon.svelte";
  import Input from "../../components/Input.svelte";
  import General from "./General.svelte";
  import Appearance from "./Appearance.svelte";
  import Audio from "./Audio.svelte";
  import Engine from "./Engine.svelte";
  import Dictation from "./Dictation.svelte";
  import Ai from "./Ai.svelte";
  import Privacy from "./Privacy.svelte";
  import Performance from "./Performance.svelte";
  import About from "./About.svelte";

  const sections = $derived([
    { id: "general", icon: "settings", label: tr("General", "General") },
    { id: "appearance", icon: "sun", label: tr("Apariencia", "Appearance") },
    { id: "audio", icon: "mic", label: tr("Voz y micrófono", "Voice & microphone") },
    { id: "engine", icon: "wave", label: tr("Motor de voz", "Speech engine") },
    { id: "dictation", icon: "keyboard", label: tr("Dictado y atajos", "Dictation & shortcuts") },
    { id: "ai", icon: "sparkle", label: tr("IA e integraciones", "AI & integrations") },
    { id: "privacy", icon: "lock", label: tr("Privacidad", "Privacy") },
    { id: "performance", icon: "cpu", label: tr("Rendimiento", "Performance") },
    { id: "about", icon: "info", label: tr("Acerca de", "About") },
  ]);

  // Índice de búsqueda: sección → términos.
  const index = $derived([
    ["general", tr("Idioma de la interfaz, inicio con Windows, iniciar minimizada, al cerrar, bandeja, notificaciones, sonidos, volumen, actualizaciones, copia de seguridad", "Interface language, start with Windows, start minimized, on close, tray, notifications, sounds, volume, updates, backup")],
    ["appearance", tr("Tema oscuro claro automático, tamaño de la barra, opacidad, posición, monitor, barra en reposo, animaciones, nivel de información, pantalla completa, juegos, vista previa", "Dark light theme, bar size, opacity, position, monitor, idle bar, animations, information level, fullscreen, games, preview")],
    ["audio", tr("Micrófono, prueba, nivel de entrada, ganancia automática, filtro, duración máxima, silenciar audio, permisos de Windows", "Microphone, test, input level, auto gain, filter, max duration, mute audio, Windows permissions")],
    ["engine", tr("Motor de reconocimiento, modelo local, descargar Parakeet Whisper, idioma, detección automática, hilos, memoria, nube OpenAI Groq, clave de API", "Recognition engine, local model, download Parakeet Whisper, language, auto-detect, threads, memory, cloud OpenAI Groq, API key")],
    ["dictation", tr("Atajos globales, mantener pulsado, alternancia, híbrido, inserción automática, portapapeles, escribir, espaciado, limpieza, muletillas, repeticiones, puntuación dictada, mayúsculas, números", "Global shortcuts, hold, toggle, hybrid, auto insert, clipboard, typing, spacing, cleanup, fillers, repetitions, spoken punctuation, capitalization, numbers")],
    ["ai", tr("Proveedor de IA, OpenAI, Claude Anthropic, Groq, Ollama, modelo, clave, límites, coste, prueba de conexión, datos enviados", "AI provider, OpenAI, Claude Anthropic, Groq, Ollama, model, key, limits, cost, connection test, data sent")],
    ["privacy", tr("Historial, retención, audio, borrar datos, nombre de la aplicación, conexiones externas, funcionamiento local", "History, retention, audio, delete data, app name, external connections, local operation")],
    ["performance", tr("Transcripción incremental, fotogramas, bajo consumo, velocidad de escritura, diagnóstico, memoria, CPU, registros, exportar importar ajustes, restablecer", "Incremental transcription, frames, low power, typing speed, diagnostics, memory, CPU, logs, export import settings, reset")],
    ["about", tr("Versión, licencias, actualizaciones", "Version, licenses, updates")],
  ]);

  let query = $state("");
  const norm = (s: string) => s.toLowerCase().normalize("NFD").replace(/[̀-ͯ]/g, "");
  const results = $derived(
    query.trim()
      ? index.filter(([id, text]) => norm(text + " " + sections.find((s) => s.id === id)?.label).includes(norm(query.trim())))
      : [],
  );
  const current = $derived(router.sub || "general");
</script>

<div class="settings">
  <aside>
    <h1>{tr("Ajustes", "Settings")}</h1>
    <Input bind:value={query} icon="search" placeholder={tr("Buscar ajustes", "Search settings")} width="100%" />
    {#if query.trim()}
      <ul class="results">
        {#each results as [id, text]}
          {@const s = sections.find((x) => x.id === id)}
          <li>
            <a href="#/settings/{id}" onclick={() => (query = "")}>
              <Icon name={s?.icon ?? "settings"} size={14} />
              <span><strong>{s?.label}</strong><em>{text.split(",").filter((t) => norm(t).includes(norm(query.trim()))).slice(0, 3).join(", ") || text.split(",")[0]}</em></span>
            </a>
          </li>
        {:else}
          <li class="none faint">{tr("Sin resultados", "No results")}</li>
        {/each}
      </ul>
    {:else}
      <nav>
        {#each sections as s}
          <a href="#/settings/{s.id}" class:active={current === s.id}>
            <Icon name={s.icon} size={15} />
            {s.label}
          </a>
        {/each}
      </nav>
    {/if}
  </aside>
  <div class="content">
    {#key current}
      {#if current === "general"}<General />
      {:else if current === "appearance"}<Appearance />
      {:else if current === "audio"}<Audio />
      {:else if current === "engine"}<Engine />
      {:else if current === "dictation"}<Dictation />
      {:else if current === "ai"}<Ai />
      {:else if current === "privacy"}<Privacy />
      {:else if current === "performance"}<Performance />
      {:else if current === "about"}<About />
      {:else}<General />{/if}
    {/key}
  </div>
</div>

<style>
  .settings {
    display: grid;
    grid-template-columns: 220px 1fr;
    gap: 36px;
    align-items: start;
  }
  aside {
    position: sticky;
    top: 0;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  h1 {
    font-size: 22px;
    padding: 6px 2px 2px;
  }
  nav {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  nav a {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 32px;
    padding: 0 10px;
    border-radius: var(--radius-sm);
    color: var(--text-2);
    text-decoration: none;
    font-size: var(--text-md);
  }
  nav a:hover {
    background: var(--hover);
    color: var(--text);
  }
  nav a.active {
    background: var(--active);
    color: var(--text);
  }
  .results {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .results a {
    display: flex;
    gap: 10px;
    padding: 8px 10px;
    border-radius: var(--radius-sm);
    text-decoration: none;
    color: var(--text);
  }
  .results a:hover {
    background: var(--hover);
  }
  .results :global(svg) {
    margin-top: 3px;
  }
  .results span {
    display: flex;
    flex-direction: column;
    font-size: var(--text-sm);
  }
  .results em {
    font-style: normal;
    color: var(--text-3);
    font-size: var(--text-xs);
  }
  .none {
    font-size: var(--text-sm);
    padding: 8px 10px;
  }
  .content {
    min-width: 0;
    padding-top: 6px;
  }
</style>
