<script lang="ts">
  import { onMount } from "svelte";
  import { api, errorText, on, saveDialog } from "../lib/api";
  import { appName, duration, number, relativeDate } from "../lib/format";
  import { router } from "../lib/router.svelte";
  import { app, tr } from "../lib/store.svelte";
  import type { AiAction, HistoryEntry } from "../lib/types";
  import Button from "../components/Button.svelte";
  import Dialog from "../components/Dialog.svelte";
  import Empty from "../components/Empty.svelte";
  import Icon from "../components/Icon.svelte";
  import Input from "../components/Input.svelte";
  import Menu, { type MenuItem } from "../components/Menu.svelte";
  import PageHeader from "../components/PageHeader.svelte";
  import Segmented from "../components/Segmented.svelte";
  import Select from "../components/Select.svelte";
  import TextArea from "../components/TextArea.svelte";
  import Toggle from "../components/Toggle.svelte";

  const PAGE = 40;
  let entries = $state<HistoryEntry[]>([]);
  let total = $state(0);
  let search = $state("");
  let filter = $state<"all" | "fav">("all");
  let oldest = $state(false);
  let loading = $state(false);
  let expanded = $state<number | null>(null);
  let editing = $state<number | null>(null);
  let draft = $state("");
  let busy = $state<number | null>(null);
  let clearOpen = $state(false);
  let keepFav = $state(true);
  let exportFormat = $state("txt");
  let timer: ReturnType<typeof setTimeout> | undefined;

  async function load(reset = true) {
    loading = true;
    try {
      const page = await api.historyQuery({
        search,
        favorites_only: filter === "fav",
        oldest_first: oldest,
        limit: PAGE,
        offset: reset ? 0 : entries.length,
      });
      entries = reset ? page.entries : [...entries, ...page.entries];
      total = page.total;
    } catch (e) {
      app.toast(errorText(e), "error");
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    load();
    const off = on("history-changed", () => editing === null && load());
    return () => off.then((f) => f());
  });

  function onSearch() {
    clearTimeout(timer);
    timer = setTimeout(() => load(), 220);
  }

  async function toggleFav(e: HistoryEntry) {
    await api.historyFavorite(e.id, !e.favorite);
    e.favorite = !e.favorite;
    if (filter === "fav" && !e.favorite) load();
  }

  async function saveEdit(e: HistoryEntry) {
    await api.historyUpdate(e.id, draft);
    e.final_text = draft;
    e.edited = true;
    editing = null;
    app.toast(tr("Cambios guardados", "Changes saved"), "success");
  }

  async function remove(e: HistoryEntry) {
    await api.historyDelete(e.id);
    entries = entries.filter((x) => x.id !== e.id);
    total -= 1;
  }

  async function reprocess(e: HistoryEntry, action: AiAction) {
    busy = e.id;
    try {
      const r = await api.historyReprocess(e.id, action);
      if (r.entry) {
        Object.assign(e, r.entry);
      }
      app.toast(r.ai_used ? tr("Reprocesado con IA", "Reprocessed with AI") : tr("Reprocesado con las reglas locales", "Reprocessed with local rules"), "success");
    } catch (err) {
      app.toast(errorText(err), "error");
    } finally {
      busy = null;
    }
  }

  function reprocessItems(e: HistoryEntry): MenuItem[] {
    const modes: MenuItem[] = app.settings.writing.modes.map((m) => ({
      label: m.name,
      icon: m.id === e.mode ? "check" : undefined,
      action: () => reprocess(e, { type: "mode", mode_id: m.id }),
    }));
    const aiReady = app.settings.ai.enabled && app.settings.ai.consent_given;
    const ai: MenuItem[] = [
      { label: tr("Corregir gramática", "Fix grammar"), icon: "check", action: () => reprocess(e, { type: "fix_grammar" }), separator: true, disabled: !aiReady },
      { label: tr("Resumir", "Summarize"), icon: "list", action: () => reprocess(e, { type: "summarize" }), disabled: !aiReady },
      { label: tr("Versión profesional", "Professional version"), icon: "mail", action: () => reprocess(e, { type: "professional" }), disabled: !aiReady },
      { label: tr("Estructurar ideas", "Structure ideas"), icon: "modes", action: () => reprocess(e, { type: "structure" }), disabled: !aiReady },
      { label: tr("Lista de tareas", "Task list"), icon: "check", action: () => reprocess(e, { type: "tasks" }), disabled: !aiReady },
      { label: tr("Tono más amable", "Friendlier tone"), icon: "sparkle", action: () => reprocess(e, { type: "tone", tone: tr("amable y cercano", "friendly and warm") }), disabled: !aiReady },
      { label: tr("Traducir al inglés", "Translate to Spanish"), icon: "language", action: () => reprocess(e, { type: "translate", language: tr("inglés", "Spanish") }), disabled: !aiReady },
    ];
    return [...modes, ...ai];
  }

  async function exportAll() {
    const ext = exportFormat === "markdown" ? "md" : exportFormat;
    const path = await saveDialog(`dictados.${ext}`, [{ name: exportFormat.toUpperCase(), extensions: [ext] }]);
    if (!path) return;
    try {
      await api.historyExport([], exportFormat, path);
      app.toast(tr("Historial exportado", "History exported"), "success");
    } catch (e) {
      app.toast(errorText(e), "error");
    }
  }

  async function clearAll() {
    await api.historyClear(keepFav);
    clearOpen = false;
    load();
    app.toast(tr("Historial borrado", "History cleared"), "success");
  }

  function modeName(id: string) {
    return app.settings.writing.modes.find((m) => m.id === id)?.name ?? id;
  }
</script>

<PageHeader title={tr("Historial", "History")} description={tr("Tus dictados se guardan solo en este equipo.", "Your dictations are stored only on this computer.")}>
  {#snippet actions()}
    <Select bind:value={exportFormat} width="120px" label={tr("Formato", "Format")} options={[
      { value: "txt", label: "Texto (.txt)" },
      { value: "markdown", label: "Markdown" },
      { value: "csv", label: "CSV" },
      { value: "json", label: "JSON" },
    ]} />
    <Button icon="download" onclick={exportAll} disabled={total === 0}>{tr("Exportar", "Export")}</Button>
    <Button icon="trash" variant="ghost" title={tr("Borrar historial", "Clear history")} onclick={() => (clearOpen = true)} disabled={total === 0} />
  {/snippet}
</PageHeader>

{#if !app.settings.privacy.save_history}
  <div class="note">
    <Icon name="lock" size={15} />
    <span>{tr("El guardado del historial está desactivado. Los dictados nuevos no se guardan.", "History saving is off. New dictations are not stored.")}</span>
    <Button size="sm" variant="ghost" onclick={() => router.go("settings/privacy")}>{tr("Cambiar", "Change")}</Button>
  </div>
{/if}

<div class="toolbar">
  <Input bind:value={search} icon="search" placeholder={tr("Buscar en tus dictados…", "Search your dictations…")} oninput={onSearch} width="100%" />
  <Segmented bind:value={filter} onchange={() => load()} options={[
    { value: "all", label: tr("Todos", "All") },
    { value: "fav", label: tr("Favoritos", "Favorites"), icon: "star" },
  ]} />
  <Button variant="ghost" icon="sort" title={oldest ? tr("Más antiguos primero", "Oldest first") : tr("Más recientes primero", "Newest first")} onclick={() => ((oldest = !oldest), load())}>
    {oldest ? tr("Antiguos", "Oldest") : tr("Recientes", "Newest")}
  </Button>
</div>

{#if entries.length === 0 && !loading}
  <Empty icon="history" title={search ? tr("Sin resultados", "No results") : tr("Aún no hay dictados", "No dictations yet")} text={search ? tr("Prueba con otras palabras; la búsqueda ignora tildes y mayúsculas.", "Try other words; search ignores accents and case.") : tr("Pulsa el atajo en cualquier aplicación y empieza a hablar.", "Press the shortcut in any app and start talking.")} />
{:else}
  <p class="count faint num">{number(total)} {total === 1 ? tr("dictado", "dictation") : tr("dictados", "dictations")}</p>
  <ul class="list">
    {#each entries as e (e.id)}
      <li class:open={expanded === e.id}>
        <div class="head">
          <button class="star" class:on={e.favorite} onclick={() => toggleFav(e)} aria-label={tr("Favorito", "Favorite")} title={tr("Favorito", "Favorite")}>
            <Icon name="star" size={15} fill={e.favorite} />
          </button>
          <button class="main" onclick={() => (expanded = expanded === e.id ? null : e.id)}>
            {#if editing === e.id}
              <span class="txt">{tr("Editando…", "Editing…")}</span>
            {:else}
              <span class="txt" class:clamp={expanded !== e.id}>{e.final_text}</span>
            {/if}
            <span class="meta faint">
              {relativeDate(e.created_at)}
              {#if e.app}· {appName(e.app)}{/if}
              · {modeName(e.mode)}
              {#if e.ai_used}· IA{/if}
              {#if e.edited}· {tr("editado", "edited")}{/if}
            </span>
          </button>
          <div class="acts">
            {#if busy === e.id}<span class="spin"></span>{/if}
            <Button size="sm" variant="ghost" icon="copy" title={tr("Copiar", "Copy")} onclick={() => api.historyCopy(e.id).then(() => app.toast(tr("Copiado", "Copied"), "success"))} />
            <Menu icon="sparkle" title={tr("Reprocesar", "Reprocess")} items={reprocessItems(e)} />
            <Menu
              items={[
                { label: tr("Editar", "Edit"), icon: "edit", action: () => ((editing = e.id), (draft = e.final_text), (expanded = e.id)) },
                { label: tr("Volver a usar (Alt+Shift+Z lo pega)", "Reuse (Alt+Shift+Z pastes it)"), icon: "refresh", action: () => api.historyReuse(e.id).then(() => app.toast(tr("Listo para pegar con el atajo «Pegar último»", "Ready to paste with the “Paste last” shortcut"), "success")) },
                { label: tr("Eliminar", "Delete"), icon: "trash", action: () => remove(e), separator: true },
              ]}
            />
          </div>
        </div>
        {#if expanded === e.id}
          <div class="detail">
            {#if editing === e.id}
              <TextArea bind:value={draft} rows={5} label={tr("Texto", "Text")} />
              <div class="edit-acts">
                <Button size="sm" variant="ghost" onclick={() => (editing = null)}>{tr("Cancelar", "Cancel")}</Button>
                <Button size="sm" variant="primary" onclick={() => saveEdit(e)}>{tr("Guardar", "Save")}</Button>
              </div>
            {:else}
              <dl>
                <dt>{tr("Duración", "Duration")}</dt><dd class="num">{duration(e.duration_ms)}</dd>
                <dt>{tr("Palabras", "Words")}</dt><dd class="num">{e.words}</dd>
                <dt>{tr("Motor", "Engine")}</dt><dd>{e.engine === "local" ? tr("Local", "Local") : e.engine} {e.model ? `· ${e.model}` : ""}</dd>
                <dt>{tr("Procesado en", "Processed in")}</dt><dd class="num">{(e.processing_ms / 1000).toFixed(1)} s</dd>
              </dl>
              {#if e.raw_text && e.raw_text !== e.final_text}
                <div class="raw">
                  <div class="rl faint">{tr("Transcripción original", "Original transcript")}</div>
                  <div class="rt">{e.raw_text}</div>
                </div>
              {/if}
            {/if}
          </div>
        {/if}
      </li>
    {/each}
  </ul>
  {#if entries.length < total}
    <div class="more"><Button variant="outline" loading={loading} onclick={() => load(false)}>{tr("Cargar más", "Load more")}</Button></div>
  {/if}
{/if}

<Dialog bind:open={clearOpen} title={tr("Borrar el historial", "Clear history")}>
  <p>{tr("Se eliminarán los dictados guardados en este equipo y su audio, si lo hubiera. Las estadísticas se conservan.", "Saved dictations on this computer and their audio, if any, will be deleted. Statistics are kept.")}</p>
  <label class="keep"><Toggle bind:checked={keepFav} label={tr("Conservar favoritos", "Keep favorites")} /> {tr("Conservar favoritos", "Keep favorites")}</label>
  {#snippet actions()}
    <Button variant="ghost" onclick={() => (clearOpen = false)}>{tr("Cancelar", "Cancel")}</Button>
    <Button variant="primary" onclick={clearAll}>{tr("Borrar", "Delete")}</Button>
  {/snippet}
</Dialog>

<style>
  .note {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px 10px 14px;
    margin-bottom: 16px;
    border-radius: var(--radius);
    border: 1px dashed var(--border-strong);
    font-size: var(--text-sm);
    color: var(--text-2);
  }
  .note span {
    flex: 1;
  }
  .toolbar {
    display: grid;
    grid-template-columns: 1fr auto auto;
    gap: 8px;
    align-items: center;
    margin-bottom: 14px;
  }
  .count {
    font-size: var(--text-xs);
    margin: 0 2px 8px;
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  li + li {
    border-top: 1px solid var(--border);
  }
  li.open {
    background: var(--surface-2);
  }
  .head {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    padding: 10px 10px 10px 8px;
  }
  .star {
    border: 0;
    background: transparent;
    color: var(--text-3);
    padding: 6px;
    border-radius: 6px;
    cursor: pointer;
  }
  .star:hover {
    color: var(--text);
    background: var(--hover);
  }
  .star.on {
    color: var(--text);
  }
  .main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
    border: 0;
    background: transparent;
    text-align: left;
    padding: 4px 4px;
    cursor: pointer;
  }
  .txt {
    font-size: var(--text-md);
    white-space: pre-wrap;
    line-height: 1.5;
    user-select: text;
  }
  .txt.clamp {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .meta {
    font-size: var(--text-xs);
  }
  .acts {
    display: flex;
    align-items: center;
    gap: 2px;
    padding-top: 2px;
  }
  .spin {
    width: 13px;
    height: 13px;
    margin: 0 6px;
    border-radius: 50%;
    border: 1.6px solid var(--text-2);
    border-right-color: transparent;
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .detail {
    padding: 0 16px 14px 46px;
  }
  dl {
    display: grid;
    grid-template-columns: max-content 1fr max-content 1fr;
    gap: 4px 14px;
    margin: 4px 0 0;
    font-size: var(--text-sm);
  }
  dt {
    color: var(--text-3);
  }
  dd {
    margin: 0;
    color: var(--text-2);
  }
  .raw {
    margin-top: 12px;
    padding: 10px 12px;
    border-radius: var(--radius-sm);
    background: var(--surface);
    border: 1px solid var(--border);
  }
  .rl {
    font-size: var(--text-xs);
    margin-bottom: 3px;
  }
  .rt {
    font-size: var(--text-sm);
    color: var(--text-2);
    user-select: text;
  }
  .edit-acts {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
    margin-top: 8px;
  }
  .more {
    display: flex;
    justify-content: center;
    margin-top: 16px;
  }
  .keep {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 14px;
    color: var(--text);
  }
</style>
