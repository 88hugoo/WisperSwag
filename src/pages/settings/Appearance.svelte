<script lang="ts">
  import { app, tr } from "../../lib/store.svelte";
  import Button from "../../components/Button.svelte";
  import PillPreview from "../../components/PillPreview.svelte";
  import Row from "../../components/Row.svelte";
  import Section from "../../components/Section.svelte";
  import Segmented from "../../components/Segmented.svelte";
  import Select from "../../components/Select.svelte";
  import Slider from "../../components/Slider.svelte";
  import Toggle from "../../components/Toggle.svelte";

  const a = $derived(app.settings.appearance);
  let previewPhase = $state("handsfree");
  const previewTheme = $derived<"dark" | "light">(document.documentElement.dataset.theme === "light" ? "light" : "dark");
</script>

<h2 class="h">{tr("Apariencia", "Appearance")}</h2>

<div class="stage" class:light={previewTheme === "light"}>
  <div class="desk">
    <PillPreview phase={previewPhase} theme={previewTheme} />
  </div>
  <div class="phases">
    <Segmented size="sm" bind:value={previewPhase} options={[
      { value: "hover", label: tr("Reposo", "Idle") },
      { value: "hold", label: tr("Mantener", "Hold") },
      { value: "handsfree", label: tr("Manos libres", "Hands-free") },
      { value: "command", label: tr("Comando", "Command") },
      { value: "processing", label: tr("Procesando", "Processing") },
      { value: "notice", label: tr("Aviso", "Notice") },
      { value: "error", label: "Error" },
    ]} />
  </div>
</div>

<Section title={tr("Tema", "Theme")}>
  <Row title={tr("Tema", "Theme")} description={tr("Se aplica a esta ventana y a la barra flotante.", "Applies to this window and the floating bar.")}>
    <Segmented bind:value={app.settings.appearance.theme} options={[
      { value: "system", label: tr("Automático", "Auto"), icon: "monitor" },
      { value: "dark", label: tr("Oscuro", "Dark"), icon: "moon" },
      { value: "light", label: tr("Claro", "Light"), icon: "sun" },
    ]} />
  </Row>
  <Row title={tr("Animaciones", "Animations")} description={tr("Transiciones suaves. Desactívalas para el mínimo consumo.", "Smooth transitions. Turn off for minimum usage.")}>
    <Toggle bind:checked={app.settings.appearance.animations} />
  </Row>
</Section>

<Section title={tr("Barra flotante", "Floating bar")}>
  <Row title={tr("Tamaño", "Size")}>
    <Segmented bind:value={app.settings.appearance.pill_size} options={[
      { value: "small", label: tr("Pequeña", "Small") },
      { value: "medium", label: tr("Mediana", "Medium") },
      { value: "large", label: tr("Grande", "Large") },
    ]} />
  </Row>
  <Row title={tr("Opacidad del fondo", "Background opacity")}>
    <Slider bind:value={app.settings.appearance.pill_opacity} min={0.5} max={1} step={0.01} format={(v) => `${Math.round(v * 100)} %`} />
  </Row>
  <Row title={tr("Información visible", "Visible information")} description={tr("Mínima: solo la onda. Estándar: botones de detener y cancelar. Detallada: también tiempo y modo.", "Minimal: waveform only. Standard: stop and cancel buttons. Detailed: also time and mode.")}>
    <Segmented bind:value={app.settings.appearance.info_level} options={[
      { value: "minimal", label: tr("Mínima", "Minimal") },
      { value: "standard", label: tr("Estándar", "Standard") },
      { value: "detailed", label: tr("Detallada", "Detailed") },
    ]} />
  </Row>
  <Row title={tr("Posición", "Position")} description={a.pill_anchor === "custom" ? tr("Personalizada (arrastra la barra para moverla).", "Custom (drag the bar to move it).") : tr("También puedes arrastrar la barra a donde quieras.", "You can also drag the bar anywhere.")}>
    <Select bind:value={app.settings.appearance.pill_anchor} width="200px" options={[
      { value: "bottom_center", label: tr("Abajo, centrada", "Bottom center") },
      { value: "bottom_left", label: tr("Abajo a la izquierda", "Bottom left") },
      { value: "bottom_right", label: tr("Abajo a la derecha", "Bottom right") },
      { value: "top_center", label: tr("Arriba, centrada", "Top center") },
      { value: "custom", label: tr("Personalizada", "Custom") },
    ]} />
    {#if a.pill_anchor === "custom"}
      <Button size="sm" variant="ghost" icon="refresh" title={tr("Restablecer", "Reset")} onclick={() => (app.settings.appearance.pill_anchor = "bottom_center")} />
    {/if}
  </Row>
  <Row title={tr("Monitor", "Monitor")} description={tr("Dónde aparece la barra si tienes varias pantallas.", "Where the bar appears with multiple displays.")}>
    <Select bind:value={app.settings.appearance.pill_monitor} width="200px" options={[
      { value: "active", label: tr("El de la ventana activa", "Active window's") },
      { value: "cursor", label: tr("El del puntero", "Pointer's") },
      { value: "primary", label: tr("El principal", "Primary") },
    ]} />
  </Row>
  <Row title={tr("Mini barra en reposo", "Idle mini bar")} description={tr("Una línea discreta que se amplía al pasar el ratón; clic para dictar.", "A discreet line that expands on hover; click to dictate.")}>
    <Toggle bind:checked={app.settings.appearance.show_idle_pill} />
  </Row>
  <Row title={tr("Ocultar sobre juegos y pantalla completa", "Hide over games and fullscreen")} description={tr("El dictado sigue funcionando; solo no se dibuja la barra.", "Dictation still works; the bar just isn't drawn.")}>
    <Toggle bind:checked={app.settings.appearance.hide_in_fullscreen} />
  </Row>
  <Row title={tr("Vista previa del texto", "Text preview")} description={tr("Muestra encima de la barra lo reconocido por frases mientras hablas (requiere motor local y transcripción incremental).", "Shows recognized text above the bar, sentence by sentence (needs local engine and incremental transcription).")}>
    <Toggle bind:checked={app.settings.appearance.live_preview} />
  </Row>
</Section>

<style>
  .h {
    margin-bottom: 20px;
  }
  .stage {
    position: relative;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    background:
      radial-gradient(120% 140% at 50% 120%, rgba(255, 255, 255, 0.06), transparent 60%),
      repeating-linear-gradient(0deg, transparent 0 23px, rgba(255, 255, 255, 0.025) 23px 24px),
      repeating-linear-gradient(90deg, transparent 0 23px, rgba(255, 255, 255, 0.025) 23px 24px),
      #1a1a1d;
    padding: 34px 20px 16px;
    margin-bottom: 26px;
  }
  .stage.light {
    background:
      radial-gradient(120% 140% at 50% 120%, rgba(0, 0, 0, 0.05), transparent 60%),
      repeating-linear-gradient(0deg, transparent 0 23px, rgba(0, 0, 0, 0.03) 23px 24px),
      repeating-linear-gradient(90deg, transparent 0 23px, rgba(0, 0, 0, 0.03) 23px 24px),
      #e9e9ec;
  }
  .desk {
    height: 92px;
    display: flex;
    align-items: flex-end;
    justify-content: center;
  }
  .phases {
    display: flex;
    justify-content: center;
    margin-top: 14px;
  }
</style>
