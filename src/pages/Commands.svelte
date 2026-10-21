<script lang="ts">
  import { app, tr } from "../lib/store.svelte";
  import type { CommandAction, VoiceCommand } from "../lib/types";
  import Button from "../components/Button.svelte";
  import Input from "../components/Input.svelte";
  import Kbd from "../components/Kbd.svelte";
  import PageHeader from "../components/PageHeader.svelte";
  import Row from "../components/Row.svelte";
  import Section from "../components/Section.svelte";
  import Select from "../components/Select.svelte";
  import Toggle from "../components/Toggle.svelte";

  const c = $derived(app.settings.commands);

  function actionLabel(a: CommandAction): string {
    switch (a.type) {
      case "delete_last": return tr("Borrar lo último que se dictó", "Delete the last dictation");
      case "undo": return tr("Deshacer (Ctrl+Z)", "Undo (Ctrl+Z)");
      case "press_enter": return tr("Pulsar Intro", "Press Enter");
      case "press_tab": return tr("Pulsar Tab", "Press Tab");
      case "new_line": return tr("Nueva línea", "New line");
      case "new_paragraph": return tr("Nuevo párrafo", "New paragraph");
      case "select_all": return tr("Seleccionar todo (Ctrl+A)", "Select all (Ctrl+A)");
      case "copy": return tr("Copiar (Ctrl+C)", "Copy (Ctrl+C)");
      case "paste": return tr("Pegar (Ctrl+V)", "Paste (Ctrl+V)");
      case "paste_last": return tr("Pegar el último dictado", "Paste the last dictation");
      case "keys": return `${tr("Pulsar", "Press")} ${a.combo}`;
      case "insert_text": return `${tr("Escribir", "Type")} «${a.text}»`;
    }
  }

  // Nuevo comando
  let phrase = $state("");
  let kind = $state<"keys" | "insert_text">("keys");
  let value = $state("Ctrl+S");

  function add() {
    if (!phrase.trim() || !value.trim()) return;
    const action: CommandAction = kind === "keys" ? { type: "keys", combo: value.trim() } : { type: "insert_text", text: value };
    const cmd: VoiceCommand = { id: crypto.randomUUID(), phrases: phrase.split(",").map((s) => s.trim()).filter(Boolean), action, enabled: true, builtin: false };
    app.settings.commands.commands = [...c.commands, cmd];
    phrase = "";
  }
</script>

<PageHeader title={tr("Comandos de voz", "Voice commands")} description={tr("Acciones rápidas por voz. Solo se ejecutan si el dictado completo es la frase del comando, así nunca interfieren con lo que escribes.", "Quick voice actions. They only run if the whole dictation is the command phrase, so they never interfere with your writing.")} />

<Section>
  <Row title={tr("Comandos de voz", "Voice commands")} description={tr("Reconocer las frases de esta página.", "Recognize the phrases on this page.")}>
    <Toggle bind:checked={app.settings.commands.enabled} />
  </Row>
  <Row title={tr("Enviar con «pulsa intro»", "Send with “press enter”")} description={tr("Si terminas un dictado con esa frase, se inserta el texto y se pulsa Intro (útil en chats).", "If you end a dictation with that phrase, the text is inserted and Enter is pressed (handy in chats).")}>
    <Toggle bind:checked={app.settings.commands.send_suffix} />
  </Row>
  {#if c.send_suffix}
    <Row title={tr("Frases de envío", "Send phrases")} description={tr("Separadas por comas.", "Comma-separated.")}>
      <Input value={c.send_phrases.join(", ")} width="300px" onblur={(e) => (app.settings.commands.send_phrases = (e.target as HTMLInputElement).value.split(",").map((s) => s.trim()).filter(Boolean))} />
    </Row>
  {/if}
</Section>

<Section title={tr("Puntuación dictada", "Spoken punctuation")} description={tr("Funciona dentro de cualquier frase si está activada en Ajustes › Dictado.", "Works inside any sentence when enabled in Settings › Dictation.")}>
  <div class="spoken">
    {#each [
      ["nueva línea", "↵"],
      ["punto y aparte", ". ↵↵"],
      ["nuevo párrafo", "↵↵"],
      ["punto y seguido", "."],
      ["punto y coma", ";"],
      ["puntos suspensivos", "…"],
      ["abre / cierra paréntesis", "( )"],
      ["abre / cierra comillas", "\" \""],
      ["abre / cierra interrogación", "¿ ?"],
      ["ana arroba gmail punto com", "ana@gmail.com"],
    ] as [say, gives]}
      <div class="sp"><span class="say">«{say}»</span><span class="gives mono">{gives}</span></div>
    {/each}
  </div>
</Section>

<Section title={tr("Comandos", "Commands")}>
  {#each c.commands as cmd (cmd.id)}
    <Row title={actionLabel(cmd.action)} description={cmd.phrases.map((p) => `«${p}»`).join("  ·  ")}>
      {#if cmd.action.type === "keys"}<Kbd keys={cmd.action.combo.split("+").join(" + ")} size="sm" />{/if}
      <Toggle bind:checked={cmd.enabled} />
      {#if !cmd.builtin}
        <Button size="sm" variant="ghost" icon="trash" title={tr("Eliminar", "Delete")} onclick={() => (app.settings.commands.commands = c.commands.filter((x) => x.id !== cmd.id))} />
      {/if}
    </Row>
  {/each}
</Section>

<Section title={tr("Nuevo comando", "New command")}>
  <div class="new">
    <Input bind:value={phrase} placeholder={tr("Frase(s): guarda el documento, guardar", "Phrase(s): save the document")} width="100%" />
    <Select bind:value={kind} width="180px" options={[
      { value: "keys", label: tr("Pulsar teclas", "Press keys") },
      { value: "insert_text", label: tr("Escribir texto", "Type text") },
    ]} onchange={(k) => (value = k === "keys" ? "Ctrl+S" : "")} />
    <Input bind:value placeholder={kind === "keys" ? "Ctrl+Shift+S" : tr("Texto", "Text")} mono={kind === "keys"} width="100%" />
    <Button variant="primary" icon="plus" onclick={add} disabled={!phrase.trim() || !value.trim()}>{tr("Añadir", "Add")}</Button>
  </div>
  <p class="hint faint">{tr("Formato de teclas: Ctrl, Alt, Shift, Win y una tecla (A–Z, 0–9, F1–F24, Enter, Tab, Esc, Space…).", "Key format: Ctrl, Alt, Shift, Win and one key (A–Z, 0–9, F1–F24, Enter, Tab, Esc, Space…).")}</p>
</Section>

<style>
  .spoken {
    display: grid;
    grid-template-columns: 1fr 1fr;
  }
  .sp {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 18px;
    font-size: var(--text-sm);
    border-bottom: 1px solid var(--border);
  }
  .sp:nth-child(odd) {
    border-right: 1px solid var(--border);
  }
  .say {
    color: var(--text-2);
  }
  .gives {
    color: var(--text);
  }
  .new {
    display: grid;
    grid-template-columns: 1.4fr auto 1fr auto;
    gap: 8px;
    padding: 14px 18px 4px;
  }
  .hint {
    padding: 4px 18px 14px;
    font-size: var(--text-xs);
  }
</style>
