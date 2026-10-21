<script lang="ts">
  import { api, errorText, openDialog, saveDialog } from "../lib/api";
  import { app, tr } from "../lib/store.svelte";
  import type { Replacement, VocabEntry } from "../lib/types";
  import Badge from "../components/Badge.svelte";
  import Button from "../components/Button.svelte";
  import Empty from "../components/Empty.svelte";
  import Icon from "../components/Icon.svelte";
  import Input from "../components/Input.svelte";
  import PageHeader from "../components/PageHeader.svelte";
  import Row from "../components/Row.svelte";
  import Section from "../components/Section.svelte";
  import Segmented from "../components/Segmented.svelte";
  import Select from "../components/Select.svelte";
  import Toggle from "../components/Toggle.svelte";

  const v = $derived(app.settings.vocabulary);
  let tab = $state<"terms" | "replacements">("terms");
  let listFilter = $state<string>("*");
  let query = $state("");

  // Formulario de término
  let term = $state("");
  let variants = $state("");
  let termList = $state("General");
  let note = $state("");
  // Formulario de sustitución
  let from = $state("");
  let to = $state("");
  let wholeWord = $state(true);
  let caseSensitive = $state(false);
  let isRegex = $state(false);
  let regexError = $state<string | null>(null);
  let repList = $state("General");
  let newList = $state("");
  let testInput = $state("");
  let testOutput = $state("");
  let editingId = $state<string | null>(null);

  const uid = () => crypto.randomUUID();
  const listOptions = $derived(v.lists.map((l) => ({ value: l, label: l })));

  const terms = $derived(
    v.entries.filter(
      (e) =>
        (listFilter === "*" || e.list === listFilter) &&
        (!query || (e.term + " " + e.variants.join(" ")).toLowerCase().includes(query.toLowerCase())),
    ),
  );
  const reps = $derived(
    v.replacements.filter(
      (r) => (listFilter === "*" || r.list === listFilter) && (!query || (r.from + " " + r.to).toLowerCase().includes(query.toLowerCase())),
    ),
  );

  function addTerm() {
    const t = term.trim();
    if (!t) return;
    const vars = variants.split(",").map((s) => s.trim()).filter(Boolean);
    if (editingId) {
      const e = app.settings.vocabulary.entries.find((x) => x.id === editingId);
      if (e) Object.assign(e, { term: t, variants: vars, list: termList, note });
      editingId = null;
    } else {
      if (v.entries.some((e) => e.term.toLowerCase() === t.toLowerCase())) {
        app.toast(tr("Ese término ya existe", "That term already exists"), "error");
        return;
      }
      app.settings.vocabulary.entries = [{ id: uid(), term: t, variants: vars, list: termList, enabled: true, note }, ...v.entries];
    }
    term = variants = note = "";
  }

  function editTerm(e: VocabEntry) {
    editingId = e.id;
    term = e.term;
    variants = e.variants.join(", ");
    termList = e.list;
    note = e.note;
  }

  async function addReplacement() {
    if (!from.trim()) return;
    if (isRegex) {
      try {
        await api.validateRegex(from);
        regexError = null;
      } catch (e) {
        regexError = errorText(e);
        return;
      }
    }
    const r: Replacement = { id: uid(), from: from.trim(), to, whole_word: wholeWord, case_sensitive: caseSensitive, regex: isRegex, list: repList, enabled: true };
    app.settings.vocabulary.replacements = [r, ...v.replacements];
    from = to = "";
  }

  function toggleList(l: string) {
    const d = app.settings.vocabulary.disabled_lists;
    app.settings.vocabulary.disabled_lists = d.includes(l) ? d.filter((x) => x !== l) : [...d, l];
  }

  function addList() {
    const l = newList.trim();
    if (l && !v.lists.includes(l)) app.settings.vocabulary.lists = [...v.lists, l];
    newList = "";
  }

  function removeList(l: string) {
    if (["General"].includes(l)) return;
    if (v.entries.some((e) => e.list === l) || v.replacements.some((r) => r.list === l)) {
      app.toast(tr("La lista no está vacía", "The list is not empty"), "error");
      return;
    }
    app.settings.vocabulary.lists = v.lists.filter((x) => x !== l);
    if (listFilter === l) listFilter = "*";
  }

  async function importFile() {
    const path = await openDialog({ filters: [{ name: tr("Vocabulario", "Vocabulary"), extensions: ["json", "csv", "txt"] }] });
    if (!path) return;
    try {
      await app.saveNow();
      const r = await api.vocabImport(path, listFilter === "*" ? "General" : listFilter);
      await app.reload();
      app.toast(tr(`Importados ${r.entries} términos y ${r.replacements} sustituciones`, `Imported ${r.entries} terms and ${r.replacements} replacements`), "success");
    } catch (e) {
      app.toast(errorText(e), "error");
    }
  }

  async function exportFile(format: string) {
    const path = await saveDialog(`vocabulario.${format}`, [{ name: format.toUpperCase(), extensions: [format] }]);
    if (!path) return;
    try {
      await app.saveNow();
      await api.vocabExport(path, format);
      app.toast(tr("Vocabulario exportado", "Vocabulary exported"), "success");
    } catch (e) {
      app.toast(errorText(e), "error");
    }
  }

  async function test() {
    await app.saveNow();
    const r = await api.processText(testInput, "natural", false);
    testOutput = r.text;
  }
</script>

<PageHeader title={tr("Vocabulario", "Vocabulary")} description={tr("Nombres, términos técnicos y sustituciones que Wispr Swag debe escribir siempre bien.", "Names, technical terms and replacements Wispr Swag should always get right.")}>
  {#snippet actions()}
    <Button icon="upload" onclick={importFile}>{tr("Importar", "Import")}</Button>
    <Button icon="download" onclick={() => exportFile("csv")}>CSV</Button>
    <Button icon="download" onclick={() => exportFile("json")}>JSON</Button>
  {/snippet}
</PageHeader>

<div class="honest">
  <Icon name="info" size={15} />
  <p>
    {tr(
      "Añadir un término no reentrena el modelo acústico. Wispr Swag corrige las variantes que indiques y las parecidas por pronunciación, unifica la ortografía exacta y, con motores en la nube o IA, lo envía como pista.",
      "Adding a term doesn't retrain the acoustic model. Wispr Swag fixes the variants you list and similar-sounding ones, enforces the exact spelling and, with cloud engines or AI, sends it as a hint.",
    )}
  </p>
</div>

<div class="lists">
  <button class="chip" class:active={listFilter === "*"} onclick={() => (listFilter = "*")}>{tr("Todas", "All")} <span class="n">{v.entries.length + v.replacements.length}</span></button>
  {#each v.lists as l}
    {@const disabled = v.disabled_lists.includes(l)}
    <span class="chip-wrap" class:disabled>
      <button class="chip" class:active={listFilter === l} onclick={() => (listFilter = l)}>
        {l} <span class="n">{v.entries.filter((e) => e.list === l).length + v.replacements.filter((r) => r.list === l).length}</span>
      </button>
      <button class="mini" title={disabled ? tr("Activar lista", "Enable list") : tr("Desactivar lista", "Disable list")} onclick={() => toggleList(l)}>
        <Icon name={disabled ? "eye-off" : "eye"} size={12} />
      </button>
      {#if l !== "General"}<button class="mini" title={tr("Eliminar lista", "Delete list")} onclick={() => removeList(l)}><Icon name="x" size={12} /></button>{/if}
    </span>
  {/each}
  <span class="new-list">
    <Input bind:value={newList} placeholder={tr("Nueva lista", "New list")} width="130px" onkeydown={(e) => e.key === "Enter" && addList()} />
  </span>
</div>

<div class="bar">
  <Segmented bind:value={tab} options={[
    { value: "terms", label: `${tr("Términos", "Terms")} · ${v.entries.length}` },
    { value: "replacements", label: `${tr("Sustituciones", "Replacements")} · ${v.replacements.length}` },
  ]} />
  <Input bind:value={query} icon="search" placeholder={tr("Filtrar", "Filter")} width="220px" />
</div>

{#if tab === "terms"}
  <div class="form">
    <Input bind:value={term} placeholder={tr("Término exacto (p. ej. Kubernetes)", "Exact term (e.g. Kubernetes)")} onkeydown={(e) => e.key === "Enter" && addTerm()} />
    <Input bind:value={variants} placeholder={tr("Variantes mal reconocidas, separadas por comas", "Misrecognized variants, comma-separated")} onkeydown={(e) => e.key === "Enter" && addTerm()} />
    <Select bind:value={termList} options={listOptions} width="150px" label={tr("Lista", "List")} />
    <Button variant="primary" icon={editingId ? "check" : "plus"} onclick={addTerm} disabled={!term.trim()}>{editingId ? tr("Guardar", "Save") : tr("Añadir", "Add")}</Button>
  </div>
  {#if terms.length === 0}
    <Empty icon="book" title={tr("Sin términos", "No terms")} text={tr("Añade nombres propios, marcas, siglas o jerga de tu trabajo.", "Add proper names, brands, acronyms or work jargon.")} />
  {:else}
    <ul class="table">
      {#each terms as e (e.id)}
        <li class:off={!e.enabled}>
          <div class="cell">
            <div class="t">{e.term}</div>
            {#if e.variants.length}<div class="sub faint">{tr("Corrige", "Fixes")}: {e.variants.join(", ")}</div>{/if}
            {#if e.note}<div class="sub faint">{e.note}</div>{/if}
          </div>
          <Badge>{e.list}</Badge>
          <Toggle bind:checked={e.enabled} label={tr("Activo", "Active")} />
          <Button size="sm" variant="ghost" icon="edit" title={tr("Editar", "Edit")} onclick={() => editTerm(e)} />
          <Button size="sm" variant="ghost" icon="trash" title={tr("Eliminar", "Delete")} onclick={() => (app.settings.vocabulary.entries = v.entries.filter((x) => x.id !== e.id))} />
        </li>
      {/each}
    </ul>
  {/if}
{:else}
  <div class="form rep">
    <Input bind:value={from} placeholder={isRegex ? tr("Expresión regular", "Regular expression") : tr("Cuando se diga…", "When I say…")} mono={isRegex} invalid={!!regexError} onkeydown={(e) => e.key === "Enter" && addReplacement()} />
    <Icon name="arrow" size={15} />
    <Input bind:value={to} placeholder={tr("…escribir", "…write")} onkeydown={(e) => e.key === "Enter" && addReplacement()} />
    <Select bind:value={repList} options={listOptions} width="140px" label={tr("Lista", "List")} />
    <Button variant="primary" icon="plus" onclick={addReplacement} disabled={!from.trim()}>{tr("Añadir", "Add")}</Button>
  </div>
  <div class="opts">
    <label><Toggle bind:checked={wholeWord} label={tr("Palabra completa", "Whole word")} /> {tr("Palabra completa", "Whole word")}</label>
    <label><Toggle bind:checked={caseSensitive} label={tr("Distinguir mayúsculas", "Case sensitive")} /> {tr("Distinguir mayúsculas", "Case sensitive")}</label>
    <label><Toggle bind:checked={isRegex} label={tr("Expresión regular", "Regex")} /> {tr("Expresión regular", "Regex")}</label>
    {#if regexError}<span class="err">{regexError}</span>{/if}
  </div>
  {#if reps.length === 0}
    <Empty icon="refresh" title={tr("Sin sustituciones", "No replacements")} text={tr("Convierte expresiones habladas en el texto exacto: «arroba empresa» → «@empresa».", "Turn spoken expressions into exact text.")} />
  {:else}
    <ul class="table">
      {#each reps as r (r.id)}
        <li class:off={!r.enabled}>
          <div class="cell pair">
            <span class:mono={r.regex}>{r.from}</span>
            <Icon name="arrow" size={13} />
            <span class="t">{r.to || tr("(eliminar)", "(remove)")}</span>
            <span class="flags faint">{[r.whole_word && !r.regex ? tr("palabra", "word") : "", r.case_sensitive ? "Aa" : "", r.regex ? "regex" : ""].filter(Boolean).join(" · ")}</span>
          </div>
          <Badge>{r.list}</Badge>
          <Toggle bind:checked={r.enabled} label={tr("Activa", "Active")} />
          <Button size="sm" variant="ghost" icon="trash" title={tr("Eliminar", "Delete")} onclick={() => (app.settings.vocabulary.replacements = v.replacements.filter((x) => x.id !== r.id))} />
        </li>
      {/each}
    </ul>
  {/if}
{/if}

<div class="spacer"></div>
<Section title={tr("Corrección", "Correction")}>
  <Row title={tr("Corrección por pronunciación", "Pronunciation matching")} description={tr("Corrige palabras que suenan como un término del vocabulario («cubernetes» → «Kubernetes»). Solo actúa sobre términos de 5 letras o más.", "Fixes words that sound like a vocabulary term. Only for terms with 5+ letters.")}>
    <Toggle bind:checked={app.settings.vocabulary.phonetic_matching} label={tr("Corrección por pronunciación", "Pronunciation matching")} />
  </Row>
  <Row title={tr("Probar el vocabulario", "Test the vocabulary")} description={tr("Escribe una frase como la reconocería el motor y comprueba el resultado.", "Type a sentence as the engine would recognize it and check the result.")} stacked>
    <div class="test">
      <Input bind:value={testInput} placeholder={tr("p. ej. despliega en cubernetes y avisa a chat gpt", "e.g. deploy to cubernetes")} onkeydown={(e) => e.key === "Enter" && test()} width="100%" />
      <Button onclick={test} disabled={!testInput.trim()}>{tr("Probar", "Test")}</Button>
    </div>
    {#if testOutput}<div class="out">{testOutput}</div>{/if}
  </Row>
</Section>

<style>
  .honest {
    display: flex;
    gap: 10px;
    padding: 12px 14px;
    border-radius: var(--radius);
    background: var(--surface-2);
    border: 1px solid var(--border);
    color: var(--text-2);
    font-size: var(--text-sm);
    margin-bottom: 18px;
  }
  .honest :global(svg) {
    margin-top: 2px;
  }
  .lists {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
    margin-bottom: 16px;
  }
  .chip-wrap {
    display: inline-flex;
    align-items: center;
    border-radius: 999px;
    border: 1px solid var(--border-strong);
    padding-right: 3px;
  }
  .chip-wrap.disabled {
    opacity: 0.45;
    border-style: dashed;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 12px;
    border-radius: 999px;
    border: 1px solid var(--border-strong);
    background: transparent;
    font-size: var(--text-sm);
    cursor: pointer;
    color: var(--text-2);
  }
  .chip-wrap .chip {
    border: 0;
    padding-right: 6px;
  }
  .chip.active {
    color: var(--text);
    background: var(--active);
  }
  .n {
    font-size: var(--text-xs);
    color: var(--text-3);
  }
  .mini {
    width: 20px;
    height: 20px;
    display: grid;
    place-items: center;
    border: 0;
    border-radius: 50%;
    background: transparent;
    color: var(--text-3);
    cursor: pointer;
  }
  .mini:hover {
    color: var(--text);
    background: var(--hover);
  }
  .bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
  }
  .form {
    display: grid;
    grid-template-columns: 1fr 1.3fr auto auto;
    gap: 8px;
    margin-bottom: 12px;
  }
  .form.rep {
    grid-template-columns: 1fr auto 1fr auto auto;
    align-items: center;
    color: var(--text-3);
  }
  .opts {
    display: flex;
    gap: 18px;
    align-items: center;
    flex-wrap: wrap;
    margin: 0 0 14px 2px;
    font-size: var(--text-sm);
    color: var(--text-2);
  }
  .opts label {
    display: inline-flex;
    gap: 8px;
    align-items: center;
  }
  .err {
    font-size: var(--text-xs);
    color: var(--text);
  }
  .table {
    list-style: none;
    margin: 0;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .table li {
    display: grid;
    grid-template-columns: 1fr auto auto auto auto;
    gap: 10px;
    align-items: center;
    padding: 10px 10px 10px 16px;
  }
  .table li + li {
    border-top: 1px solid var(--border);
  }
  .table li.off .cell {
    opacity: 0.45;
  }
  .t {
    font-weight: 540;
  }
  .sub {
    font-size: var(--text-xs);
    margin-top: 1px;
  }
  .pair {
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--text-2);
    min-width: 0;
  }
  .pair .mono {
    font-family: var(--mono);
    font-size: var(--text-sm);
  }
  .flags {
    font-size: var(--text-xs);
  }
  .spacer {
    height: 28px;
  }
  .test {
    display: flex;
    gap: 8px;
    width: 100%;
  }
  .out {
    width: 100%;
    padding: 10px 12px;
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    border: 1px solid var(--border);
    font-size: var(--text-md);
    user-select: text;
  }
</style>
