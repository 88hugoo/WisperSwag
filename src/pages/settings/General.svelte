<script lang="ts">
  import { api, errorText, openUrl } from "../../lib/api";
  import { app, tr } from "../../lib/store.svelte";
  import type { UpdateInfo } from "../../lib/types";
  import Button from "../../components/Button.svelte";
  import Row from "../../components/Row.svelte";
  import Section from "../../components/Section.svelte";
  import Segmented from "../../components/Segmented.svelte";
  import Select from "../../components/Select.svelte";
  import Slider from "../../components/Slider.svelte";
  import Toggle from "../../components/Toggle.svelte";

  const g = $derived(app.settings.general);
  let update = $state<UpdateInfo | null>(null);
  let checking = $state(false);

  async function checkNow() {
    checking = true;
    try {
      update = await api.checkUpdates();
    } catch (e) {
      app.toast(errorText(e), "error");
    } finally {
      checking = false;
    }
  }
</script>

<h2 class="h">{tr("General", "General")}</h2>

<Section>
  <Row title={tr("Idioma de la interfaz", "Interface language")} description={tr("El idioma de dictado se elige en «Motor de voz».", "The dictation language is set in “Speech engine”.")}>
    <Segmented bind:value={app.settings.general.ui_language} options={[
      { value: "auto", label: tr("Sistema", "System") },
      { value: "es", label: "Español" },
      { value: "en", label: "English" },
    ]} />
  </Row>
</Section>

<Section title={tr("Inicio y cierre", "Startup & closing")}>
  <Row title={tr("Iniciar con Windows", "Start with Windows")} description={tr("Se abre en segundo plano, lista para dictar.", "Opens in the background, ready to dictate.")}>
    <Toggle bind:checked={app.settings.general.launch_at_startup} />
  </Row>
  <Row title={tr("Iniciar minimizada", "Start minimized")} description={tr("Al abrirla, solo aparece el icono de la bandeja y la barra.", "On launch, only the tray icon and the bar appear.")}>
    <Toggle bind:checked={app.settings.general.start_minimized} />
  </Row>
  <Row title={tr("Al cerrar esta ventana", "When closing this window")} description={tr("Wispr Swag sigue disponible en la bandeja en las dos primeras opciones.", "Wispr Swag stays available in the tray with the first two options.")}>
    <Select bind:value={app.settings.general.close_behavior} width="250px" options={[
      { value: "release_memory", label: tr("Cerrar y liberar memoria (recomendado)", "Close and free memory (recommended)") },
      { value: "hide_to_tray", label: tr("Ocultar en la bandeja (reabre al instante)", "Hide to tray (reopens instantly)") },
      { value: "quit", label: tr("Salir de la aplicación", "Quit the app") },
    ]} />
  </Row>
</Section>

<Section title={tr("Avisos y sonidos", "Notifications & sounds")}>
  <Row title={tr("Sonidos", "Sounds")} description={tr("Tonos breves al empezar, terminar o cancelar un dictado.", "Short tones when starting, finishing or cancelling.")}>
    <Toggle bind:checked={app.settings.general.sounds} />
  </Row>
  <Row title={tr("Volumen de los sonidos", "Sound volume")} disabled={!g.sounds}>
    <Slider bind:value={app.settings.general.sound_volume} min={0} max={1} step={0.05} format={(v) => `${Math.round(v * 100)} %`} />
  </Row>
  <Row title={tr("Notificaciones del sistema", "System notifications")} description={tr("Además de la barra, muestra los errores como notificación de Windows (útil con juegos a pantalla completa).", "Besides the bar, shows errors as Windows notifications (useful with fullscreen games).")}>
    <Toggle bind:checked={app.settings.general.system_notifications} />
  </Row>
</Section>

<Section title={tr("Mantenimiento", "Maintenance")}>
  <Row title={tr("Copia de seguridad diaria de los ajustes", "Daily settings backup")} description={tr("Guarda las 10 últimas en la carpeta de configuración.", "Keeps the last 10 in the config folder.")}>
    <Toggle bind:checked={app.settings.general.auto_backup} />
  </Row>
  <Row title={tr("Comprobar actualizaciones al iniciar", "Check for updates on startup")} description={tr("Consulta GitHub Releases (única conexión de esta opción). No descarga nada sin tu permiso.", "Queries GitHub Releases (the only connection made). Nothing is downloaded without your consent.")}>
    <Toggle bind:checked={app.settings.general.check_updates} />
  </Row>
  <Row title={tr("Buscar actualizaciones ahora", "Check for updates now")} description={update ? (update.newer ? `${tr("Disponible", "Available")}: ${update.latest}` : update.latest ? tr("Tienes la última versión.", "You're up to date.") : tr("Aún no hay versiones publicadas.", "No published releases yet.")) : undefined}>
    {#if update?.newer}<Button variant="primary" icon="external" onclick={() => openUrl(update!.url)}>{tr("Ver", "View")}</Button>{/if}
    <Button loading={checking} icon="refresh" onclick={checkNow}>{tr("Comprobar", "Check")}</Button>
  </Row>
</Section>

<style>
  .h {
    margin-bottom: 20px;
  }
</style>
