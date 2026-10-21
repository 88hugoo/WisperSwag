<script lang="ts">
  import { api, errorText } from "../lib/api";
  import { appName } from "../lib/format";
  import { app, tr } from "../lib/store.svelte";
  import type { AppCategory, AppProfile } from "../lib/types";
  import Button from "../components/Button.svelte";
  import Empty from "../components/Empty.svelte";
  import Icon from "../components/Icon.svelte";
  import Input from "../components/Input.svelte";
  import PageHeader from "../components/PageHeader.svelte";
  import Row from "../components/Row.svelte";
  import Section from "../components/Section.svelte";
  import Select from "../components/Select.svelte";
  import TextArea from "../components/TextArea.svelte";
  import Toggle from "../components/Toggle.svelte";

  const categories: { id: AppCategory; icon: string; es: string; en: string; ex: string }[] = [
    { id: "code", icon: "code", es: "Editores de código", en: "Code editors", ex: "VS Code, Cursor, JetBrains, Visual Studio" },
    { id: "terminal", icon: "command", es: "Terminales", en: "Terminals", ex: "Windows Terminal, PowerShell, cmd" },
    { id: "email", icon: "mail", es: "Correo", en: "Email", ex: "Outlook, Thunderbird" },
    { id: "chat", icon: "message", es: "Mensajería", en: "Messaging", ex: "Slack, Teams, WhatsApp, Discord, Telegram" },
    { id: "browser", icon: "globe", es: "Navegadores", en: "Browsers", ex: "Chrome, Edge, Firefox, Brave" },
    { id: "documents", icon: "file", es: "Documentos", en: "Documents", ex: "Word, LibreOffice, PowerPoint" },
    { id: "notes", icon: "note", es: "Notas", en: "Notes", ex: "Obsidian, Notion, OneNote, Bloc de notas" },
    { id: "other", icon: "apps", es: "Otras aplicaciones", en: "Other apps", ex: "" },
  ];

  const modeOptions = $derived([
    { value: "", label: tr("Modo predeterminado", "Default mode") },
    ...app.settings.writing.modes.map((m) => ({ value: m.id, label: m.name })),
  ]);
  const methodOptions = $derived([
    { value: null, label: tr("Según ajustes", "Use settings") },
    { value: "paste", label: tr("Pegar (Ctrl+V)", "Paste (Ctrl+V)") },
    { value: "paste_shift_insert", label: tr("Pegar (Mayús+Insert)", "Paste (Shift+Insert)") },
    { value: "type", label: tr("Escribir carácter a carácter", "Type character by character") },
    { value: "clipboard_only", label: tr("Solo copiar", "Copy only") },
  ]);

  let editing = $state<AppProfile | null>(null);
  let isNew = $state(false);
  let exeText = $state("");
  let titleText = $state("");
  let detecting = $state(0);

  function getCat(c: AppCategory) {
    return app.settings.apps.category_modes[c] ?? "";
  }

  function setCat(c: AppCategory, v: string) {
    const m = { ...app.settings.apps.category_modes };
    if (v) m[c] = v;
    else delete m[c];
    app.settings.apps.category_modes = m;
  }

  function create() {
    editing = { id: crypto.randomUUID(), name: "", exe: [], title_contains: [], mode: null, insert_method: null, ai_context: "", enabled: true };
    exeText = titleText = "";
    isNew = true;
  }

  function edit(p: AppProfile) {
    editing = structuredClone($state.snapshot(p)) as AppProfile;
    exeText = p.exe.join(", ");
    titleText = p.title_contains.join(", ");
    isNew = false;
  }

  function save() {
    if (!editing) return;
    const p = $state.snapshot(editing) as AppProfile;
    p.exe = exeText.split(",").map((s) => s.trim().toLowerCase()).filter(Boolean);
    p.title_contains = titleText.split(",").map((s) => s.trim()).filter(Boolean);
    if (!p.name.trim()) p.name = p.exe[0] ? appName(p.exe[0]) : tr("Perfil", "Profile");
    if (p.exe.length === 0 && p.title_contains.length === 0) {
      app.toast(tr("Indica al menos un ejecutable o texto del título", "Add at least one executable or title text"), "error");
      return;
    }
    app.settings.apps.profiles = isNew ? [...app.settings.apps.profiles, p] : app.settings.apps.profiles.map((x) => (x.id === p.id ? p : x));
    editing = null;
  }

  async function detect() {
    detecting = 3;
    const timer = setInterval(() => (detecting = Math.max(0, detecting - 1)), 1000);
    try {
      const exe = await api.foregroundIn(3);
      exeText = exeText ? `${exeText}, ${exe}` : exe;
      if (editing && !editing.name) editing.name = appName(exe);
    } catch (e) {
      app.toast(errorText(e), "error");
    } finally {
      clearInterval(timer);
      detecting = 0;
    }
  }
</script>

<PageHeader title={tr("Aplicaciones", "Apps")} description={tr("Adapta el estilo del dictado a la aplicación en la que escribes.", "Adapt your dictation style to the app you're writing in.")}>
  {#snippet actions()}
    <Button variant="primary" icon="plus" onclick={create}>{tr("Nuevo perfil", "New profile")}</Button>
  {/snippet}
</PageHeader>

<Section>
  <Row title={tr("Contexto de aplicación", "App context")} description={tr("Detecta la aplicación activa (solo el nombre del ejecutable) para elegir el modo de escritura.", "Detects the active app (executable name only) to pick the writing mode.")}>
    <Toggle bind:checked={app.settings.apps.enabled} />
  </Row>
  <Row title={tr("Reconocer sitios web por el título", "Recognize websites by title")} description={tr("Permite distinguir Gmail o Slack dentro del navegador. El título se procesa en memoria y nunca se guarda ni se envía.", "Lets Wispr Swag tell Gmail or Slack apart inside the browser. Titles are processed in memory and never stored or sent.")} disabled={!app.settings.apps.enabled}>
    <Toggle bind:checked={app.settings.apps.use_window_titles} />
  </Row>
</Section>

{#if editing}
  <div class="editor">
    <div class="grid">
      <label class="f"><span>{tr("Nombre", "Name")}</span><Input bind:value={editing.name} placeholder="Slack" width="100%" /></label>
      <label class="f">
        <span>{tr("Ejecutables", "Executables")}</span>
        <div class="with-btn">
          <Input bind:value={exeText} placeholder="slack.exe, teams.exe" mono width="100%" />
          <Button onclick={detect} loading={detecting > 0} icon="target">{detecting ? `${detecting}…` : tr("Detectar", "Detect")}</Button>
        </div>
        {#if detecting}<em class="faint hint">{tr("Cambia a la aplicación ahora…", "Switch to the app now…")}</em>{/if}
      </label>
      <label class="f"><span>{tr("El título contiene (opcional)", "Title contains (optional)")}</span><Input bind:value={titleText} placeholder="Gmail" width="100%" /></label>
      <label class="f"><span>{tr("Modo", "Mode")}</span>
        <Select value={editing.mode ?? ""} onchange={(v) => editing && (editing.mode = v || null)} options={modeOptions} width="100%" /></label>
      <label class="f"><span>{tr("Inserción", "Insertion")}</span><Select bind:value={editing.insert_method} options={methodOptions} width="100%" /></label>
    </div>
    <label class="f full"><span>{tr("Indicaciones para la IA en esta aplicación (opcional)", "AI hints for this app (optional)")}</span>
      <TextArea bind:value={editing.ai_context} rows={2} placeholder={tr("p. ej. Canal interno del equipo: tono directo, sin saludos.", "e.g. Internal team channel: direct tone, no greetings.")} /></label>
    <div class="ed-acts">
      <Button variant="ghost" onclick={() => (editing = null)}>{tr("Cancelar", "Cancel")}</Button>
      <Button variant="primary" onclick={save}>{tr("Guardar perfil", "Save profile")}</Button>
    </div>
  </div>
{/if}

<Section title={tr("Perfiles personalizados", "Custom profiles")} description={tr("Tienen prioridad sobre las categorías.", "They take priority over categories.")}>
  {#if app.settings.apps.profiles.length === 0}
    <Empty icon="apps" title={tr("Sin perfiles", "No profiles")} text={tr("Crea uno para usar un modo o método de inserción concreto en una aplicación.", "Create one to use a specific mode or insertion method in an app.")} />
  {:else}
    {#each app.settings.apps.profiles as p (p.id)}
      <Row title={p.name} description={[p.exe.join(", "), p.title_contains.length ? `${tr("título", "title")}: ${p.title_contains.join(", ")}` : "", p.mode ? `${tr("modo", "mode")}: ${app.settings.writing.modes.find((m) => m.id === p.mode)?.name ?? p.mode}` : ""].filter(Boolean).join(" · ")}>
        <Toggle bind:checked={p.enabled} />
        <Button size="sm" variant="ghost" icon="edit" title={tr("Editar", "Edit")} onclick={() => edit(p)} />
        <Button size="sm" variant="ghost" icon="trash" title={tr("Eliminar", "Delete")} onclick={() => (app.settings.apps.profiles = app.settings.apps.profiles.filter((x) => x.id !== p.id))} />
      </Row>
    {/each}
  {/if}
</Section>

<Section title={tr("Modo por categoría", "Mode by category")} description={tr("Se aplica a las aplicaciones conocidas de cada tipo.", "Applies to known apps of each type.")}>
  {#each categories as c}
    <Row title={tr(c.es, c.en)} description={c.ex}>
      <span class="ic"><Icon name={c.icon} size={15} /></span>
      <Select value={getCat(c.id)} onchange={(v) => setCat(c.id, v)} options={modeOptions} width="210px" />
    </Row>
  {/each}
</Section>

<style>
  .editor {
    padding: 18px;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border-strong);
    background: var(--surface-2);
    margin-bottom: 26px;
  }
  .grid {
    display: grid;
    grid-template-columns: 1fr 1.4fr;
    gap: 12px 16px;
  }
  .f {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: var(--text-sm);
    font-weight: 520;
  }
  .f.full {
    margin-top: 12px;
  }
  .with-btn {
    display: flex;
    gap: 6px;
  }
  .hint {
    font-size: var(--text-xs);
    font-weight: 400;
  }
  .ed-acts {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 14px;
  }
  .ic {
    color: var(--text-3);
  }
</style>
