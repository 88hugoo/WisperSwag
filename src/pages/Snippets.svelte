<script lang="ts">
  import { app, tr } from "../lib/store.svelte";
  import type { Snippet } from "../lib/types";
  import Button from "../components/Button.svelte";
  import Empty from "../components/Empty.svelte";
  import Icon from "../components/Icon.svelte";
  import Input from "../components/Input.svelte";
  import PageHeader from "../components/PageHeader.svelte";
  import TextArea from "../components/TextArea.svelte";
  import Toggle from "../components/Toggle.svelte";

  let editing = $state<Snippet | null>(null);
  let isNew = $state(false);

  function create() {
    editing = { id: crypto.randomUUID(), trigger: "", content: "", enabled: true, whole_utterance_only: false };
    isNew = true;
  }

  function save() {
    if (!editing || !editing.trigger.trim() || !editing.content.trim()) return;
    const s = $state.snapshot(editing) as Snippet;
    s.trigger = s.trigger.trim();
    if (isNew) app.settings.snippets = [s, ...app.settings.snippets];
    else app.settings.snippets = app.settings.snippets.map((x) => (x.id === s.id ? s : x));
    editing = null;
    isNew = false;
  }

  function norm(s: string) {
    return s.toLowerCase().normalize("NFD").replace(/[̀-ͯ]/g, "").replace(/[^a-z0-9ñ ]/g, "").trim();
  }

  const conflict = $derived(
    editing && editing.trigger.trim()
      ? app.settings.snippets.find((s) => s.id !== editing!.id && (norm(s.trigger).includes(norm(editing!.trigger)) || norm(editing!.trigger).includes(norm(s.trigger))))
      : undefined,
  );

  const VARS = ["{{fecha}}", "{{fecha_larga}}", "{{hora}}", "{{dia}}", "{{año}}"];
</script>

<PageHeader title={tr("Fragmentos", "Snippets")} description={tr("Di una frase corta y Wispr Swag escribirá el texto completo: direcciones, firmas, respuestas frecuentes o plantillas.", "Say a short phrase and Wispr Swag types the full text: addresses, signatures, frequent replies or templates.")}>
  {#snippet actions()}
    <Button variant="primary" icon="plus" onclick={create}>{tr("Nuevo fragmento", "New snippet")}</Button>
  {/snippet}
</PageHeader>

{#if editing}
  <div class="editor">
    <div class="field">
      <span class="lbl">{tr("Frase que dices", "Phrase you say")}</span>
      <Input bind:value={editing.trigger} placeholder={tr("p. ej. mi dirección", "e.g. my address")} autofocus width="100%" />
      {#if conflict}<span class="warn">{tr("Se parece al activador", "Similar to the trigger")} «{conflict.trigger}»</span>{/if}
    </div>
    <div class="field">
      <span class="lbl">{tr("Texto que se escribe", "Text to type")}</span>
      <TextArea bind:value={editing.content} rows={5} placeholder={tr("Calle Mayor 14, 28013 Madrid", "221B Baker Street, London")} />
      <div class="vars">
        <span class="faint">{tr("Variables:", "Variables:")}</span>
        {#each VARS as v}
          <button class="var" onclick={() => editing && (editing.content += v)}>{v}</button>
        {/each}
      </div>
    </div>
    <div class="row">
      <label><Toggle bind:checked={editing.whole_utterance_only} label={tr("Solo si es todo el dictado", "Only as the whole dictation")} /> {tr("Solo si es todo el dictado (evita activaciones dentro de frases)", "Only as the whole dictation (avoids triggering inside sentences)")}</label>
      <div class="btns">
        <Button variant="ghost" onclick={() => ((editing = null), (isNew = false))}>{tr("Cancelar", "Cancel")}</Button>
        <Button variant="primary" onclick={save} disabled={!editing.trigger.trim() || !editing.content.trim()}>{tr("Guardar", "Save")}</Button>
      </div>
    </div>
  </div>
{/if}

{#if app.settings.snippets.length === 0 && !editing}
  <Empty icon="snippet" title={tr("Aún no tienes fragmentos", "No snippets yet")} text={tr("Ejemplo: di «mi correo» y se escribirá tu dirección de correo completa.", "Example: say “my email” and your full email address is typed.")}>
    <Button icon="plus" onclick={create}>{tr("Crear el primero", "Create the first one")}</Button>
  </Empty>
{:else}
  <ul class="grid">
    {#each app.settings.snippets as s (s.id)}
      <li class:off={!s.enabled}>
        <div class="top">
          <span class="trig"><Icon name="mic" size={13} /> «{s.trigger}»</span>
          <Toggle bind:checked={s.enabled} label={tr("Activo", "Active")} />
        </div>
        <pre class="content">{s.content}</pre>
        <div class="foot">
          <span class="faint">{s.whole_utterance_only ? tr("Dictado completo", "Whole dictation") : tr("En cualquier parte", "Anywhere")}</span>
          <div>
            <Button size="sm" variant="ghost" icon="edit" title={tr("Editar", "Edit")} onclick={() => ((editing = structuredClone($state.snapshot(s))), (isNew = false))} />
            <Button size="sm" variant="ghost" icon="trash" title={tr("Eliminar", "Delete")} onclick={() => (app.settings.snippets = app.settings.snippets.filter((x) => x.id !== s.id))} />
          </div>
        </div>
      </li>
    {/each}
  </ul>
{/if}

<style>
  .editor {
    padding: 18px;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border-strong);
    background: var(--surface-2);
    margin-bottom: 22px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .lbl {
    font-size: var(--text-sm);
    font-weight: 540;
  }
  .warn {
    font-size: var(--text-xs);
    color: var(--text-2);
  }
  .vars {
    display: flex;
    gap: 6px;
    align-items: center;
    flex-wrap: wrap;
    font-size: var(--text-xs);
  }
  .var {
    border: 1px solid var(--border-strong);
    background: transparent;
    border-radius: 6px;
    padding: 2px 7px;
    font-family: var(--mono);
    font-size: 11px;
    cursor: pointer;
    color: var(--text-2);
  }
  .var:hover {
    color: var(--text);
    background: var(--hover);
  }
  .row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
  }
  .row label {
    display: flex;
    gap: 10px;
    align-items: center;
    font-size: var(--text-sm);
    color: var(--text-2);
  }
  .btns {
    display: flex;
    gap: 8px;
  }
  .grid {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
    gap: 10px;
  }
  .grid li {
    display: flex;
    flex-direction: column;
    padding: 14px 14px 8px;
    border-radius: var(--radius);
    border: 1px solid var(--border);
    background: var(--surface);
  }
  .grid li.off {
    opacity: 0.55;
  }
  .top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
  }
  .trig {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-weight: 560;
  }
  .content {
    flex: 1;
    margin: 10px 0 8px;
    font-family: var(--font);
    font-size: var(--text-sm);
    color: var(--text-2);
    white-space: pre-wrap;
    max-height: 110px;
    overflow: hidden;
  }
  .foot {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: var(--text-xs);
  }
</style>
