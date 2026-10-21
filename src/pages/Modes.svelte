<script lang="ts">
  import defaults from "../lib/defaults.json";
  import { api, errorText } from "../lib/api";
  import { router } from "../lib/router.svelte";
  import { app, tr } from "../lib/store.svelte";
  import type { WritingMode } from "../lib/types";
  import Button from "../components/Button.svelte";
  import Icon from "../components/Icon.svelte";
  import Input from "../components/Input.svelte";
  import PageHeader from "../components/PageHeader.svelte";
  import Row from "../components/Row.svelte";
  import Section from "../components/Section.svelte";
  import Select from "../components/Select.svelte";
  import Slider from "../components/Slider.svelte";
  import TextArea from "../components/TextArea.svelte";
  import Toggle from "../components/Toggle.svelte";

  const ICONS: Record<string, string> = {
    natural: "mic",
    professional: "mail",
    concise: "minus",
    formal: "shield",
    informal: "message",
    email: "mail",
    messages: "message",
    code: "code",
    notes: "note",
    tasks: "check",
    creative: "sparkle",
    custom: "edit",
  };

  let selectedId = $state(router.sub || app.settings.writing.active_mode);
  const mode = $derived(app.settings.writing.modes.find((m) => m.id === selectedId) ?? app.settings.writing.modes[0]);
  let sample = $state(tr("eh bueno pues quería decirte que la reunión del jueves se mueve al viernes a las diez y que lleves el informe", "um so I wanted to tell you that thursday's meeting moves to friday at ten and please bring the report"));
  let output = $state("");
  let outputInfo = $state("");
  let testing = $state(false);
  const aiReady = $derived(app.settings.ai.enabled && app.settings.ai.consent_given);

  function select(id: string) {
    selectedId = id;
    output = "";
  }

  function duplicate(m: WritingMode) {
    const copy: WritingMode = { ...structuredClone($state.snapshot(m)), id: crypto.randomUUID(), name: `${m.name} (${tr("copia", "copy")})`, builtin: false };
    app.settings.writing.modes = [...app.settings.writing.modes, copy];
    select(copy.id);
  }

  function remove(m: WritingMode) {
    if (m.builtin) return;
    app.settings.writing.modes = app.settings.writing.modes.filter((x) => x.id !== m.id);
    if (app.settings.writing.active_mode === m.id) app.settings.writing.active_mode = "natural";
    select("natural");
  }

  function restore(m: WritingMode) {
    const d = (defaults as any).writing.modes.find((x: WritingMode) => x.id === m.id);
    if (d) Object.assign(m, structuredClone(d));
  }

  async function test(useAi: boolean) {
    testing = true;
    try {
      await app.saveNow();
      const r = await api.processText(sample, mode.id, useAi);
      output = r.command ? tr("(se interpretaría como un comando de voz)", "(would be interpreted as a voice command)") : r.text;
      outputInfo = r.ai_used ? tr("Procesado con IA", "Processed with AI") : r.ai_error ? `${tr("IA no disponible", "AI unavailable")}: ${r.ai_error}` : tr("Solo reglas locales", "Local rules only");
    } catch (e) {
      app.toast(errorText(e), "error");
    } finally {
      testing = false;
    }
  }
</script>

<PageHeader title={tr("Modos de escritura", "Writing modes")} description={tr("Cada modo define cómo se limpia y formatea lo que dictas. Funcionan sin conexión; con IA configurada, también reescriben el texto.", "Each mode defines how your dictation is cleaned and formatted. They work offline; with AI configured they also rewrite text.")} />

<div class="layout">
  <ul class="modes">
    {#each app.settings.writing.modes as m (m.id)}
      <li>
        <button class:selected={m.id === mode.id} onclick={() => select(m.id)}>
          <Icon name={ICONS[m.id] ?? "edit"} size={15} />
          <span class="n">{m.name}</span>
          {#if m.id === app.settings.writing.active_mode}<span class="act">{tr("Activo", "Active")}</span>{/if}
        </button>
      </li>
    {/each}
  </ul>

  {#if mode}
    <div class="editor">
      <div class="head">
        <div class="title">
          <Input bind:value={mode.name} width="260px" label={tr("Nombre", "Name")} />
          <Input bind:value={mode.description} placeholder={tr("Descripción", "Description")} width="100%" />
        </div>
        <div class="acts">
          {#if mode.id === app.settings.writing.active_mode}
            <Button variant="outline" icon="check" disabled>{tr("Modo predeterminado", "Default mode")}</Button>
          {:else}
            <Button variant="primary" onclick={() => (app.settings.writing.active_mode = mode.id)}>{tr("Usar por defecto", "Use by default")}</Button>
          {/if}
          <Button variant="ghost" icon="copy" title={tr("Duplicar", "Duplicate")} onclick={() => duplicate(mode)} />
          {#if mode.builtin}
            <Button variant="ghost" icon="refresh" title={tr("Restaurar valores originales", "Restore defaults")} onclick={() => restore(mode)} />
          {:else}
            <Button variant="ghost" icon="trash" title={tr("Eliminar", "Delete")} onclick={() => remove(mode)} />
          {/if}
        </div>
      </div>

      <Section title={tr("Reglas locales (siempre activas, sin conexión)", "Local rules (always on, offline)")}>
        <Row title={tr("Estructura", "Structure")} description={tr("Cómo se organiza el texto.", "How the text is organized.")}>
          <Select bind:value={mode.local.structure} width="210px" options={[
            { value: "plain", label: tr("Texto continuo", "Plain text") },
            { value: "paragraphs", label: tr("Párrafos en las transiciones", "Paragraphs at transitions") },
            { value: "bullets", label: tr("Viñetas", "Bullets") },
            { value: "checklist", label: tr("Lista de tareas", "Checklist") },
            { value: "email", label: tr("Correo (saludo/cuerpo/despedida)", "Email (greeting/body/closing)") },
            { value: "code", label: tr("Código (símbolos y camelCase)", "Code (symbols & camelCase)") },
          ]} />
        </Row>
        <Row title={tr("Punto final", "Final period")}>
          <Select bind:value={mode.local.final_period} width="210px" options={[
            { value: "keep", label: tr("Mantener", "Keep") },
            { value: "remove_if_single_sentence", label: tr("Quitar en frases sueltas", "Remove for single sentences") },
            { value: "remove", label: tr("Quitar siempre", "Always remove") },
          ]} />
        </Row>
        <Row title={tr("Quitar coletillas", "Remove discourse markers")} description={tr("«Bueno, pues…», «o sea», «en plan» al inicio de frase.", "“Well, so…”, “I mean” at sentence start.")}>
          <Toggle bind:checked={mode.local.remove_discourse_markers} />
        </Row>
        <Row title={tr("Mayúsculas al inicio de frase", "Sentence capitalization")}>
          <Toggle bind:checked={mode.local.capitalize} />
        </Row>
        <Row title={tr("Empezar en minúscula", "Start in lowercase")} description={tr("Estilo de chat informal.", "Casual chat style.")}>
          <Toggle bind:checked={mode.local.lowercase_start} />
        </Row>
        <Row title={tr("Plantilla", "Template")} description={tr("Opcional. Usa {{texto}} para el dictado y {{fecha}}, {{hora}}, {{dia}} para la fecha.", "Optional. Use {{texto}} for the dictation and {{fecha}}, {{hora}}, {{dia}} for dates.")} stacked>
          <TextArea bind:value={mode.template} rows={2} placeholder={"{{texto}}"} mono />
        </Row>
      </Section>

      <Section title={tr("Inteligencia artificial", "Artificial intelligence")}>
        {#if !aiReady}
          <div class="ai-off">
            <Icon name="info" size={15} />
            <span>{tr("La IA no está configurada; este modo usará solo las reglas locales.", "AI isn't configured; this mode will use local rules only.")}</span>
            <Button size="sm" variant="ghost" onclick={() => router.go("settings/ai")}>{tr("Configurar", "Configure")}</Button>
          </div>
        {/if}
        <Row title={tr("Reescribir con IA", "Rewrite with AI")} description={tr("Envía el texto (nunca el audio) al proveedor configurado.", "Sends the text (never audio) to the configured provider.")}>
          <Toggle bind:checked={mode.ai_enabled} />
        </Row>
        <Row title={tr("Instrucciones", "Instructions")} description={tr("Qué debe hacer la IA con tu dictado.", "What the AI should do with your dictation.")} stacked>
          <TextArea bind:value={mode.ai_instructions} rows={4} placeholder={tr("p. ej. Reescribe como un mensaje breve para mi equipo, en tono cercano.", "e.g. Rewrite as a short message for my team, friendly tone.")} />
        </Row>
        <Row title={tr("Creatividad", "Creativity")} description={tr("Bajo = fiel a tus palabras; alto = más libre.", "Low = faithful; high = freer.")}>
          <Slider bind:value={mode.temperature} min={0} max={1} step={0.05} format={(v) => v.toFixed(2)} />
        </Row>
      </Section>

      <Section title={tr("Probar este modo", "Try this mode")}>
        <Row title={tr("Texto de ejemplo", "Sample text")} description={tr("Escríbelo como lo reconocería el motor de voz.", "Type it as the speech engine would recognize it.")} stacked>
          <TextArea bind:value={sample} rows={3} />
          <div class="test-btns">
            <Button onclick={() => test(false)} loading={testing}>{tr("Probar sin IA", "Test without AI")}</Button>
            <Button variant="primary" icon="sparkle" onclick={() => test(true)} disabled={!aiReady || !mode.ai_enabled} loading={testing}>{tr("Probar con IA", "Test with AI")}</Button>
          </div>
          {#if output}
            <div class="out">
              <div class="oi faint">{outputInfo}</div>
              <div class="ot">{output}</div>
            </div>
          {/if}
        </Row>
      </Section>
    </div>
  {/if}
</div>

<style>
  .layout {
    display: grid;
    grid-template-columns: 210px 1fr;
    gap: 22px;
    align-items: start;
  }
  .modes {
    list-style: none;
    margin: 0;
    padding: 0;
    position: sticky;
    top: 0;
  }
  .modes button {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    height: 34px;
    padding: 0 10px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-2);
    cursor: pointer;
    font-size: var(--text-md);
    text-align: left;
  }
  .modes button:hover {
    background: var(--hover);
    color: var(--text);
  }
  .modes button.selected {
    background: var(--active);
    color: var(--text);
  }
  .n {
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .act {
    font-size: 10.5px;
    font-weight: 600;
    padding: 1px 6px;
    border-radius: 999px;
    background: var(--text);
    color: var(--bg);
  }
  .head {
    display: flex;
    gap: 14px;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 20px;
  }
  .title {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .acts {
    display: flex;
    gap: 4px;
  }
  .ai-off {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 18px;
    font-size: var(--text-sm);
    color: var(--text-2);
    border-bottom: 1px solid var(--border);
    background: var(--surface-2);
  }
  .ai-off span {
    flex: 1;
  }
  .test-btns {
    display: flex;
    gap: 8px;
  }
  .out {
    width: 100%;
    padding: 12px 14px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-strong);
    background: var(--surface-2);
  }
  .oi {
    font-size: var(--text-xs);
    margin-bottom: 4px;
  }
  .ot {
    white-space: pre-wrap;
    user-select: text;
  }
</style>
